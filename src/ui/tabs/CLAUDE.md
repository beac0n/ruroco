# src/ui/tabs/

Three tabs, each a `render(state, commands_list, ui)` function dispatched from `app_frame.rs`:

- `dashboard.rs`: config + key management, delegating to sub-views `dashboard_config.rs` and
  `dashboard_key.rs`.
- `create.rs`: form to build a `CommandData`; "Add" appends to `commands_list` (which auto-saves).
- `execute.rs`: list of saved commands with run/delete; running writes the in-memory key to a
  `NamedTempFile` created inside `get_conf_dir()` (auto-deleted on drop, since `SendCommand` has no
  `key` field, only `key_file`) - NOT `NamedTempFile::new()`, which resolves to the platform temp
  dir and would silently fail every send on Android (no writable `/tmp` in the app sandbox; see
  `get_conf_dir_android` in `client/config/mod.rs`) - builds a `SendCommand` directly from
  `CommandData`, and calls `Sender::create(..).send()` **synchronously** (no async, so a slow send
  blocks the UI thread), coloring the row by status.

Shared styling helpers live in `widgets.rs` (`bordered`, `icon_button`, `equal_buttons`,
clipboard buttons that branch desktop vs Android).

To add a tab: write `tabs/<name>.rs` with a `render(&mut <State>, &mut CommandsList, &mut egui::Ui)`
fn, declare it in `tabs/mod.rs`, add a `Tab` variant and a state field on `RurocoApp` (`app/mod.rs`),
and add a `selectable_value` + match arm in `app_frame.rs`.
