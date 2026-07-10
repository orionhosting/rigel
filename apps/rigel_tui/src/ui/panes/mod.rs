use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};

use crate::app::App;

mod local_pane;
mod remote_pane;
mod shared;
mod status_line;
mod transfer_queue;

pub use local_pane::draw_local_pane;
pub use remote_pane::draw_remote_pane;
pub use status_line::draw_status_line;
pub use transfer_queue::draw_transfer_queue;

/// Draws the browse layout with local/remote panes, transfer and status helpers.
pub fn draw_browse(frame: &mut Frame, app: &mut App) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),    // panes
            Constraint::Length(3), // transfer queue
            Constraint::Length(1), // status/help line
        ])
        .split(frame.area());

    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(root[0]);

    draw_local_pane(frame, app, panes[0]);
    draw_remote_pane(frame, app, panes[1]);
    draw_transfer_queue(frame, app, root[1]);
    draw_status_line(frame, app, root[2]);
}
