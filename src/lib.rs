use async_trait::async_trait;
use std::sync::Arc;
use std::borrow::Cow;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubOption {
    Always,
    Once,
}

pub type Callback<T> = Arc<dyn Fn(&T) + Send + Sync>;
pub type CallbackMap<T> = HashMap<Cow<'static, str>, (SubOption, Callback<T>)>;
pub type TargetMap<T> = HashMap<Cow<'static, str>, CallbackMap<T>>;
pub type EventMap<T> = HashMap<Cow<'static, str>, TargetMap<T>>;

#[async_trait]
pub trait AsyncPubSub<T: Send + Sync + 'static>: Send + Sync {
    async fn subscribe<E, C, I, F>(&self, event: E, callback_name: C, target_id: I, option: SubOption, callback: F) -> Result<(), PubSubError>
    where
        E: Into<Cow<'static, str>> + Send,
        C: Into<Cow<'static, str>> + Send,
        I: Into<Cow<'static, str>> + Send,
        F: Fn(&T) + Send + Sync + 'static;

    async fn broadcast(&self, event: &str, data: &T) -> Result<(), PubSubError>;

    async fn unsubscribe(&self, event: &str, callback_name: Option<&str>, target_id: &str) -> Result<bool, PubSubError>;
}

#[derive(Debug)]
pub enum PubSubError {
    LockPoisoned,
    SubscriptionError(String),
    BroadcastError(String),
    UnsubscribeError(String),
}

pub struct PubSub<T> {
    events: RwLock<EventMap<T>>,
}

impl<T> PubSub<T> {
    pub fn new() -> Self {
        Self {
            events: RwLock::new(HashMap::new()),
        }
    }

    pub fn subscribe<E, C, I, F>(
        &self,
        event: E,
        callback_name: C,
        target_id: I,
        option: SubOption,
        callback: F,
    ) where
        E: Into<Cow<'static, str>>,
        C: Into<Cow<'static, str>>,
        I: Into<Cow<'static, str>>,
        F: Fn(&T) + Send + Sync + 'static,
    {
        let mut events = self.events.write();
        
        let targets = events.entry(event.into()).or_default();
        let callbacks = targets.entry(target_id.into()).or_default();
        
        callbacks.insert(callback_name.into(), (option, Arc::new(callback)));
    }

    pub fn broadcast(&self, event: &str, data: &T) {
        let mut callbacks_to_run = Vec::new();
        {
            let events = self.events.read();
            if let Some(targets) = events.get(event) {
                for (_, callbacks) in targets.iter() {
                    for (_, (opt, cb)) in callbacks.iter() {
                        callbacks_to_run.push((*opt, Arc::clone(cb)));
                    }
                }
            } else {
                return;
            }
        }

        let mut has_once_callbacks = false;
        for (opt, cb) in callbacks_to_run {
            cb(data);
            if opt == SubOption::Once {
                has_once_callbacks = true;
            }
        }

        if has_once_callbacks {
            let mut events = self.events.write();
            if let Some(targets) = events.get_mut(event) {
                for (_, callbacks) in targets.iter_mut() {
                    callbacks.retain(|_, (opt, _)| *opt == SubOption::Always);
                }
                targets.retain(|_, callbacks| !callbacks.is_empty());
            }
        }
    }

    pub fn unsubscribe(&self, event: &str, callback_name: Option<&str>, target_id: &str) -> bool {
        let mut events = self.events.write();
        let mut removed = false;
        
        if let Some(targets) = events.get_mut(event) {
            if let Some(c_name) = callback_name {
                if let Some(callbacks) = targets.get_mut(target_id) {
                    removed = callbacks.remove(c_name).is_some();
                }
            } else {
                removed = targets.remove(target_id).is_some();
            }
            targets.retain(|_, callbacks| !callbacks.is_empty());
        }
        removed
    }

    pub fn remove_all_subscriptions(&self) {
        let mut events = self.events.write();
        events.clear();
    }
}

#[async_trait]
impl<T: Send + Sync + 'static> AsyncPubSub<T> for PubSub<T> {
    async fn subscribe<E, C, I, F>(&self, event: E, callback_name: C, target_id: I, option: SubOption, callback: F) -> Result<(), PubSubError>
    where
        E: Into<Cow<'static, str>> + Send,
        C: Into<Cow<'static, str>> + Send,
        I: Into<Cow<'static, str>> + Send,
        F: Fn(&T) + Send + Sync + 'static,
    {
        Ok(self.subscribe(event, callback_name, target_id, option, callback))
    }

    async fn broadcast(&self, event: &str, data: &T) -> Result<(), PubSubError> {
        self.broadcast(event, data);
        Ok(())
    }

    async fn unsubscribe(&self, event: &str, callback_name: Option<&str>, target_id: &str) -> Result<bool, PubSubError> {
        Ok(self.unsubscribe(event, callback_name, target_id))
    }
}

pub static GLOBAL_BUS: OnceLock<PubSub<String>> = OnceLock::new();

pub fn global_bus() -> &'static PubSub<String> {
    GLOBAL_BUS.get_or_init(PubSub::new)
}

#[macro_export]
macro_rules! broadcast {
    ($event:expr, $data:expr) => {
        $crate::global_bus().broadcast($event, &$data.to_string());
    };
}

#[macro_export]
macro_rules! subscribe {
    ($event:expr, $callback_name:expr, $target_id:expr, $option:expr, $closure:expr) => {
        $crate::global_bus().subscribe($event, $callback_name, $target_id, $option, $closure);
    };
    ($event:expr, $callback_name:expr, $target_id:expr, $closure:expr) => {
        $crate::global_bus().subscribe($event, $callback_name, $target_id, $crate::SubOption::Always, $closure);
    };
    ($event:expr, $closure:expr) => {
        $crate::global_bus().subscribe(
            $event,
            concat!("cb_", line!()),
            file!(),
            $crate::SubOption::Always,
            $closure
        );
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_always_and_once_subscriptions() {
        let bus = PubSub::<String>::new();
        let counter = Arc::new(Mutex::new(0));

        let c1 = Arc::clone(&counter);
        bus.subscribe("test_event", "cb1", "target1", SubOption::Always, move |_| {
            *c1.lock().unwrap() += 1;
        });

        let c2 = Arc::clone(&counter);
        bus.subscribe("test_event", "cb2", "target1", SubOption::Once, move |_| {
            *c2.lock().unwrap() += 1;
        });

        bus.broadcast("test_event", &"hello".to_string());
        assert_eq!(*counter.lock().unwrap(), 2);

        bus.broadcast("test_event", &"hello".to_string());
        assert_eq!(*counter.lock().unwrap(), 3);
    }

    #[test]
    fn test_unsubscribe_specific_callback() {
        let bus = PubSub::<String>::new();
        let counter = Arc::new(Mutex::new(0));

        let c1 = Arc::clone(&counter);
        bus.subscribe("evt", "cb1", "tgt", SubOption::Always, move |_| {
            *c1.lock().unwrap() += 1;
        });

        let c2 = Arc::clone(&counter);
        bus.subscribe("evt", "cb2", "tgt", SubOption::Always, move |_| {
            *c2.lock().unwrap() += 1;
        });

        bus.broadcast("evt", &"".to_string());
        assert_eq!(*counter.lock().unwrap(), 2);

        assert!(bus.unsubscribe("evt", Some("cb1"), "tgt"));

        bus.broadcast("evt", &"".to_string());
        assert_eq!(*counter.lock().unwrap(), 3);
    }

    #[test]
    fn test_unsubscribe_entire_target() {
        let bus = PubSub::<String>::new();
        let counter = Arc::new(Mutex::new(0));

        let c1 = Arc::clone(&counter);
        bus.subscribe("evt", "cb1", "tgt", SubOption::Always, move |_| {
            *c1.lock().unwrap() += 1;
        });

        let c2 = Arc::clone(&counter);
        bus.subscribe("evt", "cb2", "tgt", SubOption::Always, move |_| {
            *c2.lock().unwrap() += 1;
        });

        assert!(bus.unsubscribe("evt", None, "tgt"));

        bus.broadcast("evt", &"".to_string());
        assert_eq!(*counter.lock().unwrap(), 0);
    }

    #[test]
    fn test_global_macros() {
        let counter = Arc::new(Mutex::new(0));
        let c1 = Arc::clone(&counter);

        subscribe!("macro_event", move |data: &String| {
            assert_eq!(data, "payload");
            *c1.lock().unwrap() += 1;
        });

        broadcast!("macro_event", "payload");
        assert_eq!(*counter.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn test_async_subscribe_broadcast_unsubscribe() {
        let bus = PubSub::<String>::new();
        let counter = Arc::new(Mutex::new(0));

        let c1 = Arc::clone(&counter);
        bus.subscribe("async_event", "cb1", "target1", SubOption::Always, move |_| {
            *c1.lock().unwrap() += 1;
        });

        bus.broadcast("async_event", &"hello".to_string());
        assert_eq!(*counter.lock().unwrap(), 1);

        assert!(bus.unsubscribe("async_event", Some("cb1"), "target1"));
    }
}
