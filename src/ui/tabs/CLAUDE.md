# src/ui/tabs/

Three tabs, each a `render(state, commands_list, ui)` function dispatched from `app_frame.rs`:

- `dashboard.rs`: config + key management, delegating to sub-views `dashboard_config.rs` and
  `dashboard_key.rs`.
- `create.rs`: form to build a `CommandData`; "Add" appends to `commands_list` (which auto-saves).
- `execute.rs`: list of saved commands with run/delete; running calls the client's send path
  **synchronously** (no async, so a slow send blocks the UI thread) and colors the row by status.

Shared styling helpers live in `widgets.rs` (`bordered`, `icon_button`, `equal_buttons`,
clipboard buttons that branch desktop vs Android).

To add a tab: write `tabs/<name>.rs` with a `render(&mut <State>, &mut CommandsList, &mut egui::Ui)`
fn, declare it in `tabs/mod.rs`, add a `Tab` variant and a state field on `RurocoApp` (`app/mod.rs`),
and add a `selectable_value` + match arm in `app_frame.rs`.
