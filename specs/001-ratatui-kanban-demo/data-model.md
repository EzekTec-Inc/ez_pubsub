# Phase 1: Data Model

## Entities

### `Ticket`
- `id: u32`
- `title: String`
- `status: Column`

### `Column` (Enum)
- `Todo`
- `InProgress`
- `Done`

### `KanbanState`
- `tickets: Vec<Ticket>`
- `focused_ticket_id: Option<u32>`

## Events (PubSub Topics)
- `ui_input`: Payload `KeyEvent`
- `action_move_ticket`: Payload `(u32, Direction)`
- `state_updated`: Payload `KanbanState`