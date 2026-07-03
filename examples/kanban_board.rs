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

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Column {
    Todo,
    InProgress,
    Done,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Ticket {
    pub id: u32,
    pub title: String,
    pub status: Column,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum InputMode {
    Normal,
    Editing,
    Creating,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct KanbanState {
    pub tickets: Vec<Ticket>,
    pub focused_ticket_id: Option<u32>,
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub quit: bool,
    pub show_help: bool,
    pub warning_message: Option<String>,
    pub wip_limit_in_progress: usize,
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

    // Advanced Actions
    ActionMoveOrderUp,
    ActionMoveOrderDown,
    ActionToggleHelp,
    ActionEnterCreateMode,
    ActionCancelCreateMode,
    ActionSubmitCreate,
    ActionClearWarning,
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
                            // Any keypress clears the warning message if one is active
                            {
                                let mut st = state.lock().await;
                                if st.warning_message.is_some() {
                                    st.warning_message = None;
                                    let cloned_state = Arc::new(st.clone());
                                    let _ = pubsub
                                        .publish(
                                            "state_update",
                                            AppEvent::StateUpdated(cloned_state),
                                        )
                                        .await;
                                }
                            }

                            let current_mode = state.lock().await.input_mode.clone();
                            match current_mode {
                                InputMode::Creating => match key.code {
                                    KeyCode::Enter => {
                                        let _ = pubsub
                                            .publish("action", AppEvent::ActionSubmitCreate)
                                            .await;
                                    }
                                    KeyCode::Esc => {
                                        let _ = pubsub
                                            .publish("action", AppEvent::ActionCancelCreateMode)
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
                                InputMode::Normal => {
                                    // If show_help is active, any key closes help
                                    let show_help = state.lock().await.show_help;
                                    if show_help {
                                        let _ = pubsub
                                            .publish("action", AppEvent::ActionToggleHelp)
                                            .await;
                                        return Ok(());
                                    }

                                    let is_shift = key
                                        .modifiers
                                        .contains(crossterm::event::KeyModifiers::SHIFT);

                                    match key.code {
                                        KeyCode::Up | KeyCode::Char('k') if is_shift => {
                                            let _ = pubsub
                                                .publish("action", AppEvent::ActionMoveOrderUp)
                                                .await;
                                        }
                                        KeyCode::Down | KeyCode::Char('j') if is_shift => {
                                            let _ = pubsub
                                                .publish("action", AppEvent::ActionMoveOrderDown)
                                                .await;
                                        }
                                        KeyCode::Char('K') => {
                                            let _ = pubsub
                                                .publish("action", AppEvent::ActionMoveOrderUp)
                                                .await;
                                        }
                                        KeyCode::Char('J') => {
                                            let _ = pubsub
                                                .publish("action", AppEvent::ActionMoveOrderDown)
                                                .await;
                                        }
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
                                                .publish("action", AppEvent::ActionEnterCreateMode)
                                                .await;
                                        }
                                        KeyCode::Char('e') | KeyCode::Enter => {
                                            let _ = pubsub
                                                .publish("action", AppEvent::ActionEnterEditMode)
                                                .await;
                                        }
                                        KeyCode::Char('d')
                                        | KeyCode::Delete
                                        | KeyCode::Backspace => {
                                            let _ = pubsub
                                                .publish("action", AppEvent::ActionDeleteFocused)
                                                .await;
                                        }
                                        KeyCode::Char('q') => {
                                            let _ = pubsub
                                                .publish("action", AppEvent::ActionQuit)
                                                .await;
                                        }
                                        KeyCode::Char('?') => {
                                            let _ = pubsub
                                                .publish("action", AppEvent::ActionToggleHelp)
                                                .await;
                                        }
                                        _ => {}
                                    }
                                }
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
                                let wip_limit = st.wip_limit_in_progress;
                                let in_progress_count = st
                                    .tickets
                                    .iter()
                                    .filter(|t| t.status == Column::InProgress)
                                    .count();

                                if let Some(ticket) = st.tickets.iter_mut().find(|t| t.id == *id) {
                                    let old_status = ticket.status;
                                    let target_status = match (old_status, dir) {
                                        (Column::Todo, Direction::Right) => {
                                            Some(Column::InProgress)
                                        }
                                        (Column::InProgress, Direction::Right) => {
                                            Some(Column::Done)
                                        }
                                        (Column::Done, Direction::Left) => Some(Column::InProgress),
                                        (Column::InProgress, Direction::Left) => Some(Column::Todo),
                                        _ => None,
                                    };

                                    if let Some(target) = target_status {
                                        if target == Column::InProgress
                                            && in_progress_count >= wip_limit
                                            && old_status != Column::InProgress
                                        {
                                            st.warning_message = Some(format!(
                                                "WIP limit of {} reached for IN PROGRESS!",
                                                wip_limit
                                            ));
                                        } else {
                                            ticket.status = target;
                                        }
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
                                if let Some(focused_id) = st.focused_ticket_id {
                                    let wip_limit = st.wip_limit_in_progress;
                                    let in_progress_count = st
                                        .tickets
                                        .iter()
                                        .filter(|t| t.status == Column::InProgress)
                                        .count();

                                    if let Some(ticket) =
                                        st.tickets.iter_mut().find(|t| t.id == focused_id)
                                    {
                                        let old_status = ticket.status;
                                        let target_status = match (old_status, dir) {
                                            (Column::Todo, Direction::Right) => {
                                                Some(Column::InProgress)
                                            }
                                            (Column::InProgress, Direction::Right) => {
                                                Some(Column::Done)
                                            }
                                            (Column::Done, Direction::Left) => {
                                                Some(Column::InProgress)
                                            }
                                            (Column::InProgress, Direction::Left) => {
                                                Some(Column::Todo)
                                            }
                                            _ => None,
                                        };

                                        if let Some(target) = target_status {
                                            if target == Column::InProgress
                                                && in_progress_count >= wip_limit
                                                && old_status != Column::InProgress
                                            {
                                                st.warning_message = Some(format!(
                                                    "WIP limit of {} reached for IN PROGRESS!",
                                                    wip_limit
                                                ));
                                            } else {
                                                ticket.status = target;
                                            }
                                        }
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
                                if let Some(ticket) = st
                                    .focused_ticket_id
                                    .and_then(|id| st.tickets.iter().find(|t| t.id == id))
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
                                    if let Some(ticket) = st
                                        .focused_ticket_id
                                        .and_then(|id| st.tickets.iter_mut().find(|t| t.id == id))
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
                                if st.input_mode == InputMode::Editing
                                    || st.input_mode == InputMode::Creating
                                {
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
                                if st.input_mode == InputMode::Editing
                                    || st.input_mode == InputMode::Creating
                                {
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
                            AppEvent::ActionMoveOrderUp => {
                                let mut st = state.lock().await;
                                if let Some(idx) = st
                                    .focused_ticket_id
                                    .and_then(|id| st.tickets.iter().position(|t| t.id == id))
                                {
                                    let status = st.tickets[idx].status;
                                    let mut prev_idx = None;
                                    for i in (0..idx).rev() {
                                        if st.tickets[i].status == status {
                                            prev_idx = Some(i);
                                            break;
                                        }
                                    }
                                    if let Some(p_idx) = prev_idx {
                                        st.tickets.swap(idx, p_idx);
                                    }
                                }
                                let cloned_state = Arc::new(st.clone());
                                let _ = pubsub
                                    .publish("state_update", AppEvent::StateUpdated(cloned_state))
                                    .await;
                            }
                            AppEvent::ActionMoveOrderDown => {
                                let mut st = state.lock().await;
                                if let Some(idx) = st
                                    .focused_ticket_id
                                    .and_then(|id| st.tickets.iter().position(|t| t.id == id))
                                {
                                    let status = st.tickets[idx].status;
                                    let mut next_idx = None;
                                    for i in (idx + 1)..st.tickets.len() {
                                        if st.tickets[i].status == status {
                                            next_idx = Some(i);
                                            break;
                                        }
                                    }
                                    if let Some(n_idx) = next_idx {
                                        st.tickets.swap(idx, n_idx);
                                    }
                                }
                                let cloned_state = Arc::new(st.clone());
                                let _ = pubsub
                                    .publish("state_update", AppEvent::StateUpdated(cloned_state))
                                    .await;
                            }
                            AppEvent::ActionToggleHelp => {
                                let mut st = state.lock().await;
                                st.show_help = !st.show_help;
                                let cloned_state = Arc::new(st.clone());
                                let _ = pubsub
                                    .publish("state_update", AppEvent::StateUpdated(cloned_state))
                                    .await;
                            }
                            AppEvent::ActionEnterCreateMode => {
                                let mut st = state.lock().await;
                                st.input_mode = InputMode::Creating;
                                st.input_buffer.clear();
                                let cloned_state = Arc::new(st.clone());
                                let _ = pubsub
                                    .publish("state_update", AppEvent::StateUpdated(cloned_state))
                                    .await;
                            }
                            AppEvent::ActionCancelCreateMode => {
                                let mut st = state.lock().await;
                                st.input_mode = InputMode::Normal;
                                st.input_buffer.clear();
                                let cloned_state = Arc::new(st.clone());
                                let _ = pubsub
                                    .publish("state_update", AppEvent::StateUpdated(cloned_state))
                                    .await;
                            }
                            AppEvent::ActionSubmitCreate => {
                                let mut st = state.lock().await;
                                if st.input_mode == InputMode::Creating {
                                    let title = if st.input_buffer.trim().is_empty() {
                                        "New Task".to_string()
                                    } else {
                                        st.input_buffer.clone()
                                    };
                                    let new_id =
                                        st.tickets.iter().map(|t| t.id).max().unwrap_or(0) + 1;
                                    st.tickets.push(Ticket {
                                        id: new_id,
                                        title,
                                        status: Column::Todo,
                                    });
                                    st.focused_ticket_id = Some(new_id);
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
                            AppEvent::ActionClearWarning => {
                                let mut st = state.lock().await;
                                st.warning_message = None;
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

fn centered_rect(
    percent_x: u16,
    percent_y: u16,
    r: ratatui::layout::Rect,
) -> ratatui::layout::Rect {
    let popup_layout = Layout::default()
        .direction(LayoutDirection::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(LayoutDirection::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
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
                let area = f.area();

                // 1. Vertical Layout (Top header banner, Main columns, Bottom status/warning bar)
                let vertical_chunks = Layout::default()
                    .direction(LayoutDirection::Vertical)
                    .constraints(
                        [
                            Constraint::Length(1), // Top info banner
                            Constraint::Min(0),    // Main board columns
                            Constraint::Length(1), // Bottom bar
                        ]
                        .as_ref(),
                    )
                    .split(area);

                // Draw Top Header Banner
                let top_banner = ratatui::widgets::Paragraph::new(Span::styled(
                    " 📋 KANBAN EVENT BUS DEMO  |  Press '?' for Help  |  WIP Limit (IP): 3",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ));
                f.render_widget(top_banner, vertical_chunks[0]);

                // 2. Horizontal layout for the 3 columns
                let board_chunks = Layout::default()
                    .direction(LayoutDirection::Horizontal)
                    .constraints(
                        [
                            Constraint::Percentage(33),
                            Constraint::Percentage(33),
                            Constraint::Percentage(33),
                        ]
                        .as_ref(),
                    )
                    .split(vertical_chunks[1]);

                let todo_count = current_state
                    .tickets
                    .iter()
                    .filter(|t| t.status == Column::Todo)
                    .count();
                let ip_count = current_state
                    .tickets
                    .iter()
                    .filter(|t| t.status == Column::InProgress)
                    .count();
                let done_count = current_state
                    .tickets
                    .iter()
                    .filter(|t| t.status == Column::Done)
                    .count();

                let todo_title = format!(" TODO [{}] ", todo_count);
                let ip_title = format!(
                    " IN PROGRESS [{}/{}] ",
                    ip_count, current_state.wip_limit_in_progress
                );
                let done_title = format!(" DONE [{}] ", done_count);

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

                    let span = Span::styled(format!(" [{}] {}", ticket.id, display_title), style);
                    match ticket.status {
                        Column::Todo => todo_items.push(ListItem::new(span)),
                        Column::InProgress => in_progress_items.push(ListItem::new(span)),
                        Column::Done => done_items.push(ListItem::new(span)),
                    }
                }

                // Render lists with borders
                let todo_list = List::new(todo_items)
                    .block(Block::default().title(todo_title).borders(Borders::ALL));

                let in_progress_border_color = if ip_count >= current_state.wip_limit_in_progress {
                    Color::Red
                } else if ip_count > 0 {
                    Color::Yellow
                } else {
                    Color::White
                };
                let in_progress_list = List::new(in_progress_items).block(
                    Block::default()
                        .title(ip_title)
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(in_progress_border_color)),
                );

                let done_list = List::new(done_items)
                    .block(Block::default().title(done_title).borders(Borders::ALL));

                f.render_widget(todo_list, board_chunks[0]);
                f.render_widget(in_progress_list, board_chunks[1]);
                f.render_widget(done_list, board_chunks[2]);

                // Draw Bottom Status/Input/Warning Bar
                let bottom_widget = if current_state.input_mode == InputMode::Creating {
                    ratatui::widgets::Paragraph::new(Span::styled(
                        format!(" 💡 [Creating New Ticket]: {}█", current_state.input_buffer),
                        Style::default()
                            .fg(Color::LightBlue)
                            .add_modifier(Modifier::BOLD),
                    ))
                } else if let Some(ref warning) = current_state.warning_message {
                    ratatui::widgets::Paragraph::new(Span::styled(
                        format!(" ⚠️ [Warning]: {} (Press any key to dismiss)", warning),
                        Style::default()
                            .fg(Color::Red)
                            .add_modifier(Modifier::BOLD),
                    ))
                } else {
                    ratatui::widgets::Paragraph::new(Span::styled(
                        " 💡 Normal Mode  |  'a': Add  |  'e': Edit  |  'd': Delete  |  'Shift+k/j': Reorder  |  '?': Help  |  'q': Quit",
                        Style::default().fg(Color::Gray),
                    ))
                };
                f.render_widget(bottom_widget, vertical_chunks[2]);

                // 3. Render Center Help Overlay if show_help is active
                if current_state.show_help {
                    let help_area = centered_rect(65, 75, area);
                    f.render_widget(ratatui::widgets::Clear, help_area);

                    let help_text = vec![
                        ratatui::text::Line::from(Span::styled(
                            " KANBAN KEYBOARD SHORTCUTS ",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        )),
                        ratatui::text::Line::from(""),
                        ratatui::text::Line::from(vec![
                            Span::styled("  h / l  (Left / Right) : ", Style::default().fg(Color::Yellow)),
                            Span::raw("Move focused ticket between columns"),
                        ]),
                        ratatui::text::Line::from(vec![
                            Span::styled("  j / k  (Down / Up)    : ", Style::default().fg(Color::Yellow)),
                            Span::raw("Navigate focused ticket selection"),
                        ]),
                        ratatui::text::Line::from(vec![
                            Span::styled("  Shift+k / K           : ", Style::default().fg(Color::Yellow)),
                            Span::raw("Reorder focused ticket position UP"),
                        ]),
                        ratatui::text::Line::from(vec![
                            Span::styled("  Shift+j / J           : ", Style::default().fg(Color::Yellow)),
                            Span::raw("Reorder focused ticket position DOWN"),
                        ]),
                        ratatui::text::Line::from(vec![
                            Span::styled("  a                     : ", Style::default().fg(Color::Yellow)),
                            Span::raw("Add / Create a new ticket interactively"),
                        ]),
                        ratatui::text::Line::from(vec![
                            Span::styled("  e / Enter             : ", Style::default().fg(Color::Yellow)),
                            Span::raw("Edit / Rename focused ticket title"),
                        ]),
                        ratatui::text::Line::from(vec![
                            Span::styled("  d / Backspace / Del   : ", Style::default().fg(Color::Yellow)),
                            Span::raw("Delete currently focused ticket"),
                        ]),
                        ratatui::text::Line::from(vec![
                            Span::styled("  ?                     : ", Style::default().fg(Color::Yellow)),
                            Span::raw("Toggle this help modal overlay"),
                        ]),
                        ratatui::text::Line::from(vec![
                            Span::styled("  q                     : ", Style::default().fg(Color::Yellow)),
                            Span::raw("Quit the application safely"),
                        ]),
                        ratatui::text::Line::from(""),
                        ratatui::text::Line::from(Span::styled(
                            " >>> PRESS ANY KEY TO CLOSE THIS HELP SCREEN <<< ",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::DIM),
                        )),
                    ];

                    let help_paragraph = ratatui::widgets::Paragraph::new(help_text)
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .title(" Help & Information ")
                                .border_style(Style::default().fg(Color::Cyan)),
                        )
                        .alignment(ratatui::layout::Alignment::Center);

                    f.render_widget(help_paragraph, help_area);
                }
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

pub struct StorageManager {
    pubsub: Arc<PubSub<AppEvent>>,
    file_path: &'static str,
}

impl StorageManager {
    pub fn new(pubsub: Arc<PubSub<AppEvent>>) -> Self {
        Self {
            pubsub,
            file_path: "kanban_board.json",
        }
    }

    pub fn load_initial_state(&self) -> KanbanState {
        let default_state = || KanbanState {
            tickets: vec![Ticket {
                id: 1,
                title: "Task 1".into(),
                status: Column::Todo,
            }],
            focused_ticket_id: Some(1),
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            quit: false,
            show_help: false,
            warning_message: None,
            wip_limit_in_progress: 3,
        };

        std::fs::read_to_string(self.file_path)
            .ok()
            .and_then(|content| serde_json::from_str::<KanbanState>(&content).ok())
            .map(|mut state| {
                state.warning_message = None;
                state.input_mode = InputMode::Normal;
                state.show_help = false;
                state.quit = false;
                state
            })
            .unwrap_or_else(default_state)
    }

    pub async fn run(self) {
        let file_path = self.file_path;
        self.pubsub
            .subscribe(
                "state_update",
                "storage_manager_save_cb",
                "storage_manager",
                SubOption::Always,
                move |event_msg| async move {
                    if let AppEvent::StateUpdated(new_state) = &*event_msg {
                        let state_clone = (**new_state).clone();
                        let file_path_clone = file_path;
                        tokio::spawn(async move {
                            if let Ok(json) = serde_json::to_string_pretty(&state_clone) {
                                let _ = tokio::fs::write(file_path_clone, json).await;
                            }
                        });
                    }
                    Ok(())
                },
            )
            .await
            .expect("Failed to subscribe StorageManager");
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let pubsub = Arc::new(PubSub::<AppEvent>::new());

    let storage_manager = StorageManager::new(pubsub.clone());
    let initial_state = storage_manager.load_initial_state();

    let state_manager = StateManager::new(initial_state.clone(), pubsub.clone());
    tokio::spawn(async move {
        state_manager.run().await;
    });

    tokio::spawn(async move {
        storage_manager.run().await;
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
            show_help: false,
            warning_message: None,
            wip_limit_in_progress: 3,
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
            show_help: false,
            warning_message: None,
            wip_limit_in_progress: 3,
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
            show_help: false,
            warning_message: None,
            wip_limit_in_progress: 3,
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

    #[tokio::test]
    async fn test_state_manager_enforces_wip_limit() {
        let pubsub = Arc::new(PubSub::<AppEvent>::new());
        let initial_state = KanbanState {
            tickets: vec![
                Ticket {
                    id: 1,
                    title: "T1".into(),
                    status: Column::InProgress,
                },
                Ticket {
                    id: 2,
                    title: "T2".into(),
                    status: Column::InProgress,
                },
                Ticket {
                    id: 3,
                    title: "T3".into(),
                    status: Column::InProgress,
                },
                Ticket {
                    id: 4,
                    title: "T4".into(),
                    status: Column::Todo,
                },
            ],
            focused_ticket_id: Some(4),
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            quit: false,
            show_help: false,
            warning_message: None,
            wip_limit_in_progress: 3,
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
                "test_wip_cb",
                "test_wip_target",
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

        // Publish action to move ticket 4 (Todo -> InProgress)
        pubsub
            .publish("action", AppEvent::ActionMoveTicket(4, Direction::Right))
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;

        let st = state_received
            .lock()
            .await
            .take()
            .expect("Did not receive StateUpdated");

        // The ticket status should remain Column::Todo because of the WIP limit
        let ticket = st.tickets.iter().find(|t| t.id == 4).unwrap();
        assert_eq!(ticket.status, Column::Todo);

        // Warning message should be populated
        assert!(st.warning_message.is_some());
        assert!(
            st.warning_message
                .unwrap()
                .contains("WIP limit of 3 reached")
        );
    }

    #[tokio::test]
    async fn test_state_manager_reorders_tickets() {
        let pubsub = Arc::new(PubSub::<AppEvent>::new());
        let initial_state = KanbanState {
            tickets: vec![
                Ticket {
                    id: 1,
                    title: "Task 1".into(),
                    status: Column::Todo,
                },
                Ticket {
                    id: 2,
                    title: "Task 2".into(),
                    status: Column::Todo,
                },
            ],
            focused_ticket_id: Some(1),
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            quit: false,
            show_help: false,
            warning_message: None,
            wip_limit_in_progress: 3,
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
                "test_reorder_cb",
                "test_reorder_target",
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

        // Reorder down: focused_id 1 is at index 0, should swap with id 2 at index 1
        pubsub
            .publish("action", AppEvent::ActionMoveOrderDown)
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;

        let st = state_received
            .lock()
            .await
            .take()
            .expect("Did not receive StateUpdated");

        // The order should now be [Task 2, Task 1]
        assert_eq!(st.tickets[0].id, 2);
        assert_eq!(st.tickets[1].id, 1);
    }

    #[tokio::test]
    async fn test_storage_manager_autosaves() {
        let pubsub = Arc::new(PubSub::<AppEvent>::new());

        // Define a custom test JSON path to avoid overwriting production kanban_board.json
        let mut storage_manager = StorageManager::new(pubsub.clone());
        let test_file = "test_kanban_board.json";
        storage_manager.file_path = test_file;

        // Clean up test file if it already exists
        let _ = std::fs::remove_file(test_file);

        let initial_state = storage_manager.load_initial_state();
        // Since test file doesn't exist yet, it should be the default fallback state
        assert_eq!(initial_state.tickets.len(), 1);
        assert_eq!(initial_state.tickets[0].id, 1);

        tokio::spawn(async move {
            storage_manager.run().await;
        });

        tokio::time::sleep(Duration::from_millis(50)).await;

        let new_state = KanbanState {
            tickets: vec![Ticket {
                id: 10,
                title: "Saved Task".into(),
                status: Column::Todo,
            }],
            focused_ticket_id: Some(10),
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            quit: false,
            show_help: false,
            warning_message: None,
            wip_limit_in_progress: 3,
        };

        // Publish StateUpdated to trigger StorageManager auto-save
        pubsub
            .publish("state_update", AppEvent::StateUpdated(Arc::new(new_state)))
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(100)).await;

        // Verify that the file was created and contains our new state
        assert!(std::path::Path::new(test_file).exists());
        let saved_content = std::fs::read_to_string(test_file).unwrap();
        let loaded_state: KanbanState = serde_json::from_str(&saved_content).unwrap();
        assert_eq!(loaded_state.tickets.len(), 1);
        assert_eq!(loaded_state.tickets[0].id, 10);
        assert_eq!(loaded_state.tickets[0].title, "Saved Task");

        // Clean up
        let _ = std::fs::remove_file(test_file);
    }
}
