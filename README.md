# ez_pubsub

`ez_pubsub` is a lightweight, thread-safe publish-subscribe event bus for Rust.
It is useful when different parts of an application need to react to named events without being tightly coupled to each other.

## Highlights

- **Small API surface**: create a bus, subscribe callbacks, broadcast events, unsubscribe when needed.
- **Thread-safe internals**: subscriptions are stored behind `parking_lot::RwLock`.
- **Typed payloads**: each `PubSub<T>` instance broadcasts values of one payload type.
- **One-shot subscriptions**: use `SubOption::Once` for callbacks that should run only on the next broadcast.
- **Async-friendly trait**: `AsyncPubSub` provides async method signatures for integration with async code.
- **Global string bus macros**: optional convenience macros for quick app-wide string events.

## Installation

Add the crate to your `Cargo.toml`:

```toml
[dependencies]
ez_pubsub = "0.1"
```

Or, while developing against the repository directly:

```toml
[dependencies]
ez_pubsub = { git = "https://github.com/engr-uba/ez_pubsub" }
```

## Basic usage

```rust
use ez_pubsub::{PubSub, SubOption};
use std::sync::{Arc, Mutex};

let bus = PubSub::<String>::new();
let messages = Arc::new(Mutex::new(Vec::new()));

let received = Arc::clone(&messages);
bus.subscribe(
    "user.created",
    "audit-log",
    "audit-service",
    SubOption::Always,
    move |payload| {
        received.lock().unwrap().push(payload.clone());
    },
);

bus.broadcast("user.created", &"alice".to_string());

assert_eq!(messages.lock().unwrap().as_slice(), &["alice"]);
```

## One-shot subscriptions

Use `SubOption::Once` when a callback should be removed automatically after it receives one event.

```rust
use ez_pubsub::{PubSub, SubOption};
use std::sync::{Arc, Mutex};

let bus = PubSub::<u32>::new();
let count = Arc::new(Mutex::new(0));

let count_once = Arc::clone(&count);
bus.subscribe("ready", "only-once", "worker-1", SubOption::Once, move |_| {
    *count_once.lock().unwrap() += 1;
});

bus.broadcast("ready", &1);
bus.broadcast("ready", &2);

assert_eq!(*count.lock().unwrap(), 1);
```

## Unsubscribing

Pass a callback name to remove one callback for a target, or `None` to remove all callbacks for that target.

```rust
use ez_pubsub::{PubSub, SubOption};

let bus = PubSub::<String>::new();

bus.subscribe("event", "callback-a", "target-1", SubOption::Always, |_| {});
bus.subscribe("event", "callback-b", "target-1", SubOption::Always, |_| {});

assert!(bus.unsubscribe("event", Some("callback-a"), "target-1"));
assert!(bus.unsubscribe("event", None, "target-1"));
```

## Global string bus macros

For quick application-level string events, the crate exposes a global `PubSub<String>` and two macros.

```rust
use ez_pubsub::{broadcast, subscribe};

subscribe!("app.started", |message: &String| {
    println!("received: {message}");
});

broadcast!("app.started", "boot complete");
```

For libraries or testable application components, prefer creating and passing an explicit `PubSub<T>` instance instead of relying on the global bus.

## API notes

### Event, target, and callback names

Subscriptions are stored as:

```text
event name -> target id -> callback name -> callback
```

This lets a single target own multiple named callbacks for the same event. Reusing the same event, target id, and callback name replaces the previous callback.

### Callback execution

Callbacks receive `&T` and are invoked synchronously when `broadcast` is called. The internal read lock is released before callbacks run, so callbacks may safely interact with the same bus without holding the subscription map lock.

### Async integration

`AsyncPubSub` wraps the same operations in async method signatures. The callbacks themselves are currently synchronous closures. If you need truly async callback bodies, spawn work from inside the callback or wrap this crate behind your own async task/channel boundary.

## Architecture & Mechanics

### 1. Subscription & Callback Hierarchy
Subscribers register to events with a hierarchical namespace structure: `Event` ──► `Target` ──► `Callback Name`. This three-tiered organization allows complex components to easily register multiple distinct callbacks for the same event and teardown subscriptions granularly:

```text
  PubSub Event Bus
         │
         └── Events Map (RwLock<HashMap<Cow, TargetMap>>)
               │
               ├── "state_update" ──► TargetMap (HashMap<Cow, CallbackMap>)
               │                        │
               │                        └── "ui" ──► CallbackMap (HashMap<Cow, (SubOption, Callback)>)
               │                                       │
               │                                       ├── "ui_state_cb" ────► (Always, CallbackFn)
               │                                       └── "cleanup_alert" ──► (Once, CallbackFn)
               │
               └── "ui_input" ─────► ...
```

---

### 2. Publication & Message Dispatch Flow
`ez_pubsub` is designed for high concurrency and avoids deadlocks by releasing locks *before* executing async callbacks. Crucially, it manages one-shot (`SubOption::Once`) callbacks atomically in a race-free manner:

```text
                   [publish(event, data)]
                              │
                              ▼
                  Acquire RWLock Read Lock
                 Are there any "Once" subs?
                   /                    \
                (Yes)                   (No)
                 /                        \
         Drop Read Lock              Keep Read Lock
        Acquire Write Lock          Clone "Always" Callbacks
      Atomically Extract "Once"         (cheap Arc copies)
        and Keep "Always"                 │
        Release Write Lock          Release Read Lock
                │                         │
                ▼                         ▼
            ┌─────────────────────────────────┐
            │ Combine Callback Futures into a │
            │   Vec<Future> and await them    │
            │   concurrently via join_all()   │
            └─────────────────────────────────┘
```

## Development

Run the test suite:

```bash
cargo test
```

Run lint checks:

```bash
cargo clippy --all-targets --all-features
```

Build documentation locally:

```bash
cargo doc --open
```

## When to use this crate

Use `ez_pubsub` when you want a small in-process event bus for application modules, examples, tools, or tests.

For cross-process messaging, durable queues, backpressure, or distributed event streaming, use a dedicated messaging system such as NATS, Kafka, Redis Streams, or Tokio channels depending on your needs.
