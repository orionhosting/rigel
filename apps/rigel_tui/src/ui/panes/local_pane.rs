use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::ListItem;

use crate::app::{App, Pane};

use super::shared::styled_list;

/// Render the local directory pane for the current session.
pub fn draw_local_pane(frame: &mut Frame, app: &mut App, area: Rect) {
    let Some(session) = app.session.as_ref() else {
        return;
    };

    let title = format!("local: {}", session.local_cwd.display());
    let items: Vec<ListItem> = session
        .local_entries
        .iter()
        .map(|entry| {
            ListItem::new(if entry.is_dir {
                format!("{}/", entry.name)
            } else {
                entry.name.clone()
            })
        })
        .collect();

    let focused = app.focus == Pane::Local;
    let list = styled_list(items, &title, focused);
    frame.render_stateful_widget(list, area, &mut app.local_state);
}
