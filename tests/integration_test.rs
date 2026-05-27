
use ez_pubsub::{AsyncPubSub, PubSub, SubOption};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_single_publisher_single_subscriber() {
    let pubsub = PubSub::<String>::new();
    let received_messages = Arc::new(Mutex::new(vec![]));

    let messages_clone = received_messages.clone();
    pubsub
        .subscribe("test_topic", "cb1", "target1", SubOption::Always, move |message: Arc<String>| {
            let messages = messages_clone.clone();
            Box::pin(async move {
                messages.lock().unwrap().push((*message).clone());
                Ok(())
            })
        })
        .await
        .unwrap();

    pubsub
        .publish("test_topic", "hello".to_string())
        .await
        .unwrap();
    sleep(Duration::from_millis(10)).await;

    let messages = received_messages.lock().unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0], "hello");
}

#[tokio::test]
async fn test_single_publisher_multiple_subscribers() {
    let pubsub = PubSub::<String>::new();
    let received_messages1 = Arc::new(Mutex::new(vec![]));
    let received_messages2 = Arc::new(Mutex::new(vec![]));

    let messages_clone1 = received_messages1.clone();
    pubsub
        .subscribe("test_topic", "cb1", "target1", SubOption::Always, move |message: Arc<String>| {
            let messages = messages_clone1.clone();
            Box::pin(async move {
                messages.lock().unwrap().push((*message).clone());
                Ok(())
            })
        })
        .await
        .unwrap();

    let messages_clone2 = received_messages2.clone();
    pubsub
        .subscribe("test_topic", "cb2", "target2", SubOption::Always, move |message: Arc<String>| {
            let messages = messages_clone2.clone();
            Box::pin(async move {
                messages.lock().unwrap().push((*message).clone());
                Ok(())
            })
        })
        .await
        .unwrap();

    pubsub
        .publish("test_topic", "hello".to_string())
        .await
        .unwrap();
    sleep(Duration::from_millis(10)).await;

    let messages1 = received_messages1.lock().unwrap();
    assert_eq!(messages1.len(), 1);
    assert_eq!(messages1[0], "hello");

    let messages2 = received_messages2.lock().unwrap();
    assert_eq!(messages2.len(), 1);
    assert_eq!(messages2[0], "hello");
}

#[tokio::test]
async fn test_multiple_publishers_single_subscriber() {
    let pubsub = Arc::new(PubSub::<String>::new());
    let received_messages = Arc::new(Mutex::new(vec![]));

    let messages_clone = received_messages.clone();
    pubsub
        .subscribe("test_topic", "cb1", "target1", SubOption::Always, move |message: Arc<String>| {
            let messages = messages_clone.clone();
            Box::pin(async move {
                messages.lock().unwrap().push((*message).clone());
                Ok(())
            })
        })
        .await
        .unwrap();

    let pubsub1 = pubsub.clone();
    let handle1 = tokio::spawn(async move {
        pubsub1
            .publish("test_topic", "hello from publisher 1".to_string())
            .await
            .unwrap();
    });

    let pubsub2 = pubsub.clone();
    let handle2 = tokio::spawn(async move {
        pubsub2
            .publish("test_topic", "hello from publisher 2".to_string())
            .await
            .unwrap();
    });

    handle1.await.unwrap();
    handle2.await.unwrap();
    sleep(Duration::from_millis(10)).await;

    let messages = received_messages.lock().unwrap();
    assert_eq!(messages.len(), 2);
    assert!(messages.contains(&"hello from publisher 1".to_string()));
    assert!(messages.contains(&"hello from publisher 2".to_string()));
}

#[tokio::test]
async fn test_concurrent_callbacks_do_not_block() {
    let pubsub = Arc::new(PubSub::<String>::new());
    let received_messages = Arc::new(Mutex::new(vec![]));

    let messages_clone1 = received_messages.clone();
    pubsub
        .subscribe("test_topic", "slow_cb", "target1", SubOption::Always, move |message: Arc<String>| {
            let messages = messages_clone1.clone();
            Box::pin(async move {
                sleep(Duration::from_millis(100)).await;
                messages.lock().unwrap().push(format!("slow: {}", *message));
                Ok(())
            })
        })
        .await
        .unwrap();

    let messages_clone2 = received_messages.clone();
    pubsub
        .subscribe("test_topic", "fast_cb", "target2", SubOption::Always, move |message: Arc<String>| {
            let messages = messages_clone2.clone();
            Box::pin(async move {
                messages.lock().unwrap().push(format!("fast: {}", *message));
                Ok(())
            })
        })
        .await
        .unwrap();

    let pubsub_clone = pubsub.clone();
    tokio::spawn(async move {
        pubsub_clone.publish("test_topic", "hello".to_string()).await.unwrap();
    });

    sleep(Duration::from_millis(20)).await;
    
    {
        let messages = received_messages.lock().unwrap();
        assert_eq!(messages.len(), 1, "Fast callback was blocked by the slow callback!");
        assert_eq!(messages[0], "fast: hello");
    }

    sleep(Duration::from_millis(100)).await;
    {
        let messages = received_messages.lock().unwrap();
        assert_eq!(messages.len(), 2);
        assert!(messages.contains(&"slow: hello".to_string()));
    }
}

#[tokio::test]
async fn test_unsubscribe() {
    let pubsub = PubSub::<String>::new();
    let received_messages = Arc::new(Mutex::new(vec![]));

    let messages_clone = received_messages.clone();
    pubsub
        .subscribe("test_topic", "cb1", "target1", SubOption::Always, move |message: Arc<String>| {
            let messages = messages_clone.clone();
            Box::pin(async move {
                messages.lock().unwrap().push((*message).clone());
                Ok(())
            })
        })
        .await
        .unwrap();

    pubsub
        .publish("test_topic", "hello 1".to_string())
        .await
        .unwrap();
    sleep(Duration::from_millis(10)).await;

    pubsub.unsubscribe("test_topic", Some("cb1"), "target1").await.unwrap();

    pubsub
        .publish("test_topic", "hello 2".to_string())
        .await
        .unwrap();
    sleep(Duration::from_millis(10)).await;

    let messages = received_messages.lock().unwrap();
    assert_eq!(messages.len(), 1); 
    assert_eq!(messages[0], "hello 1");
}

#[tokio::test]
async fn test_sub_option_once() {
    let pubsub = PubSub::<String>::new();
    let received_messages = Arc::new(Mutex::new(vec![]));

    let messages_clone = received_messages.clone();
    pubsub
        .subscribe("test_topic", "cb1", "target1", SubOption::Once, move |message: Arc<String>| {
            let messages = messages_clone.clone();
            Box::pin(async move {
                messages.lock().unwrap().push((*message).clone());
                Ok(())
            })
        })
        .await
        .unwrap();

    pubsub
        .publish("test_topic", "hello 1".to_string())
        .await
        .unwrap();
    sleep(Duration::from_millis(10)).await;

    pubsub
        .publish("test_topic", "hello 2".to_string())
        .await
        .unwrap();
    sleep(Duration::from_millis(10)).await;

    let messages = received_messages.lock().unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0], "hello 1");
}
