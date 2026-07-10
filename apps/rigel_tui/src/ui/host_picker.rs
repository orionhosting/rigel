use ratatui::Frame;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use crate::app::App;
use crate::ui::util::centered;

/// Render the host picker with saved hosts and an action to add a new host.
pub fn draw_picker(frame: &mut Frame, app: &App, cursor: usize) {
    let area = centered(frame.area(), 60, 60);

    let mut items: Vec<ListItem> = app
        .saved_hosts
        .iter()
        .map(|h| ListItem::new(format!("{}  ({}@{})", h.label, h.username, h.host)))
        .collect();
    items.push(ListItem::new("+ add new host").style(Style::default().fg(Color::Green)));

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Saved Hosts | ENTER: connect | A: add | Q: quit"),
        )
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Cyan),
        );

    let mut state = ratatui::widgets::ListState::default();
    state.select(Some(cursor));

    frame.render_stateful_widget(list, area, &mut state);
}

/// Render the input form to add a new host.
pub fn draw_add_host(frame: &mut Frame, input: &str) {
    let area = centered(frame.area(), 60, 20);
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Add Host | format: label,host,port,user,password | Enter to save, Esc to cancel");
    let paragraph = Paragraph::new(input).block(block);
    frame.render_widget(paragraph, area);
}
