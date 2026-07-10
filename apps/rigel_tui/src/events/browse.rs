use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, Mode, Pane, PromptAction};

/// Handles the key events for the browse mode.
pub(super) async fn handle_browse(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Tab => app.toggle_focus(),
        KeyCode::Down | KeyCode::Char('j') => move_cursor(app, 1),
        KeyCode::Up | KeyCode::Char('k') => move_cursor(app, -1),
        KeyCode::Enter => enter_selected(app).await?,
        KeyCode::Backspace => go_up(app).await?,
        KeyCode::Right => start_upload(app).await?,
        KeyCode::Left => start_download(app).await?,
        KeyCode::Char('m') => start_mkdir_prompt(app),
        KeyCode::Char('r') => start_rename_prompt(app),
        KeyCode::Char('d') => delete_selected(app).await?,
        KeyCode::Char('e') => start_edit(app).await?,
        _ => {}
    }
    Ok(())
}

fn move_cursor(app: &mut App, delta: i32) {
    let len = match app.focus {
        Pane::Local => app
            .session
            .as_ref()
            .map(|s| s.local_entries.len())
            .unwrap_or(0),
        Pane::Remote => app
            .session
            .as_ref()
            .map(|s| s.remote_entries.len())
            .unwrap_or(0),
    };
    if len == 0 {
        return;
    }
    let state = app.focused_state();
    let current = state.selected().unwrap_or(0) as i32;
    let next = (current + delta).clamp(0, len as i32 - 1);
    state.select(Some(next as usize));
}

async fn enter_selected(app: &mut App) -> anyhow::Result<()> {
    let focus = app.focus;
    let idx = app.focused_state().selected().unwrap_or(0);
    let Some(session) = app.session.as_mut() else {
        return Ok(());
    };

    match focus {
        Pane::Local => {
            if let Some(entry) = session.local_entries.get(idx).cloned()
                && entry.is_dir
            {
                session.enter_local_dir(&entry.name)?;
                app.local_state.select(Some(0));
            }
        }
        Pane::Remote => {
            if let Some(entry) = session.remote_entries.get(idx).cloned()
                && entry.is_dir
            {
                session.enter_remote_dir(&entry.name).await?;
                app.remote_state.select(Some(0));
            }
        }
    }
    Ok(())
}

async fn go_up(app: &mut App) -> anyhow::Result<()> {
    let focus = app.focus;
    let Some(session) = app.session.as_mut() else {
        return Ok(());
    };
    match focus {
        Pane::Local => {
            session.local_up()?;
            app.local_state.select(Some(0));
        }
        Pane::Remote => {
            session.remote_up().await?;
            app.remote_state.select(Some(0));
        }
    }
    Ok(())
}

/// Upload the selected local file. The transfer will runs in the background.
async fn start_upload(app: &mut App) -> anyhow::Result<()> {
    if app.focus != Pane::Local {
        return Ok(());
    }
    let idx = app.local_state.selected().unwrap_or(0);
    let Some(session) = app.session.as_mut() else {
        return Ok(());
    };
    let Some(entry) = session.local_entries.get(idx).cloned() else {
        return Ok(());
    };
    if entry.is_dir {
        return Ok(()); // TODO: recursive directory upload
    }

    session.queue_upload(&entry.name);
    // TODO: the remote pane will not show the file until a manual refresh
    // should auto refresh when finished
    Ok(())
}

async fn start_download(app: &mut App) -> anyhow::Result<()> {
    if app.focus != Pane::Remote {
        return Ok(());
    }
    let idx = app.remote_state.selected().unwrap_or(0);
    let Some(session) = app.session.as_mut() else {
        return Ok(());
    };
    let Some(entry) = session.remote_entries.get(idx).cloned() else {
        return Ok(());
    };
    if entry.is_dir {
        return Ok(()); // TODO: recursive directory download
    }

    session.queue_download(&entry.name);
    Ok(())
}

fn start_mkdir_prompt(app: &mut App) {
    if app.focus == Pane::Remote {
        app.mode = Mode::TextPrompt {
            title: "new remote directory name".to_string(),
            input: String::new(),
            action: PromptAction::MkdirRemote,
        };
    }
}

fn start_rename_prompt(app: &mut App) {
    if app.focus != Pane::Remote {
        return;
    }
    let idx = app.remote_state.selected().unwrap_or(0);
    let Some(session) = app.session.as_ref() else {
        return;
    };
    let Some(entry) = session.remote_entries.get(idx) else {
        return;
    };
    app.mode = Mode::TextPrompt {
        title: format!("rename '{}' to", entry.name),
        input: entry.name.clone(),
        action: PromptAction::RenameRemote {
            old_name: entry.name.clone(),
        },
    };
}

async fn delete_selected(app: &mut App) -> anyhow::Result<()> {
    let focus = app.focus;
    if focus != Pane::Remote {
        return Ok(()); // TODO: local delete
    }
    let idx = app.remote_state.selected().unwrap_or(0);
    let Some(session) = app.session.as_mut() else {
        return Ok(());
    };
    let Some(entry) = session.remote_entries.get(idx).cloned() else {
        return Ok(());
    };

    let result = if entry.is_dir {
        session.remote_delete_dir(&entry.name).await
    } else {
        session.remote_delete_file(&entry.name).await
    };
    if let Err(e) = result {
        app.mode = Mode::Message(format!("delete failed: {e}"));
    }
    Ok(())
}

/// Start a edit of a remote file:
///
/// - Download the selected remote file to a temp path
/// - main.rs loop handles it next
async fn start_edit(app: &mut App) -> anyhow::Result<()> {
    if app.focus != Pane::Remote {
        return Ok(());
    }
    let idx = app.remote_state.selected().unwrap_or(0);
    let Some(session) = app.session.as_ref() else {
        return Ok(());
    };
    let Some(entry) = session.remote_entries.get(idx).cloned() else {
        return Ok(());
    };
    if entry.is_dir {
        return Ok(());
    }

    match session.download_for_edit(&entry.name).await {
        Ok(temp_path) => {
            app.pending_edit = Some(crate::app::PendingEdit {
                temp_path,
                remote_name: entry.name,
            });
        }
        Err(e) => app.mode = Mode::Message(format!("edit download failed: {e}")),
    }
    Ok(())
}
