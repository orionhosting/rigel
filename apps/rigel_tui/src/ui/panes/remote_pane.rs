use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::ListItem;

use crate::app::{App, Pane};

use super::shared::styled_list;

/// Render the remote directory pane for the current session.
pub fn draw_remote_pane(frame: &mut Frame, app: &mut App, area: Rect) {
    let Some(session) = app.session.as_ref() else {
        return;
    };

    let title = format!("remote: {}", session.remote_cwd);
    let items: Vec<ListItem> = session
        .remote_entries
        .iter()
        .map(|entry| {
            ListItem::new(if entry.is_dir {
                format!("{}/", entry.name)
            } else {
                entry.name.clone()
            })
        })
        .collect();

    let focused = app.focus == Pane::Remote;
    let list = styled_list(items, &title, focused);
    frame.render_stateful_widget(list, area, &mut app.remote_state);
}
