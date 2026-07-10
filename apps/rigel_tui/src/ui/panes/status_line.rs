use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;

use crate::app::{App, Mode};

/// Render the footer status line with current mode and key hints.
pub fn draw_status_line(frame: &mut Frame, app: &App, area: Rect) {
    let text = match &app.mode {
        Mode::Message(msg) => msg.clone(),
        Mode::TextPrompt { title, input, .. } => format!("{title}: {input}"),
        _ => "TAB: switch pane | ENTER: open | BACKSPACE: up | ←/→: transfer | M: mkdir | R: rename | D: delete | E: edit | Q: quit".to_string(),
    };

    frame.render_widget(Line::from(text), area);
}
