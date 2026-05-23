//! A lightweight, thread-safe publish-subscribe event bus.
//!
//! `ez_pubsub` lets you register callbacks for named events and broadcast typed
//! payloads to those callbacks. Each [`PubSub<T>`] instance handles one payload
//! type, while the crate-level macros use a global [`PubSub<String>`].
//!
//! # Example
//!
//! ```
//! use ez_pubsub::{PubSub, SubOption};
//! use std::sync::{Arc, Mutex};
//!
//! let bus = PubSub::<String>::new();
//! let messages = Arc::new(Mutex::new(Vec::new()));
//!
//! let received = Arc::clone(&messages);
//! bus.subscribe(
//!     "user.created",
//!     "audit-log",
//!     "audit-service",
//!     SubOption::Always,
//!     move |payload| received.lock().unwrap().push(payload.clone()),
//! );
//!
//! bus.broadcast("user.created", &"alice".to_string());
//!
//! assert_eq!(messages.lock().unwrap().as_slice(), &["alice"]);
//! ```

use async_trait::async_trait;
use parking_lot::RwLock;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::OnceLock;

/// Controls whether a subscription runs for every broadcast or only once.
///
/// # Example
///
/// ```
/// use ez_pubsub::SubOption;
///
/// let always = SubOption::Always;
/// let once = SubOption::Once;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubOption {
    /// Keep the callback subscribed after each matching broadcast.
    Always,
    /// Remove the callback after it handles the next matching broadcast.
    Once,
}

/// Shared callback function stored by the event bus.
pub type Callback<T> = Arc<dyn Fn(&T) + Send + Sync>;

/// Callback registry for a single target.
pub type CallbackMap<T> = HashMap<Cow<'static, str>, (SubOption, Callback<T>)>;

/// Target registry for a single event.
pub type TargetMap<T> = HashMap<Cow<'static, str>, CallbackMap<T>>;

/// Top-level event registry used internally by [`PubSub`].
pub type EventMap<T> = HashMap<Cow<'static, str>, TargetMap<T>>;

/// Async-friendly interface for publish-subscribe implementations.
///
/// The default [`PubSub`] implementation performs the same in-memory operations
/// as the synchronous API, but exposes them through async method signatures for
/// easier integration into async application code.
///
/// # Example
///
/// ```
/// use ez_pubsub::{PubSub, AsyncPubSub, SubOption};
///
/// #[tokio::main]
/// async fn main() {
///     let bus = PubSub::<String>::new();
///     
///     AsyncPubSub::subscribe(&bus, "event", "callback", "target", SubOption::Always, |msg: &String| {
///         assert_eq!(msg, "hello");
///     }).await.unwrap();
///     
///     AsyncPubSub::broadcast(&bus, "event", &"hello".to_string()).await.unwrap();
/// }
/// ```
#[async_trait]
pub trait AsyncPubSub<T: Send + Sync + 'static>: Send + Sync {
    /// Subscribe a callback to an event.
    ///
    /// Reusing the same `event`, `target_id`, and `callback_name` replaces the
    /// previous callback.
    async fn subscribe<E, C, I, F>(
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
        F: Fn(&T) + Send + Sync + 'static;

    /// Broadcast `data` to every callback subscribed to `event`.
    async fn broadcast(&self, event: &str, data: &T) -> Result<(), PubSubError>;

    /// Remove one callback for a target, or all callbacks for the target.
    ///
    /// Returns `true` when at least one subscription was removed.
    async fn unsubscribe(
        &self,
        event: &str,
        callback_name: Option<&str>,
        target_id: &str,
    ) -> Result<bool, PubSubError>;
}

/// Error type used by the async publish-subscribe trait.
///
/// # Example
///
/// ```
/// use ez_pubsub::PubSubError;
///
/// let err = PubSubError::BroadcastError("failed".to_string());
/// println!("{}", err);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PubSubError {
    /// A lock could not be acquired because it was poisoned.
    ///
    /// The current implementation uses `parking_lot`, which does not poison
    /// locks, so this variant is reserved for compatibility with other
    /// implementations of [`AsyncPubSub`].
    LockPoisoned,
    /// A subscription operation failed.
    SubscriptionError(String),
    /// A broadcast operation failed.
    BroadcastError(String),
    /// An unsubscribe operation failed.
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

/// Thread-safe in-memory publish-subscribe event bus.
///
/// A `PubSub<T>` stores callbacks for payloads of type `T`. Callback lookup is
/// organized by event name, target id, and callback name:
///
/// ```text
/// event -> target -> callback
/// ```
///
/// # Example
///
/// ```
/// use ez_pubsub::{PubSub, SubOption};
/// use std::sync::{Arc, Mutex};
///
/// let bus = PubSub::<u32>::new();
/// let total = Arc::new(Mutex::new(0));
///
/// let total_for_callback = Arc::clone(&total);
/// bus.subscribe("count", "add", "counter", SubOption::Always, move |value| {
///     *total_for_callback.lock().unwrap() += value;
/// });
///
/// bus.broadcast("count", &2);
/// bus.broadcast("count", &3);
///
/// assert_eq!(*total.lock().unwrap(), 5);
/// ```
pub struct PubSub<T> {
    events: RwLock<EventMap<T>>,
}

impl<T> PubSub<T> {
    /// Create an empty event bus.
    ///
    /// # Example
    ///
    /// ```
    /// use ez_pubsub::PubSub;
    ///
    /// let bus = PubSub::<String>::new();
    /// ```
    pub fn new() -> Self {
        Self {
            events: RwLock::new(HashMap::new()),
        }
    }

    /// Subscribe a callback to an event.
    ///
    /// `event` identifies the event to listen for. `target_id` groups callbacks
    /// by owner or component. `callback_name` identifies the callback within
    /// that target. If the same event, target id, and callback name are used
    /// again, the previous callback is replaced.
    ///
    /// # Example
    ///
    /// ```
    /// use ez_pubsub::{PubSub, SubOption};
    ///
    /// let bus = PubSub::<String>::new();
    ///
    /// bus.subscribe(
    ///     "message.received",
    ///     "print-message",
    ///     "logger",
    ///     SubOption::Always,
    ///     |message| println!("{message}"),
    /// );
    /// ```
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

    /// Broadcast data to all callbacks subscribed to `event`.
    ///
    /// `SubOption::Once` callbacks are removed after they run. The internal read
    /// lock is released before callbacks are invoked, so callbacks can safely
    /// subscribe, unsubscribe, or broadcast through the same bus.
    ///
    /// # Example
    ///
    /// ```
    /// use ez_pubsub::{PubSub, SubOption};
    /// use std::sync::{Arc, Mutex};
    ///
    /// let bus = PubSub::<String>::new();
    /// let last = Arc::new(Mutex::new(String::new()));
    ///
    /// let last_message = Arc::clone(&last);
    /// bus.subscribe("message", "remember", "state", SubOption::Always, move |message| {
    ///     *last_message.lock().unwrap() = message.clone();
    /// });
    ///
    /// bus.broadcast("message", &"hello".to_string());
    ///
    /// assert_eq!(last.lock().unwrap().as_str(), "hello");
    /// ```
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

    /// Remove subscriptions for `target_id` from `event`.
    ///
    /// When `callback_name` is `Some`, only that callback is removed from the
    /// target. When `callback_name` is `None`, all callbacks for the target are
    /// removed.
    ///
    /// Returns `true` if a callback or target existed and was removed.
    ///
    /// # Example
    ///
    /// ```
    /// use ez_pubsub::{PubSub, SubOption};
    ///
    /// let bus = PubSub::<String>::new();
    /// bus.subscribe("event", "callback", "target", SubOption::Always, |_| {});
    ///
    /// assert!(bus.unsubscribe("event", Some("callback"), "target"));
    /// assert!(!bus.unsubscribe("event", Some("callback"), "target"));
    /// ```
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

    /// Remove every subscription from the bus.
    ///
    /// # Example
    ///
    /// ```
    /// use ez_pubsub::{PubSub, SubOption};
    ///
    /// let bus = PubSub::<String>::new();
    /// bus.subscribe("event", "callback", "target", SubOption::Always, |_| {});
    ///
    /// bus.remove_all_subscriptions();
    ///
    /// assert!(!bus.unsubscribe("event", Some("callback"), "target"));
    /// ```
    pub fn remove_all_subscriptions(&self) {
        let mut events = self.events.write();
        events.clear();
    }
}

impl<T> Default for PubSub<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<T: Send + Sync + 'static> AsyncPubSub<T> for PubSub<T> {
    async fn subscribe<E, C, I, F>(
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
        F: Fn(&T) + Send + Sync + 'static,
    {
        self.subscribe(event, callback_name, target_id, option, callback);
        Ok(())
    }

    /// Broadcast `data` to every callback subscribed to `event`.
    ///
    /// # Example
    ///
    /// ```
    /// use ez_pubsub::{PubSub, AsyncPubSub, SubOption};
    /// use std::sync::{Arc, Mutex};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let bus = PubSub::<String>::new();
    ///     let received_message = Arc::new(Mutex::new(None::<String>));
    ///
    ///     let received_message_clone = Arc::clone(&received_message);
    ///     bus.subscribe(
    ///         "user.joined",
    ///         "listener1",
    ///         "group1",
    ///         SubOption::Always,
    ///         move |msg: &String| {
    ///             let mut lock = received_message_clone.lock().unwrap();
    ///             *lock = Some(msg.clone());
    ///         },
    ///     );
    ///
    ///     bus.broadcast("user.joined", &"Alice".to_string());
    ///
    ///     assert_eq!(received_message.lock().unwrap().as_ref(), Some(&"Alice".to_string()));
    /// }
    /// ```
    async fn broadcast(&self, event: &str, data: &T) -> Result<(), PubSubError> {
        self.broadcast(event, data);
        Ok(())
    }

    /// Remove one callback for a target, or all callbacks for the target.
    ///
    /// Returns `true` when at least one subscription was removed.
    ///
    /// # Example
    ///
    /// ```
    /// use ez_pubsub::{PubSub, AsyncPubSub, SubOption};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let bus = PubSub::<String>::new();
    ///     bus.subscribe("event", "callback", "target", SubOption::Always, |_| {});
    ///
    ///     assert!(bus.unsubscribe("event", Some("callback"), "target"));
    ///     assert!(!bus.unsubscribe("event", Some("callback"), "target"));
    /// }
    /// ```
    async fn unsubscribe(
        &self,
        event: &str,
        callback_name: Option<&str>,
        target_id: &str,
    ) -> Result<bool, PubSubError> {
        Ok(self.unsubscribe(event, callback_name, target_id))
    }
}
