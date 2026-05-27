
use async_trait::async_trait;
use parking_lot::RwLock;
use std::borrow::Cow;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Controls whether a subscription runs for every broadcast or only once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubOption {
    Always,
    Once,
}

/// Shared callback function stored by the event bus.
pub type Callback<T> =
    Arc<dyn Fn(Arc<T>) -> Pin<Box<dyn Future<Output = Result<(), PubSubError>> + Send>> + Send + Sync>;

/// Callback registry for a single target.
pub type CallbackMap<T> = HashMap<Cow<'static, str>, (SubOption, Callback<T>)>;

/// Target registry for a single event.
pub type TargetMap<T> = HashMap<Cow<'static, str>, CallbackMap<T>>;

/// Top-level event registry used internally by [`PubSub`].
pub type EventMap<T> = HashMap<Cow<'static, str>, TargetMap<T>>;

#[async_trait]
pub trait AsyncPubSub<T: Send + Sync + 'static>: Send + Sync {
    async fn subscribe<E, C, I, F, Fut>(
        &self,
        event: E,
        callback_name: C,
        target_id: I,
        option: SubOption,
        callback: F,
    ) -> Result<(), PubSubError>
    where
        E: Into<Cow<'static, str>> + Send,
        C: Into<Cow<'static, str>> + Send,
        I: Into<Cow<'static, str>> + Send,
        F: Fn(Arc<T>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), PubSubError>> + Send + 'static;

    async fn publish(&self, event: &str, data: T) -> Result<(), PubSubError>;

    async fn unsubscribe(
        &self,
        event: &str,
        callback_name: Option<&str>,
        target_id: &str,
    ) -> Result<bool, PubSubError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PubSubError {
    LockPoisoned,
    SubscriptionError(String),
    BroadcastError(String),
    UnsubscribeError(String),
}

impl std::fmt::Display for PubSubError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PubSubError::LockPoisoned => write!(f, "Failed to acquire lock: lock is poisoned"),
            PubSubError::SubscriptionError(msg) => write!(f, "Subscription operation failed: {}", msg),
            PubSubError::BroadcastError(msg) => write!(f, "Broadcast operation failed: {}", msg),
            PubSubError::UnsubscribeError(msg) => write!(f, "Unsubscribe operation failed: {}", msg),
        }
    }
}

impl std::error::Error for PubSubError {}

#[derive(Default)]
pub struct PubSub<T> {
    events: RwLock<EventMap<T>>,
}

impl<T> PubSub<T> {
    pub fn new() -> Self {
        Self {
            events: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl<T: Send + Sync + 'static> AsyncPubSub<T> for PubSub<T> {
    async fn subscribe<E, C, I, F, Fut>(
        &self,
        event: E,
        callback_name: C,
        target_id: I,
        option: SubOption,
        callback: F,
    ) -> Result<(), PubSubError>
    where
        E: Into<Cow<'static, str>> + Send,
        C: Into<Cow<'static, str>> + Send,
        I: Into<Cow<'static, str>> + Send,
        F: Fn(Arc<T>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), PubSubError>> + Send + 'static,
    {
        let mut events = self.events.write();
        let targets = events.entry(event.into()).or_default();
        let callbacks = targets.entry(target_id.into()).or_default();
        
        let callback_wrapper = Arc::new(move |message: Arc<T>| {
            let fut = callback(message);
            Box::pin(async move { fut.await }) as Pin<Box<dyn Future<Output = Result<(), PubSubError>> + Send>>
        });
        
        callbacks.insert(callback_name.into(), (option, callback_wrapper));
        Ok(())
    }

    async fn publish(&self, event: &str, data: T) -> Result<(), PubSubError> {
        let data = Arc::new(data);
        
        let callbacks_to_run = {
            let events = self.events.read();
            if let Some(targets) = events.get(event) {
                targets
                    .values()
                    .flat_map(|callbacks| callbacks.values())
                    .map(|(opt, cb)| (*opt, Arc::clone(cb)))
                    .collect::<Vec<_>>()
            } else {
                return Ok(());
            }
        };

        let mut has_once = false;
        let futures = callbacks_to_run.into_iter().map(|(opt, cb)| {
            if opt == SubOption::Once {
                has_once = true;
            }
            let data_clone = data.clone();
            async move {
                if let Err(e) = cb(data_clone).await {
                    eprintln!("Callback error: {:?}", e);
                }
            }
        });

        futures::future::join_all(futures).await;

        if has_once {
            let mut events = self.events.write();
            if let Some(targets) = events.get_mut(event) {
                for callbacks in targets.values_mut() {
                    callbacks.retain(|_, (opt, _)| *opt == SubOption::Always);
                }
                targets.retain(|_, callbacks| !callbacks.is_empty());
            }
        }

        Ok(())
    }

    async fn unsubscribe(
        &self,
        event: &str,
        callback_name: Option<&str>,
        target_id: &str,
    ) -> Result<bool, PubSubError> {
        let mut events = self.events.write();
        let mut removed = false;

        if let Some(targets) = events.get_mut(event) {
            if let Some(c_name) = callback_name {
                if let Some(callbacks) = targets.get_mut(target_id) {
                    if callbacks.remove(c_name).is_some() {
                        removed = true;
                    }
                    if callbacks.is_empty() {
                        targets.remove(target_id);
                    }
                }
            } else if targets.remove(target_id).is_some() {
                removed = true;
            }
            if targets.is_empty() {
                events.remove(event);
            }
        }
        Ok(removed)
    }
}
