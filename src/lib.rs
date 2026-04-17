use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubOption {
    Always,
    Once,
}

pub struct PubSub<T> {
    #[allow(clippy::type_complexity)]
    events: RwLock<
        HashMap<
            String,
            HashMap<
                String,
                HashMap<String, (SubOption, Box<dyn Fn(&T) + Send + Sync>)>,
            >,
        >,
    >,
}

impl<T> PubSub<T> {
    pub fn new() -> Self {
        Self {
            events: RwLock::new(HashMap::new()),
        }
    }

    pub fn subscribe<F>(
        &self,
        event: &str,
        callback_name: &str,
        target_id: &str,
        option: SubOption,
        callback: F,
    ) where
        F: Fn(&T) + Send + Sync + 'static,
    {
        let mut events = self.events.write().unwrap();
        
        let targets = events.entry(event.to_string()).or_default();
        let callbacks = targets.entry(target_id.to_string()).or_default();
        
        callbacks.insert(callback_name.to_string(), (option, Box::new(callback)));
    }

    pub fn broadcast(&self, event: &str, data: &T) {
        let mut events = self.events.write().unwrap();
        
        if let Some(targets) = events.get_mut(event) {
            for (_, callbacks) in targets.iter_mut() {
                callbacks.retain(|_, v| {
                    (v.1)(data);
                    v.0 == SubOption::Always
                });
            }
            targets.retain(|_, callbacks| !callbacks.is_empty());
        }
    }

    pub fn unsubscribe(&self, event: &str, callback_name: Option<&str>, target_id: &str) {
        let mut events = self.events.write().unwrap();
        
        if let Some(targets) = events.get_mut(event) {
            if let Some(c_name) = callback_name {
                if let Some(callbacks) = targets.get_mut(target_id) {
                    callbacks.remove(c_name);
                }
            } else {
                targets.remove(target_id);
            }
            targets.retain(|_, callbacks| !callbacks.is_empty());
        }
    }

    pub fn remove_all_subscriptions(&self) {
        let mut events = self.events.write().unwrap();
        events.clear();
    }
}

impl<T> Default for PubSub<T> {
    fn default() -> Self {
        Self::new()
    }
}

pub static GLOBAL_BUS: OnceLock<PubSub<String>> = OnceLock::new();

pub fn global_bus() -> &'static PubSub<String> {
    GLOBAL_BUS.get_or_init(|| PubSub::new())
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

        bus.unsubscribe("evt", Some("cb1"), "tgt");
        
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

        bus.unsubscribe("evt", None, "tgt");
        
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
}
