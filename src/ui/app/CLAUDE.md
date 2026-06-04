# src/ui/app/

`RurocoApp` (impl `eframe::App`) is the frame-loop root. It holds `active_tab: Tab`, one state
struct per tab (`dashboard`, `create`, `execute`), and `commands_list: CommandsList`.

- `dashboard_state.rs` / `execute_state.rs`: per-tab UI state. The dashboard's AES key is loaded
  from / saved to Android SharedPrefs on change (desktop returns empty).
- Saved commands persist to `<conf_dir>/commands_list.toml` via an atomic write on every
  add/remove/set (no batching); a parse error on load starts empty but preserves the file.

State persists across frames and across tab switches by design (multi-step workflows), so don't
assume a tab is reset when re-entered.
