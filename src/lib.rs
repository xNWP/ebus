use log::{error, warn};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{self, TrySendError},
    Arc, RwLock,
};

pub struct EventBus<T> {
    _core: Arc<EventBusCore<T>>,
}

unsafe impl<T> Send for EventBus<T> {}
unsafe impl<T> Sync for EventBus<T> {}

impl<T> EventBus<T> {
    pub fn new(debug_name: impl ToString) -> Self {
        Self {
            _core: Arc::new(EventBusCore::_new(debug_name.to_string())),
        }
    }

    pub fn join_as_subscriber(
        &self,
        debug_name: impl ToString,
        buff_size: usize,
    ) -> EventSubscriber<T> {
        let sub = EventSubscriber::_new(self._core.clone(), debug_name.to_string(), buff_size);
        let mut subscribers = self
            ._core
            ._subscribers
            .write()
            .expect("couldn't get subscribers write lock");
        subscribers.push(sub._core.clone());
        sub
    }

    pub fn join_as_publisher(&self, debug_name: impl ToString) -> EventPublisher<T> {
        EventPublisher::_new(self._core.clone(), debug_name.to_string())
    }

    pub fn garbage_collect(&self) {
        let mut subscribers = self
            ._core
            ._subscribers
            .write()
            .expect("couldn't get subscribers write lock");
        let mut valid_subs = Vec::new();

        for i in 0..subscribers.len() {
            let sub = &subscribers[i];
            if sub._is_alive() {
                valid_subs.push(sub.clone());
            }
        }
        *subscribers = valid_subs; // invalid subs will be dropped here
    }
}

impl<T> Drop for EventBus<T> {
    fn drop(&mut self) {
        self._core._is_alive.store(false, Ordering::Relaxed);
        let mut subs = self
            ._core
            ._subscribers
            .write()
            .expect("couldn't get subscribers lock");
        subs.clear();
        subs.shrink_to_fit();
    }
}

struct EventBusCore<T> {
    _subscribers: RwLock<Vec<Arc<EventSubscriberCore<T>>>>,
    _is_alive: AtomicBool,
    _debug_name: String,
}

impl<T> EventBusCore<T> {
    fn _new(_debug_name: String) -> Self {
        Self {
            _subscribers: RwLock::new(Vec::new()),
            _is_alive: AtomicBool::new(true),
            _debug_name,
        }
    }

    fn _publish(&self, event: T) {
        let event = Arc::new(event);

        let subscribers = self._subscribers.read().unwrap();
        for i in 0..subscribers.len() {
            subscribers[i]._try_send(event.clone());
        }
    }

    fn _is_alive(&self) -> bool {
        self._is_alive.load(Ordering::Relaxed)
    }
}

pub struct EventPublisher<T> {
    _bus: Arc<EventBusCore<T>>,
    _debug_name: String,
}

unsafe impl<T> Send for EventPublisher<T> {}
unsafe impl<T> Sync for EventPublisher<T> {}

impl<T> EventPublisher<T> {
    fn _new(_bus: Arc<EventBusCore<T>>, _debug_name: String) -> Self {
        Self { _bus, _debug_name }
    }

    pub fn publish(&self, event: T) {
        if self._bus._is_alive() {
            self._bus._publish(event);
        } else {
            warn!(
                "publisher '{}' tried to publish to dead bus '{}'",
                self._debug_name, self._bus._debug_name
            );
        }
    }

    pub fn is_bus_alive(&self) -> bool {
        self._bus._is_alive()
    }
}

pub struct EventSubscriber<T> {
    _core: Arc<EventSubscriberCore<T>>,
}

unsafe impl<T> Send for EventSubscriber<T> {}
unsafe impl<T> Sync for EventSubscriber<T> {}

impl<T> EventSubscriber<T> {
    fn _new(bus: Arc<EventBusCore<T>>, debug_name: String, buff_size: usize) -> Self {
        let _core = Arc::new(EventSubscriberCore::_new(bus, debug_name, buff_size));
        Self { _core }
    }

    pub fn get_next_event(&self) -> Option<Arc<T>> {
        self._core._get_next_event()
    }

    pub fn is_bus_alive(&self) -> bool {
        self._core._bus._is_alive()
    }
}

impl<T> Drop for EventSubscriber<T> {
    fn drop(&mut self) {
        self._core._is_alive.store(false, Ordering::Relaxed);
    }
}

struct EventSubscriberCore<T> {
    _bus: Arc<EventBusCore<T>>,
    _tx: mpsc::SyncSender<Arc<T>>,
    _rx: mpsc::Receiver<Arc<T>>,
    _is_alive: AtomicBool,
    _debug_name: String,
}

impl<T> EventSubscriberCore<T> {
    fn _new(_bus: Arc<EventBusCore<T>>, _debug_name: String, buff_size: usize) -> Self {
        let (_tx, _rx) = mpsc::sync_channel(buff_size);
        Self {
            _bus,
            _tx,
            _rx,
            _is_alive: AtomicBool::new(true),
            _debug_name,
        }
    }

    fn _get_next_event(&self) -> Option<Arc<T>> {
        match self._rx.try_recv() {
            Ok(v) => Some(v),
            Err(_) => {
                if !self._bus._is_alive() {
                    warn!(
                        "subscriber '{}' tried to receive event from dead bus '{}'",
                        self._debug_name, self._bus._debug_name
                    );
                }
                None
            }
        }
    }

    fn _is_alive(&self) -> bool {
        self._is_alive.load(Ordering::Relaxed)
    }

    fn _try_send(&self, event: Arc<T>) {
        match self._tx.try_send(event) {
            Err(TrySendError::Full(_)) => {
                error!(
                    "event queue full for subscriber '{}' on bus '{}'.",
                    self._debug_name, self._bus._debug_name
                );
            }
            Err(e) => panic!(
                "unexpected try_send error for subscriber '{}' on bus '{}': {e}",
                self._debug_name, self._bus._debug_name
            ),
            Ok(()) => {}
        }
    }
}
