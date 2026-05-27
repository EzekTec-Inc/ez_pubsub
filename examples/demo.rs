use ez_pubsub::{AsyncPubSub, PubSub, SubOption};
use std::sync::Arc;
// use tokio;

fn section(title: &str) {
    println!("\n{}", "─".repeat(55));
    println!("  {}", title);
    println!("{}\n", "─".repeat(55));
}

async fn run_sensor_bus() {
    section("1. Instance API — Typed Temperature Sensor Bus");

    let sensor_bus: PubSub<f32> = PubSub::new();

    // Living-room thermostat: always-on controller
    sensor_bus
        .subscribe(
            "temperature",
            "thermostat",
            "living_room",
            SubOption::Always,
            |temp: Arc<f32>| {
                Box::pin(async move {
                    if *temp > 28.0 {
                        println!("  [THERMOSTAT]  🌡️  {}°C — Turning AC on", temp);
                    } else {
                        println!("  [THERMOSTAT]  🌡️  {}°C — AC off, comfortable", temp);
                    }
                    Ok(())
                })
            },
        )
        .await
        .unwrap();

    // Boot-time alert: fires only once, then auto-removes
    sensor_bus
        .subscribe(
            "temperature",
            "boot_alert",
            "living_room",
            SubOption::Once,
            |temp: Arc<f32>| {
                Box::pin(async move {
                    println!(
                        "  [BOOT ALERT]  ✅  Sensor online, first reading: {}°C",
                        temp
                    );
                    Ok(())
                })
            },
        )
        .await
        .unwrap();

    println!("  >> Reading 1 (boot)");
    sensor_bus.publish("temperature", 22.5).await.unwrap();

    println!("\n  >> Reading 2");
    sensor_bus.publish("temperature", 30.1).await.unwrap();

    // Demonstrate unsubscribe
    sensor_bus
        .unsubscribe("temperature", Some("thermostat"), "living_room")
        .await
        .unwrap();
    println!("\n  >> Reading 3 (unsubscribed)");
    sensor_bus.publish("temperature", 25.0).await.unwrap();
}

#[tokio::main]
async fn main() {
    run_sensor_bus().await;
}
