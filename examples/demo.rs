///
/// ez_pubsub — Smart Home Event System Demo
/// Showcases every feature of the crate:
///   • Multiple distinct events
///   • Multiple targets per event
///   • SubOption::Always  (persistent callbacks)
///   • SubOption::Once    (self-removing callbacks)
///   • Named callback unsubscription
///   • Full target unsubscription
///   • remove_all_subscriptions()
///   • All three subscribe! macro forms
///   • Direct instance API (PubSub<T>)
///   • Global bus via broadcast! / subscribe! macros
use ez_pubsub::{PubSub, SubOption, broadcast, subscribe};

// ── TUI Instantiation─────────────────────────────────────────────────────────────────
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::Rect,
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
};
use std::io;

// ── Helpers ─────────────────────────────────────────────────────────────────

fn section(title: &str) {
    println!("\n{}", "─".repeat(55));
    println!("  {}", title);
    println!("{}\n", "─".repeat(55));
}

// ── 1. Instance API: typed sensor bus ───────────────────────────────────────
//
// The instance API lets you create a fully typed bus for any payload.
// Here we use PubSub<f32> to carry temperature readings.

fn run_sensor_bus() {
    section("1. Instance API — Typed Temperature Sensor Bus");

    let sensor_bus: PubSub<f32> = PubSub::new();

    // Living-room thermostat: always-on controller
    sensor_bus.subscribe(
        "temperature",
        "thermostat",
        "living_room",
        SubOption::Always,
        |temp: &f32| {
            if *temp > 28.0 {
                println!("  [THERMOSTAT]  🌡️  {}°C — Turning AC on", temp);
            } else {
                println!("  [THERMOSTAT]  🌡️  {}°C — AC off, comfortable", temp);
            }
        },
    );

    // Boot-time alert: fires only once, then auto-removes
    sensor_bus.subscribe(
        "temperature",
        "boot_alert",
        "living_room",
        SubOption::Once,
        |temp: &f32| {
            println!(
                "  [BOOT ALERT]  ✅  Sensor online, first reading: {}°C",
                temp
            );
        },
    );

    // Bedroom sensor: separate target on the same event
    sensor_bus.subscribe(
        "temperature",
        "bedroom_log",
        "bedroom",
        SubOption::Always,
        |temp: &f32| {
            println!("  [BEDROOM LOG] 🛏️  Bedroom temp: {}°C", temp);
        },
    );

    println!("  >> Reading 1 (boot)");
    sensor_bus.broadcast("temperature", &22.5);

    println!("\n  >> Reading 2 (boot_alert is now gone)");
    sensor_bus.broadcast("temperature", &30.1);

    println!("\n  >> Unsubscribing bedroom sensor entirely...");
    sensor_bus.unsubscribe("temperature", None, "bedroom");

    println!("  >> Reading 3 (no bedroom output)");
    sensor_bus.broadcast("temperature", &25.0);

    println!("\n  >> Unsubscribing only thermostat callback from living_room...");
    sensor_bus.unsubscribe("temperature", Some("thermostat"), "living_room");

    println!("  >> Reading 4 (living_room silent)");
    sensor_bus.broadcast("temperature", &26.0);
}

// ── 2. Global bus: multi-room security system ───────────────────────────────
//
// The global bus uses PubSub<String> and is accessed through macros.
// This simulates motion, door, and alarm events across the house.

fn run_security_system() {
    section("2. Global Bus — Multi-Room Security System");

    // ── Motion events ──────────────────────────────────────────────────────

    // Full form: explicit name + target + option
    subscribe!(
        "motion",
        "security_camera",
        "front_door",
        SubOption::Always,
        |location: &String| {
            println!("  [CAMERA]   📷  Motion recorded at: {}", location);
        }
    );

    // Full form: Once — sends a push notification only on the very first motion
    subscribe!(
        "motion",
        "push_notify",
        "front_door",
        SubOption::Once,
        |location: &String| {
            println!(
                "  [PUSH]     📲  First motion detected at {} — notifying owner",
                location
            );
        }
    );

    // Medium form: name + target, defaults to Always
    subscribe!("motion", "motion_log", "garage", |location: &String| {
        println!("  [LOG]      📝  Garage motion log: {}", location);
    });

    println!("  >> Motion event 1 (push fires once)");
    broadcast!("motion", "front_door at 09:01");

    println!("\n  >> Motion event 2 (push already removed)");
    broadcast!("motion", "front_door at 09:03");

    println!("\n  >> Motion event 3 (garage)");
    broadcast!("motion", "garage at 09:10");

    // ── Door events ────────────────────────────────────────────────────────

    // Short form: auto-generated name from file!() + line!()
    subscribe!("door", |action: &String| {
        println!("  [DOOR]     🚪  Door event → {}", action);
    });

    subscribe!(
        "door",
        "lock_controller",
        "main_entrance",
        SubOption::Always,
        |action: &String| {
            if action.contains("open") {
                println!(
                    "  [LOCK]     🔓  Main entrance unlocked by event: {}",
                    action
                );
            } else {
                println!("  [LOCK]     🔒  Main entrance locked by event: {}", action);
            }
        }
    );

    println!("\n  >> Door opened");
    broadcast!("door", "open:main_entrance:09:15");

    println!("\n  >> Unsubscribing lock_controller from main_entrance...");
    ez_pubsub::global_bus().unsubscribe("door", Some("lock_controller"), "main_entrance");

    println!("  >> Door closed (lock_controller silent)");
    broadcast!("door", "close:main_entrance:09:20");

    // ── Alarm event ────────────────────────────────────────────────────────

    subscribe!(
        "alarm",
        "siren",
        "security_panel",
        SubOption::Always,
        |level: &String| {
            println!("  [SIREN]    🚨  ALARM — level: {}", level);
        }
    );

    subscribe!(
        "alarm",
        "auto_call",
        "security_panel",
        SubOption::Once,
        |level: &String| {
            println!(
                "  [AUTO-CALL]📞  Calling emergency services! Level: {}",
                level
            );
        }
    );

    println!("\n  >> Alarm triggered (auto_call fires once)");
    broadcast!("alarm", "HIGH");

    println!("\n  >> Alarm triggered again (auto_call already removed)");
    broadcast!("alarm", "MEDIUM");
}

// ── 3. remove_all_subscriptions ─────────────────────────────────────────────
//
// Wipes every subscription on the global bus in one call.
// Broadcasts after this produce no output.

fn run_system_shutdown() {
    section("3. System Shutdown — remove_all_subscriptions()");

    // Ensure something is subscribed before clearing
    subscribe!(
        "status",
        "status_display",
        "dashboard",
        SubOption::Always,
        |msg: &String| {
            println!("  [DASHBOARD] 🖥️  Status: {}", msg);
        }
    );

    println!("  >> Broadcasting before shutdown");
    broadcast!("status", "All systems nominal");

    println!("\n  >> Calling remove_all_subscriptions()...");
    ez_pubsub::global_bus().remove_all_subscriptions();

    println!("  >> Broadcasting after shutdown (no output expected)");
    broadcast!("status", "This should be silent");
    broadcast!("motion", "This should be silent");
    broadcast!("alarm", "This should be silent");

    println!("  ✅  Confirmed: global bus is empty");
}

// ── TUI UI─────────────────────────────────────────────────────────────────
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Debug, Default, Clone)]
pub struct App {
    counter: Arc<AtomicU8>,
    exit: bool,
}

impl App {
    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        // register pubsub
        let increment_clone = self.clone();
        let decrement_clone = self.clone();

        // NOTE: pubsub mechanism to increase the counter?
        subscribe!(
            "increment",
            "security_camera",
            "front_door",
            SubOption::Always,
            move |_: &_| {
                let mut temp_inc_clone = increment_clone.clone();
                temp_inc_clone.increment_counter();
            }
        );
        subscribe!(
            "decrement",
            "security_camera",
            "front_door",
            SubOption::Always,
            move |_| {
                let mut temp_dec_clone = decrement_clone.clone();
                temp_dec_clone.decrement_counter();
            }
        );

        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            // it's important to check that the event is a key press events
            // as crossterm also emits key release and repeat events in Windows.
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => self.exit(),
            KeyCode::Left => {
                broadcast!("decrement", "front_door");
                // self.decrement_counter()
            }
            KeyCode::Right => {
                broadcast!("increment", "front_door");
                // self.increment_counter()
            }
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn decrement_counter(&mut self) {
        if self.counter.load(Ordering::Relaxed) > 0 {
            self.counter.fetch_sub(1, Ordering::Relaxed);
        }
    }

    fn increment_counter(&mut self) {
        self.counter.fetch_add(1, Ordering::Relaxed);
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(" Counter App Tutorial ".bold());
        let instructions = Line::from(vec![
            " Decrement ".into(),
            "<left>".blue().bold(),
            " Increment ".into(),
            "<right>".blue().bold(),
            " Quit ".into(),
            "<Q>".red().bold(),
        ]);
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        let counter_text = Text::from(vec![
            Line::from(vec![
                "Value: ".into(),
                self.counter
                    .load(std::sync::atomic::Ordering::Relaxed)
                    .to_string()
                    .yellow(),
            ]),
            Line::from(vec![
                "Second Value: ".into(),
                self.counter
                    .load(std::sync::atomic::Ordering::Relaxed)
                    .to_string()
                    .yellow(),
            ]),
        ]);

        Paragraph::new(counter_text)
            .centered()
            .block(block)
            .render(area, buf);
    }
}

// ── Entry point ──────────────────────────────────────────────────────────────

fn main() -> io::Result<()> {
    println!("\n╔═══════════════════════════════════════════════════════╗");
    println!("║        ez_pubsub — Smart Home Event System Demo       ║");
    println!("╚═══════════════════════════════════════════════════════╝");

    run_sensor_bus();
    run_security_system();
    run_system_shutdown();

    println!("\n╔═══════════════════════════════════════════════════════╗");
    println!("║                     Demo Complete                     ║");
    println!("╚═══════════════════════════════════════════════════════╝\n");

    // Ok(())

    ratatui::run(|terminal| App::default().run(terminal))
}
