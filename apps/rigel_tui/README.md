# Rigel TUI

The terminal UI app.

## Lifecycle

Basically how it works:

- `App` contains the session & UI state
- `ui/` uses the app state to render on the terminal
- `events/` exposes a single `handle_key` function which mutates App. (`events/` do not touch the UI, only the app)
