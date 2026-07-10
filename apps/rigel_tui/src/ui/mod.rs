mod host_picker;
mod panes;
mod util;

use ratatui::Frame;

use crate::app::{App, Mode};

/// Draw the app UI.
pub fn draw(frame: &mut Frame, app: &mut App) {
    match &app.mode {
        Mode::HostPicker { cursor } => host_picker::draw_picker(frame, app, *cursor),
        Mode::AddHost { input } => host_picker::draw_add_host(frame, input),
        Mode::Browse | Mode::TextPrompt { .. } | Mode::Message(_) => {
            panes::draw_browse(frame, app);
        }
    }
}
