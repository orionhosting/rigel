use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;

/// Render the recent transfer activity for the current session.
pub fn draw_transfer_queue(frame: &mut Frame, app: &App, area: Rect) {
    let Some(session) = app.session.as_ref() else {
        return;
    };

    let lines: Vec<Line> = session
        .transfers
        .iter()
        .rev()
        .take(2)
        .map(|job| {
            let (done, total) = job.progress.snapshot();
            let pct = (done * 100).checked_div(total).unwrap_or(0);
            let arrow = match job.direction {
                rigel_client::transfer::TransferDirection::Upload => "->",
                rigel_client::transfer::TransferDirection::Download => "<-",
            };
            Line::from(format!(
                "[{:?}] {} {} {}%",
                job.status.get(),
                arrow,
                job.remote_path,
                pct
            ))
        })
        .collect();

    let block = Block::default().borders(Borders::ALL).title("transfers");
    frame.render_widget(Paragraph::new(lines).block(block), area);
}
