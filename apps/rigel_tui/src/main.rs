mod app;
mod events;
mod ui;

use std::io;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self as term_event, Event};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use app::{App, Mode, PendingEdit};

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::new()?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let result = run(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

async fn run<B>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()>
where
    B: ratatui::backend::Backend,
    B::Error: std::error::Error + Send + Sync + 'static,
{
    loop {
        terminal.draw(|frame| ui::draw(frame, app))?;

        // Poll terminal events
        if term_event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = term_event::read()?
            && key.kind == term_event::KeyEventKind::Press
        {
            events::handle_key(app, key).await?;
        }

        if let Some(edit) = app.pending_edit.take() {
            run_editor(app, edit).await?;
            terminal.clear()?;
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

/// Disable raw mode + open the editor on the temp
/// file, then upload the result back over SFTP.
async fn run_editor(app: &mut App, edit: PendingEdit) -> Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
    let status = std::process::Command::new(&editor)
        .arg(&edit.temp_path)
        .status();

    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;

    match status {
        Ok(s) if s.success() => {
            if let Some(session) = app.session.as_mut()
                && let Err(e) = session
                    .upload_edited(&edit.temp_path, &edit.remote_name)
                    .await
            {
                app.mode = Mode::Message(format!("upload after edit failed: {e}"));
            }
        }
        Ok(_) => {
            app.mode = Mode::Message(format!("{editor} exited with an error; not uploading"));
        }
        Err(e) => {
            app.mode = Mode::Message(format!("could not launch {editor}: {e}"));
        }
    }
    Ok(())
}
