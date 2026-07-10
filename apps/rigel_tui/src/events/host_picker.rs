use crossterm::event::{KeyCode, KeyEvent};
use rigel_client::{
    Auth, Session,
    hosts::{SavedAuth, SavedHost},
};

use crate::app::{App, Mode, PromptAction};

/// Handles the host picker keys.
pub(super) async fn handle_host_picker(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    let Mode::HostPicker { cursor } = &mut app.mode else {
        return Ok(());
    };
    match key.code {
        KeyCode::Down | KeyCode::Char('j') => {
            *cursor = (*cursor + 1).min(app.saved_hosts.len());
        }
        KeyCode::Up | KeyCode::Char('k') => {
            *cursor = cursor.saturating_sub(1);
        }
        KeyCode::Char('a') => {
            app.mode = Mode::AddHost {
                input: String::new(),
            };
        }
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Enter => {
            let cursor = *cursor;
            if cursor < app.saved_hosts.len() {
                let host = app.saved_hosts[cursor].clone();
                connect_saved(app, &host).await;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Connect to a saved host, and go to browse mode if it connect successfully.
async fn connect_saved(app: &mut App, host: &SavedHost) {
    let auth: Auth = (&host.auth).into();
    let start_remote = host.start_path.clone().unwrap_or_else(|| "/".to_string());
    let start_local = std::env::current_dir().unwrap_or_else(|_| ".".into());

    match Session::connect(
        &host.host,
        host.port,
        &host.username,
        &auth,
        &start_remote,
        start_local,
    )
    .await
    {
        Ok(session) => {
            app.session = Some(session);
            app.mode = Mode::Browse;
        }
        Err(e) => {
            app.mode = Mode::Message(format!("connect failed: {e}"));
        }
    }
}

/// Add an host.
///
/// It parses the line as `label,host,port,user,password`.
/// TODO
pub(super) async fn handle_add_host(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    let Mode::AddHost { input } = &mut app.mode else {
        return Ok(());
    };
    match key.code {
        KeyCode::Esc => app.mode = Mode::HostPicker { cursor: 0 },
        KeyCode::Enter => {
            let parts: Vec<&str> = input.split(',').collect();
            if parts.len() == 5 {
                let host = SavedHost {
                    label: parts[0].to_string(),
                    host: parts[1].to_string(),
                    port: parts[2].parse().unwrap_or(22),
                    username: parts[3].to_string(),
                    auth: SavedAuth::Password(parts[4].to_string()),
                    start_path: None,
                };
                app.saved_hosts = rigel_client::hosts::upsert_host(host)?;
                app.mode = Mode::HostPicker { cursor: 0 };
            } else {
                app.mode = Mode::Message("format: label,host,port,user,password".to_string());
            }
        }
        KeyCode::Backspace => {
            input.pop();
        }
        KeyCode::Char(c) => input.push(c),
        _ => {}
    }
    Ok(())
}

pub(super) async fn handle_text_prompt(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    let Mode::TextPrompt { input, action, .. } = &mut app.mode else {
        return Ok(());
    };
    match key.code {
        KeyCode::Esc => app.mode = Mode::Browse,
        KeyCode::Backspace => {
            input.pop();
        }
        KeyCode::Char(c) => input.push(c),
        KeyCode::Enter => {
            let new_name = input.clone();
            let action_taken = std::mem::replace(action, PromptAction::MkdirRemote);
            app.mode = Mode::Browse;

            if let Some(session) = app.session.as_mut() {
                let result = match action_taken {
                    PromptAction::MkdirRemote => session.remote_mkdir(&new_name).await,
                    PromptAction::RenameRemote { old_name } => {
                        session.remote_rename(&old_name, &new_name).await
                    }
                };
                if let Err(e) = result {
                    app.mode = Mode::Message(format!("error: {e}"));
                }
            }
        }
        _ => {}
    }
    Ok(())
}
