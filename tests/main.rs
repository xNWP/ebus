use ebus::EventBus;
use rand::prelude::*;
use std::sync::Arc;
use std::{
    thread::{sleep, spawn},
    time::{Duration, Instant},
};

#[test]
fn basic_new() {
    let _ = env_logger::try_init();
    let _bus = EventBus::<u32>::new("bus");
}

#[test]
fn add_subscriber() {
    let _ = env_logger::try_init();
    let bus = EventBus::<u32>::new("bus");
    let _sub = bus.join_as_subscriber("sub", 10);
}

#[test]
fn add_publisher() {
    let _ = env_logger::try_init();
    let bus = EventBus::<u32>::new("bus");
    let _publ = bus.join_as_publisher("pub");
}

#[test]
fn basic_sync_message() {
    let _ = env_logger::try_init();
    let bus = EventBus::<u32>::new("bus");
    let publ = bus.join_as_publisher("pub");
    let sub = bus.join_as_subscriber("sub", 10);

    publ.publish(1312);

    if let Some(n) = sub.get_next_event() {
        assert_eq!(*n, 1312, "unexpected message result: {n}");
    } else {
        panic!("didn't get event");
    }
}

#[test]
fn event_bus_send_sync() {
    let _ = env_logger::try_init();
    let bus = EventBus::<u32>::new("bus");
    let jh = std::thread::spawn(move || {
        let _send = bus;
    });
    jh.join().unwrap();

    let arc_bus = Arc::new(EventBus::<u32>::new("bus"));
    let arc_bus_2 = arc_bus.clone();
    let jh = std::thread::spawn(move || {
        let _send = arc_bus_2;
    });
    jh.join().unwrap();
}

#[test]
fn subscriber_send_sync() {
    let _ = env_logger::try_init();
    let bus = EventBus::<u32>::new("bus");
    let send_sub = bus.join_as_subscriber("sub", 10);
    let jh = std::thread::spawn(move || {
        let _send = send_sub;
    });
    jh.join().unwrap();

    let send_sub_arc = Arc::new(bus.join_as_subscriber("sub_arc", 10));
    let send_sub_arc_2 = send_sub_arc.clone();
    let jh = std::thread::spawn(move || {
        let _send = send_sub_arc_2;
    });
    jh.join().unwrap();
}

#[test]
fn publisher_send_sync() {
    let _ = env_logger::try_init();
    let bus = EventBus::<u32>::new("bus");
    let send_pub = bus.join_as_publisher("pub");
    let jh = std::thread::spawn(move || {
        let _send = send_pub;
    });
    jh.join().unwrap();

    let send_pub_arc = Arc::new(bus.join_as_subscriber("pub_arc", 10));
    let send_pub_arc_2 = send_pub_arc.clone();
    let jh = std::thread::spawn(move || {
        let _send = send_pub_arc_2;
    });
    jh.join().unwrap();
}

#[test]
fn basic_async_message() {
    let _ = env_logger::try_init();
    let bus = EventBus::<u32>::new("bus");
    let publ = bus.join_as_publisher("pub");
    let sub = bus.join_as_subscriber("sub", 10);

    use std::thread::{sleep, spawn};
    use std::time::Duration;

    let jh = spawn(move || {
        publ.publish(1234);
    });
    sleep(Duration::from_secs(1));
    assert_eq!(*sub.get_next_event().unwrap(), 1234);
    jh.join().unwrap();
}

#[test]
fn many_sync_messages() {
    let _ = env_logger::try_init();
    let bus = EventBus::<u32>::new("bus");
    let publ = bus.join_as_publisher("pub");
    let sub = bus.join_as_subscriber("sub", 1200);

    for i in 0..1000 {
        publ.publish(i);
    }

    let mut it = 0;
    loop {
        if let Some(v) = sub.get_next_event() {
            assert_eq!(it, *v);
            it += 1;
        } else {
            break;
        }
    }
    assert_eq!(it, 1000);
}

#[test]
fn many_async_messages() {
    let _ = env_logger::try_init();
    let bus = EventBus::<u32>::new("bus");
    let publ = bus.join_as_publisher("pub");
    let sub = bus.join_as_subscriber("sub", 1200);

    let jh = std::thread::spawn(move || {
        let _test = bus;
        for i in 0..1000 {
            publ.publish(i);
        }
    });

    let mut it = 0;
    loop {
        if let Some(v) = sub.get_next_event() {
            assert_eq!(it, *v);
            it += 1;
            continue;
        }
        
        if !sub.is_bus_alive() {
            break;
        }
    }
    assert_eq!(it, 1000);
    jh.join().unwrap();
}

#[test]
fn is_bus_alive() {
    let _ = env_logger::try_init();
    let (sub, publ) = {
        let bus = EventBus::<u32>::new("bus");
        let sub = bus.join_as_subscriber("sub", 10);
        let publ = bus.join_as_publisher("pub");
        (sub, publ)
    };

    assert!(!sub.is_bus_alive());
    assert!(!publ.is_bus_alive());
}

fn generic_run(pub_count: u8, sub_count: u8, time: std::time::Duration) {
    let _ = env_logger::try_init();
    let bus = EventBus::<i64>::new("bus");
    let mut jhs = Vec::with_capacity((pub_count + sub_count) as usize);

    for _ in 0..sub_count {
        let sub = bus.join_as_subscriber("sub", 1024);
        jhs.push(std::thread::spawn(move || {
            let start = std::time::Instant::now();
            loop {
                if start.elapsed() > time {
                    break;
                }
                loop {
                    let ev = sub.get_next_event();
                    if ev.is_none() {
                        break;
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(random::<u64>() % 800 + 80));
            }
        }));
    }

    for _ in 0..pub_count {
        let publ = bus.join_as_publisher("pub");
        jhs.push(std::thread::spawn(move || {
            let start = std::time::Instant::now();
            loop {
                if start.elapsed() > time {
                    break;
                }
                publ.publish(random());
                std::thread::sleep(std::time::Duration::from_millis(random::<u64>() % 400 + 10));
            }
        }));
    }

    for jh in jhs {
        jh.join().unwrap();
    }
}

#[test]
fn few_pub_one_sub() {
    generic_run(4, 1, Duration::from_secs(30));
}

#[test]
fn many_pub_one_sub() {
    generic_run(12, 1, Duration::from_secs(30));
}

#[test]
fn huge_pub_one_sub() {
    generic_run(100, 1, Duration::from_secs(30));
}

#[test]
fn few_pub_few_sub() {
    generic_run(4, 4, Duration::from_secs(30));
}

#[test]
fn few_pub_many_sub() {
    generic_run(4, 12, Duration::from_secs(30));
}

#[test]
fn few_pub_huge_sub() {
    generic_run(4, 100, Duration::from_secs(60));
}

#[test]
fn huge_pub_huge_sub() {
    generic_run(100, 100, Duration::from_secs(60));
}

#[test]
fn sub_drop() {
    let _ = env_logger::try_init();
    let bus = EventBus::<String>::new("bus");
    {
        let _sub = bus.join_as_subscriber("sub", 10);
    }
    let publ = bus.join_as_publisher("pub");
    publ.publish(format!("hello! {}", random::<f32>()));
}

#[test]
fn long_run() {
    generic_run(32, 12, Duration::from_secs(120));
}

#[test]
fn open_door_pub_sub() {
    let _ = env_logger::try_init();
    let bus = EventBus::<String>::new("bus");
    let bus = Arc::new(bus);

    let create_publisher = |bus: Arc<EventBus<String>>, i: usize| {
        let start_delay = random::<u64>() % 10 + 5;
        let run_time = random::<u64>() % 15 + 5;
        spawn(move || {
            sleep(Duration::from_secs(start_delay));
            let publ = bus.join_as_publisher(format!("pub_{i}"));
            let start = Instant::now();
            loop {
                if start.elapsed() > Duration::from_secs(run_time) {
                    break;
                }
                publ.publish(format!("Hello! {}", random::<f32>()));
                sleep(Duration::from_millis(random::<u64>() % 2000 + 50));
            }
        })
    };

    let create_subscriber = |bus: Arc<EventBus<String>>, i: usize| {
        let start_delay = random::<u64>() % 10 + 5;
        let run_time = random::<u64>() % 15 + 5;
        spawn(move || {
            sleep(Duration::from_secs(start_delay));
            let sub = bus.join_as_subscriber(format!("sub_{i}"), 1024 * 1024);
            let start = Instant::now();
            loop {
                if start.elapsed() > Duration::from_secs(run_time) {
                    break;
                }

                loop {
                    let ev = sub.get_next_event();
                    if ev.is_none() {
                        break;
                    }
                    let _ev = ev.unwrap();
                }

                sleep(Duration::from_millis(random::<u64>() % 1000 + 50));
            }
        })
    };

    let mut jhs = Vec::new();
    for i in 0..100 {
        jhs.push(create_publisher(bus.clone(), i));
        jhs.push(create_subscriber(bus.clone(), i));
    }
    for jh in jhs {
        jh.join().unwrap();
    }
}

#[test]
fn receive_all_sync() {
    let bus = EventBus::<u32>::new("bus");
    let publ = bus.join_as_publisher("pub");
    let sub = bus.join_as_subscriber("sub", 12000);

    for i in 0..10000 {
        publ.publish(i);
    }
    let mut vals = Vec::new();
    loop {
        let val = sub.get_next_event();
        if val.is_none() {
            break;
        }
        vals.push(val.unwrap());
    }
    vals.sort();
    for i in 0..10000 {
        assert_eq!(i as u32, *vals[i]);
    }
}

#[test]
fn receive_all_sync_ordered() {
    let bus = EventBus::<u32>::new("bus");
    let publ = bus.join_as_publisher("pub");
    let sub = bus.join_as_subscriber("sub", 12000);

    for i in 0..10000 {
        publ.publish(i);
    }
    let mut vals = Vec::new();
    loop {
        let val = sub.get_next_event();
        if val.is_none() {
            break;
        }
        vals.push(val.unwrap());
    }
    for i in 0..10000 {
        assert_eq!(i as u32, *vals[i]);
    }
}

#[test]
fn receive_all_async() {
    let bus = EventBus::<u32>::new("bus");
    let publ = bus.join_as_publisher("pub");
    let sub = bus.join_as_subscriber("sub", 12000);

    let jh1 = spawn(move || {
        for i in 0..10000 {
            publ.publish(i);
        }
    });

    let mut vals = Vec::new();
    let mut done = false;
    loop {
        let val = sub.get_next_event();
        if val.is_none() {
            if done {
                break;
            }
            done = true;
            sleep(Duration::from_millis(100));
            continue;
        }
        vals.push(val.unwrap());
        done = false;
    }
    vals.sort();
    for i in 0..10000 {
        assert_eq!(i as u32, *vals[i]);
    }

    jh1.join().unwrap();
}

#[test]
fn receive_all_async_ordered() {
    let bus = EventBus::<u32>::new("bus");
    let publ = bus.join_as_publisher("pub");
    let sub = bus.join_as_subscriber("sub", 12000);

    let jh1 = spawn(move || {
        for i in 0..10000 {
            publ.publish(i);
        }
    });

    let mut vals = Vec::new();
    let mut done = false;
    loop {
        let val = sub.get_next_event();
        if val.is_none() {
            if done {
                break;
            }
            done = true;
            sleep(Duration::from_millis(100));
            continue;
        }
        vals.push(val.unwrap());
        done = false;
    }
    for i in 0..10000 {
        assert_eq!(i as u32, *vals[i]);
    }

    jh1.join().unwrap();
}
