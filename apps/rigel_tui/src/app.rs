use ratatui::widgets::ListState;
use rigel_client::{Session, hosts};

/// Which pane currently has keyboard focus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pane {
    Local,
    Remote,
}

/// The current UI state.
pub enum Mode {
    // Menu states
    HostPicker {
        cursor: usize,
    },
    AddHost {
        input: String,
    },

    // Connected states
    Browse,
    TextPrompt {
        title: String,
        input: String,
        action: PromptAction,
    },
    /// A temporary message for the user (e.g. error/confirmation).
    Message(String),
}

pub enum PromptAction {
    MkdirRemote,
    RenameRemote { old_name: String },
}

/// The TUI app.
pub struct App {
    pub session: Option<Session>,
    pub mode: Mode,
    pub focus: Pane,

    pub local_state: ListState,
    pub remote_state: ListState,
    pub saved_hosts: Vec<hosts::SavedHost>,
    pub should_quit: bool,

    pub pending_edit: Option<PendingEdit>,
}

pub struct PendingEdit {
    pub temp_path: std::path::PathBuf,
    pub remote_name: String,
}

impl App {
    pub fn new() -> anyhow::Result<Self> {
        let saved_hosts = hosts::load_hosts()?;
        let mut local_state = ListState::default();
        local_state.select(Some(0));
        let mut remote_state = ListState::default();
        remote_state.select(Some(0));

        Ok(Self {
            session: None,
            mode: Mode::HostPicker { cursor: 0 },
            focus: Pane::Local,
            local_state,
            remote_state,
            saved_hosts,
            should_quit: false,
            pending_edit: None,
        })
    }

    /// Get the state of the focused pane.
    pub fn focused_state(&mut self) -> &mut ListState {
        match self.focus {
            Pane::Local => &mut self.local_state,
            Pane::Remote => &mut self.remote_state,
        }
    }

    /// Switch between the local and remote panes.
    pub fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            Pane::Local => Pane::Remote,
            Pane::Remote => Pane::Local,
        };
    }
}
