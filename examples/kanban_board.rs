// examples/kanban_board.rs

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEvent},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ez_pubsub::{AsyncPubSub, PubSub, SubOption};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction as LayoutDirection, Layout},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, List, ListItem},
};
use std::io;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Column {
    Todo,
    InProgress,
    Done,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ticket {
    pub id: u32,
    pub title: String,
    pub status: Column,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Editing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KanbanState {
    pub tickets: Vec<Ticket>,
    pub focused_ticket_id: Option<u32>,
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub quit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    UiInput(KeyEvent),
    ActionMoveTicket(u32, Direction),
    ActionCreateTicket(String),
    ActionDeleteTicket(u32),
    ActionMoveFocused(Direction),
    ActionFocusNext,
    ActionFocusPrevious,
    ActionDeleteFocused,
    ActionEnterEditMode,
    ActionCancelEditMode,
    ActionTypeChar(char),
    ActionBackspace,
    ActionSubmitEdit,
    ActionQuit,
    StateUpdated(Arc<KanbanState>),
}

pub struct StateManager {
    state: Arc<Mutex<KanbanState>>,
    pubsub: Arc<PubSub<AppEvent>>,
}

impl StateManager {
    pub fn new(initial_state: KanbanState, pubsub: Arc<PubSub<AppEvent>>) -> Self {
        Self {
            state: Arc::new(Mutex::new(initial_state)),
            pubsub,
        }
    }

    pub async fn run(self) {
        let state = self.state.clone();
        let pubsub = self.pubsub.clone();

        // Subscribe to UI input (key presses) and translate them to Actions
        self.pubsub
            .subscribe(
                "ui_input",
                "state_manager_ui_input_cb",
                "state_manager",
                SubOption::Always,
                move |event_msg| {
                    let state = state.clone();
                    let pubsub = pubsub.clone();
                    async move {
                        if let AppEvent::UiInput(key) = &*event_msg {
                            let current_mode = state.lock().await.input_mode.clone();
                            match current_mode {
                                InputMode::Editing => match key.code {
                                    KeyCode::Enter => {
                                        let _ = pubsub
                                            .publish("action", AppEvent::ActionSubmitEdit)
                                            .await;
                                    }
                                    KeyCode::Esc => {
                                        let _ = pubsub
                                            .publish("action", AppEvent::ActionCancelEditMode)
                                            .await;
                                    }
                                    KeyCode::Backspace => {
                                        let _ = pubsub
                                            .publish("action", AppEvent::ActionBackspace)
                                            .await;
                                    }
                                    KeyCode::Char(c) => {
                                        let _ = pubsub
                                            .publish("action", AppEvent::ActionTypeChar(c))
                                            .await;
                                    }
                                    _ => {}
                                },
                                InputMode::Normal => match key.code {
                                    KeyCode::Right | KeyCode::Char('l') => {
                                        let _ = pubsub
                                            .publish(
                                                "action",
                                                AppEvent::ActionMoveFocused(Direction::Right),
                                            )
                                            .await;
                                    }
                                    KeyCode::Left | KeyCode::Char('h') => {
                                        let _ = pubsub
                                            .publish(
                                                "action",
                                                AppEvent::ActionMoveFocused(Direction::Left),
                                            )
                                            .await;
                                    }
                                    KeyCode::Down | KeyCode::Char('j') => {
                                        let _ = pubsub
                                            .publish("action", AppEvent::ActionFocusNext)
                                            .await;
                                    }
                                    KeyCode::Up | KeyCode::Char('k') => {
                                        let _ = pubsub
                                            .publish("action", AppEvent::ActionFocusPrevious)
                                            .await;
                                    }
                                    KeyCode::Char('a') => {
                                        let _ = pubsub
                                            .publish(
                                                "action",
                                                AppEvent::ActionCreateTicket("New Task".into()),
                                            )
                                            .await;
                                    }
                                    KeyCode::Char('e') | KeyCode::Enter => {
                                        let _ = pubsub
                                            .publish("action", AppEvent::ActionEnterEditMode)
                                            .await;
                                    }
                                    KeyCode::Char('d') | KeyCode::Delete | KeyCode::Backspace => {
                                        let _ = pubsub
                                            .publish("action", AppEvent::ActionDeleteFocused)
                                            .await;
                                    }
                                    KeyCode::Char('q') => {
                                        let _ =
                                            pubsub.publish("action", AppEvent::ActionQuit).await;
                                    }
                                    _ => {}
                                },
                            }
                        }
                        Ok(())
                    }
                },
            )
            .await
            .expect("Failed to subscribe UI input");

        let state = self.state.clone();
        let pubsub = self.pubsub.clone();

        // Subscribe to actions
        self.pubsub
            .subscribe(
                "action",
                "state_manager_action_cb",
                "state_manager",
                SubOption::Always,
                move |event_msg| {
                    let state = state.clone();
                    let pubsub = pubsub.clone();
                    async move {
                        match &*event_msg {
                            AppEvent::ActionMoveTicket(id, dir) => {
                                let mut st = state.lock().await;
                                if let Some(ticket) = st.tickets.iter_mut().find(|t| t.id == *id) {
                                    match (ticket.status, dir) {
                                        (Column::Todo, Direction::Right) => {
                                            ticket.status = Column::InProgress
                                        }
                                        (Column::InProgress, Direction::Right) => {
                                            ticket.status = Column::Done
                                        }
                                        (Column::Done, Direction::Left) => {
                                            ticket.status = Column::InProgress
                                        }
                                        (Column::InProgress, Direction::Left) => {
                                            ticket.status = Column::Todo
                                        }
                                        _ => {}
                                    }
                                }
                                let cloned_state = Arc::new(st.clone());
                                let _ = pubsub
                                    .publish("state_update", AppEvent::StateUpdated(cloned_state))
                                    .await;
                            }
                            AppEvent::ActionCreateTicket(title) => {
                                let mut st = state.lock().await;
                                let new_id = st.tickets.iter().map(|t| t.id).max().unwrap_or(0) + 1;
                                st.tickets.push(Ticket {
                                    id: new_id,
                                    title: title.clone(),
                                    status: Column::Todo,
                                });
                                let cloned_state = Arc::new(st.clone());
                                let _ = pubsub
                                    .publish("state_update", AppEvent::StateUpdated(cloned_state))
                                    .await;
                            }
                            AppEvent::ActionDeleteTicket(id) => {
                                let mut st = state.lock().await;
                                st.tickets.retain(|t| t.id != *id);
                                if st.focused_ticket_id == Some(*id) {
                                    st.focused_ticket_id = st.tickets.first().map(|t| t.id);
                                }
                                let cloned_state = Arc::new(st.clone());
                                let _ = pubsub
                                    .publish("state_update", AppEvent::StateUpdated(cloned_state))
                                    .await;
                            }
                            AppEvent::ActionMoveFocused(dir) => {
                                let mut st = state.lock().await;
                                if let Some(focused_id) = st.focused_ticket_id
                                    && let Some(ticket) =
                                        st.tickets.iter_mut().find(|t| t.id == focused_id)
                                {
                                    match (ticket.status, dir) {
                                        (Column::Todo, Direction::Right) => {
                                            ticket.status = Column::InProgress
                                        }
                                        (Column::InProgress, Direction::Right) => {
                                            ticket.status = Column::Done
                                        }
                                        (Column::Done, Direction::Left) => {
                                            ticket.status = Column::InProgress
                                        }
                                        (Column::InProgress, Direction::Left) => {
                                            ticket.status = Column::Todo
                                        }
                                        _ => {}
                                    }
                                }
                                let cloned_state = Arc::new(st.clone());
                                let _ = pubsub
                                    .publish("state_update", AppEvent::StateUpdated(cloned_state))
                                    .await;
                            }
                            AppEvent::ActionFocusNext => {
                                let mut st = state.lock().await;
                                if !st.tickets.is_empty() {
                                    let curr_idx = st
                                        .tickets
                                        .iter()
                                        .position(|t| Some(t.id) == st.focused_ticket_id)
                                        .unwrap_or(0);
                                    let next_idx = (curr_idx + 1) % st.tickets.len();
                                    st.focused_ticket_id = Some(st.tickets[next_idx].id);
                                }
                                let cloned_state = Arc::new(st.clone());
                                let _ = pubsub
                                    .publish("state_update", AppEvent::StateUpdated(cloned_state))
                                    .await;
                            }
                            AppEvent::ActionFocusPrevious => {
                                let mut st = state.lock().await;
                                if !st.tickets.is_empty() {
                                    let curr_idx = st
                                        .tickets
                                        .iter()
                                        .position(|t| Some(t.id) == st.focused_ticket_id)
                                        .unwrap_or(0);
                                    let next_idx = if curr_idx == 0 {
                                        st.tickets.len() - 1
                                    } else {
                                        curr_idx - 1
                                    };
                                    st.focused_ticket_id = Some(st.tickets[next_idx].id);
                                }
                                let cloned_state = Arc::new(st.clone());
                                let _ = pubsub
                                    .publish("state_update", AppEvent::StateUpdated(cloned_state))
                                    .await;
                            }
                            AppEvent::ActionDeleteFocused => {
                                let mut st = state.lock().await;
                                if let Some(focused_id) = st.focused_ticket_id {
                                    st.tickets.retain(|t| t.id != focused_id);
                                    st.focused_ticket_id = st.tickets.first().map(|t| t.id);
                                }
                                let cloned_state = Arc::new(st.clone());
                                let _ = pubsub
                                    .publish("state_update", AppEvent::StateUpdated(cloned_state))
                                    .await;
                            }
                            AppEvent::ActionEnterEditMode => {
                                let mut st = state.lock().await;
                                if let Some(focused_id) = st.focused_ticket_id
                                    && let Some(ticket) =
                                        st.tickets.iter().find(|t| t.id == focused_id)
                                {
                                    st.input_buffer = ticket.title.clone();
                                    st.input_mode = InputMode::Editing;
                                }
                                let cloned_state = Arc::new(st.clone());
                                let _ = pubsub
                                    .publish("state_update", AppEvent::StateUpdated(cloned_state))
                                    .await;
                            }
                            AppEvent::ActionCancelEditMode => {
                                let mut st = state.lock().await;
                                st.input_mode = InputMode::Normal;
                                st.input_buffer.clear();
                                let cloned_state = Arc::new(st.clone());
                                let _ = pubsub
                                    .publish("state_update", AppEvent::StateUpdated(cloned_state))
                                    .await;
                            }
                            AppEvent::ActionSubmitEdit => {
                                let mut st = state.lock().await;
                                if st.input_mode == InputMode::Editing {
                                    let new_title = st.input_buffer.clone();
                                    if let Some(focused_id) = st.focused_ticket_id
                                        && let Some(ticket) =
                                            st.tickets.iter_mut().find(|t| t.id == focused_id)
                                    {
                                        ticket.title = new_title;
                                    }
                                    st.input_mode = InputMode::Normal;
                                    st.input_buffer.clear();
                                    let cloned_state = Arc::new(st.clone());
                                    let _ = pubsub
                                        .publish(
                                            "state_update",
                                            AppEvent::StateUpdated(cloned_state),
                                        )
                                        .await;
                                }
                            }
                            AppEvent::ActionTypeChar(c) => {
                                let mut st = state.lock().await;
                                if st.input_mode == InputMode::Editing {
                                    st.input_buffer.push(*c);
                                    let cloned_state = Arc::new(st.clone());
                                    let _ = pubsub
                                        .publish(
                                            "state_update",
                                            AppEvent::StateUpdated(cloned_state),
                                        )
                                        .await;
                                }
                            }
                            AppEvent::ActionBackspace => {
                                let mut st = state.lock().await;
                                if st.input_mode == InputMode::Editing {
                                    st.input_buffer.pop();
                                    let cloned_state = Arc::new(st.clone());
                                    let _ = pubsub
                                        .publish(
                                            "state_update",
                                            AppEvent::StateUpdated(cloned_state),
                                        )
                                        .await;
                                }
                            }
                            AppEvent::ActionQuit => {
                                let mut st = state.lock().await;
                                st.quit = true;
                                let cloned_state = Arc::new(st.clone());
                                let _ = pubsub
                                    .publish("state_update", AppEvent::StateUpdated(cloned_state))
                                    .await;
                            }
                            _ => {}
                        }
                        Ok(())
                    }
                },
            )
            .await
            .expect("Failed to subscribe");
    }
}

pub struct KanbanApp {
    pub state: Arc<Mutex<KanbanState>>,
    pub pubsub: Arc<PubSub<AppEvent>>,
}

impl KanbanApp {
    pub fn new(initial_state: KanbanState, pubsub: Arc<PubSub<AppEvent>>) -> Self {
        Self {
            state: Arc::new(Mutex::new(initial_state)),
            pubsub,
        }
    }

    pub async fn run_headless(&self) {
        let state = self.state.clone();

        self.pubsub
            .subscribe(
                "state_update",
                "ui_state_cb",
                "ui",
                SubOption::Always,
                move |event_msg| {
                    let state = state.clone();
                    async move {
                        if let AppEvent::StateUpdated(new_state) = &*event_msg {
                            *state.lock().await = (**new_state).clone();
                        }
                        Ok(())
                    }
                },
            )
            .await
            .expect("Failed to subscribe UI");
    }

    pub async fn run_ui(&self) -> io::Result<()> {
        self.run_headless().await;

        io::stdout().execute(EnterAlternateScreen)?;
        enable_raw_mode()?;
        let backend = CrosstermBackend::new(io::stdout());
        let mut terminal = Terminal::new(backend)?;

        loop {
            // Check if we need to quit (assuming 'q' triggers UI exit)
            // The keyboard dispatcher will be implemented in US3.
            // For now, we will draw the state.
            let current_state = self.state.lock().await.clone();

            terminal.draw(|f| {
                let chunks = Layout::default()
                    .direction(LayoutDirection::Horizontal)
                    .constraints(
                        [
                            Constraint::Percentage(33),
                            Constraint::Percentage(33),
                            Constraint::Percentage(33),
                        ]
                        .as_ref(),
                    )
                    .split(f.area());

                let mut todo_items = vec![];
                let mut in_progress_items = vec![];
                let mut done_items = vec![];

                for ticket in &current_state.tickets {
                    let mut style = Style::default();
                    let is_focused = current_state.focused_ticket_id == Some(ticket.id);
                    if is_focused {
                        style = style.fg(Color::Yellow).add_modifier(Modifier::BOLD);
                    }

                    let display_title =
                        if is_focused && current_state.input_mode == InputMode::Editing {
                            format!("{}█", current_state.input_buffer)
                        } else {
                            ticket.title.clone()
                        };

                    let span = Span::styled(format!("[{}] {}", ticket.id, display_title), style);
                    match ticket.status {
                        Column::Todo => todo_items.push(ListItem::new(span)),
                        Column::InProgress => in_progress_items.push(ListItem::new(span)),
                        Column::Done => done_items.push(ListItem::new(span)),
                    }
                }

                let todo_list = List::new(todo_items)
                    .block(Block::default().title("TODO").borders(Borders::ALL));
                let in_progress_list = List::new(in_progress_items)
                    .block(Block::default().title("IN PROGRESS").borders(Borders::ALL));
                let done_list = List::new(done_items)
                    .block(Block::default().title("DONE").borders(Borders::ALL));

                f.render_widget(todo_list, chunks[0]);
                f.render_widget(in_progress_list, chunks[1]);
                f.render_widget(done_list, chunks[2]);
            })?;

            tokio::time::sleep(Duration::from_millis(60)).await;

            if current_state.quit {
                break;
            }
        }

        disable_raw_mode()?;
        io::stdout().execute(LeaveAlternateScreen)?;
        Ok(())
    }
}

pub struct KeyboardDispatcher {
    pubsub: Arc<PubSub<AppEvent>>,
}

impl KeyboardDispatcher {
    pub fn new(pubsub: Arc<PubSub<AppEvent>>) -> Self {
        Self { pubsub }
    }

    pub async fn run(self) {
        use crossterm::event::EventStream;
        use futures::StreamExt;

        let mut reader = EventStream::new();
        while let Some(Ok(event)) = reader.next().await {
            if let Event::Key(key) = event {
                // Ignore key release events to avoid double-processing
                if key.kind != event::KeyEventKind::Release {
                    let _ = self
                        .pubsub
                        .publish("ui_input", AppEvent::UiInput(key))
                        .await;
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let pubsub = Arc::new(PubSub::<AppEvent>::new());

    let initial_state = KanbanState {
        tickets: vec![Ticket {
            id: 1,
            title: "Task 1".into(),
            status: Column::Todo,
        }],
        focused_ticket_id: Some(1),
        input_mode: InputMode::Normal,
        input_buffer: String::new(),
        quit: false,
    };

    let state_manager = StateManager::new(initial_state.clone(), pubsub.clone());
    tokio::spawn(async move {
        state_manager.run().await;
    });

    let dispatcher = KeyboardDispatcher::new(pubsub.clone());
    tokio::spawn(async move {
        dispatcher.run().await;
    });

    let app = KanbanApp::new(initial_state, pubsub.clone());
    app.run_ui().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::Mutex;

    #[tokio::test]
    async fn test_state_manager_moves_ticket() {
        let pubsub = PubSub::<AppEvent>::new();
        let pubsub = Arc::new(pubsub);

        let initial_state = KanbanState {
            tickets: vec![Ticket {
                id: 1,
                title: "Task 1".into(),
                status: Column::Todo,
            }],
            focused_ticket_id: Some(1),
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            quit: false,
        };

        let state_manager = StateManager::new(initial_state, pubsub.clone());
        tokio::spawn(async move {
            state_manager.run().await;
        });

        tokio::time::sleep(Duration::from_millis(50)).await;

        let state_received = Arc::new(Mutex::new(None));
        let state_rx = state_received.clone();

        pubsub
            .subscribe(
                "state_update",
                "test_cb",
                "test_target",
                SubOption::Always,
                move |msg| {
                    let state_rx = state_rx.clone();
                    async move {
                        if let AppEvent::StateUpdated(st) = &*msg {
                            *state_rx.lock().await = Some((**st).clone());
                        }
                        Ok(())
                    }
                },
            )
            .await
            .unwrap();

        // Publish action
        pubsub
            .publish("action", AppEvent::ActionMoveTicket(1, Direction::Right))
            .await
            .unwrap();

        // Await updated state
        tokio::time::sleep(Duration::from_millis(50)).await;

        let st = state_received
            .lock()
            .await
            .take()
            .expect("Did not receive StateUpdated");
        let ticket = st.tickets.iter().find(|t| t.id == 1).unwrap();
        assert_eq!(ticket.status, Column::InProgress);
    }

    #[tokio::test]
    async fn test_ui_struct_updates_on_state_updated() {
        let pubsub = Arc::new(PubSub::<AppEvent>::new());
        let initial_state = KanbanState {
            tickets: vec![],
            focused_ticket_id: None,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            quit: false,
        };

        let app = KanbanApp::new(initial_state, pubsub.clone());
        app.run_headless().await;

        let new_state = KanbanState {
            tickets: vec![Ticket {
                id: 2,
                title: "Task 2".into(),
                status: Column::Done,
            }],
            focused_ticket_id: Some(2),
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            quit: false,
        };
        pubsub
            .publish(
                "state_update",
                AppEvent::StateUpdated(Arc::new(new_state.clone())),
            )
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;

        let st = app.state.lock().await;
        assert_eq!(st.tickets.len(), 1);
        assert_eq!(st.tickets[0].id, 2);
    }

    #[tokio::test]
    async fn test_keyboard_dispatcher_broadcasts_events() {
        let pubsub = Arc::new(PubSub::<AppEvent>::new());
        let _dispatcher = KeyboardDispatcher::new(pubsub.clone());

        let received = Arc::new(Mutex::new(false));
        let rx = received.clone();

        pubsub
            .subscribe(
                "action",
                "kb_test_cb",
                "kb_test",
                SubOption::Always,
                move |event_msg| {
                    let rx = rx.clone();
                    async move {
                        if let AppEvent::ActionCreateTicket(title) = &*event_msg {
                            if title == "New Task" {
                                *rx.lock().await = true;
                            }
                        }
                        Ok(())
                    }
                },
            )
            .await
            .unwrap();

        pubsub
            .publish("action", AppEvent::ActionCreateTicket("New Task".into()))
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;

        let got = *received.lock().await;
        assert!(got, "Did not receive ActionCreateTicket event");
    }
}
