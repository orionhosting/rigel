mod browse;
mod host_picker;

use anyhow::Result;
use crossterm::event::KeyEvent;

use crate::app::{App, Mode};

/// Handle a key event from the TUI.
///
/// Note that this should always returns Ok, since the TUI
/// should not be able to crash. Instead it shows an error
/// message to the user.
pub async fn handle_key(app: &mut App, key: KeyEvent) -> Result<()> {
    match &app.mode {
        Mode::HostPicker { .. } => host_picker::handle_host_picker(app, key).await?,
        Mode::AddHost { .. } => host_picker::handle_add_host(app, key).await?,
        Mode::TextPrompt { .. } => host_picker::handle_text_prompt(app, key).await?,
        Mode::Message(_) => {
            // any key = go back to browse mode
            app.mode = Mode::Browse;
        }
        Mode::Browse => browse::handle_browse(app, key).await?,
    }
    Ok(())
}
