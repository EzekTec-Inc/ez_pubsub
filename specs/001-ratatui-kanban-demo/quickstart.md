# Quickstart

To run the new Kanban Ratatui demo:

```bash
cargo run --example kanban_board
```

### Controls

| Key / Binding | Action |
| :--- | :--- |
| **`Up`** / **`k`** | Navigate selection UP |
| **`Down`** / **`j`** | Navigate selection DOWN |
| **`Left`** / **`h`** | Move ticket Left (`Done` $\to$ `In Progress` $\to$ `Todo`) |
| **`Right`** / **`l`** | Move ticket Right (`Todo` $\to$ `In Progress` $\to$ `Done`) |
| **`Shift+Up`** / **`K`** | Swap focused ticket order **UP** |
| **`Shift+Down`** / **`J`** | Swap focused ticket order **DOWN** |
| **`a`** | Create a new ticket (opens interactive bottom bar text input) |
| **`e`** / **`Enter`** | Edit / Rename focused ticket title |
| **`d`** / **`Backspace`** / **`Del`** | Delete focused ticket |
| **`?`** | Toggle Help Overlay Modal |
| **`q`** | Quit application safely |

### Advanced Features

- **Work in Progress (WIP) Limits:** The `IN PROGRESS` column has a WIP limit of `3`. Attempting to move more than 3 tickets into it will trigger a warning banner at the bottom of the screen, and the border will change color to red.
- **Persistence (Auto-Save/Load):** State is automatically and asynchronously saved to `kanban_board.json` on disk whenever any change occurs, and automatically reloaded at boot!
- **Centered Help Screen:** Pressing `?` displays a beautiful interactive help overlay card.