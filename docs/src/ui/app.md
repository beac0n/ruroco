# App Root and State

This chapter documents the `app/` module (`mod.rs`, `dashboard_state.rs`, `execute_state.rs`) and
the frame dispatcher `app_frame.rs`. Together they define the root `RurocoApp`, the `Tab` enum, the
per-tab state structs, and how state persists across frames.

## `src/ui/app/mod.rs`

Declares the `dashboard_state` and `execute_state` submodules and re-exports their public types:

```rust
pub(crate) use dashboard_state::{DashboardState, PasteTarget};
pub(crate) use execute_state::{ExecuteState, Status, StatusKey};
```

### `Tab`

```rust
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Tab { Dashboard, Create, Execute }
```

Identifies the active tab. `Copy + PartialEq` so it can be used directly with
`egui::Ui::selectable_value`.

### `CreateForm`

```rust
pub(crate) struct CreateForm {
    pub(crate) address: String,
    pub(crate) command: String,
    pub(crate) ip: String,
    pub(crate) permissive: bool,
    pub(crate) ipv4: bool,
    pub(crate) ipv6: bool,
}
```

The edit buffers backing the Create tab's form fields. Defined here (alongside `RurocoApp`) rather
than in a `*_state.rs` file because it has no platform behavior.

### `RurocoApp`

```rust
pub(crate) struct RurocoApp {
    pub(crate) commands_list: CommandsList,
    pub(crate) active_tab: Tab,
    pub(crate) status_bar_dp: f32,
    pub(crate) dashboard: DashboardState,
    pub(crate) create: CreateForm,
    pub(crate) execute: ExecuteState,
}

impl RurocoApp {
    pub(crate) fn new(conf_dir: &Path) -> anyhow::Result<Self>
    pub(crate) fn new_with_status_bar(conf_dir: &Path, status_bar_dp: f32) -> anyhow::Result<Self>
}
```

The single source of truth for all GUI state. `new` delegates to `new_with_status_bar(conf_dir, 0.0)`
(desktop has no status bar inset). Construction:

- Loads `CommandsList::create(conf_dir)` (reads `<conf_dir>/commands_list.toml`).
- Seeds `dashboard.config_text` from `commands_list.to_string()` so the Dashboard's editable config
  text starts as the serialized command list.
- `active_tab` starts at `Tab::Dashboard`.
- `dashboard.key` is seeded from `DashboardState::load_persisted_key()` (Android SharedPrefs, empty
  on desktop).
- `create.command` defaults to `config::DEFAULT_COMMAND`; the other create fields start empty/false.
- `execute.status` starts as an empty `HashMap`.

Because the struct lives for the whole process, all of this state survives frame redraws and tab
switches.

## `src/ui/app/dashboard_state.rs`

```rust
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum PasteTarget { Key, Config }

pub(crate) struct DashboardState {
    pub(crate) config_text: String,
    pub(crate) key: String,
    pub(crate) show_key: bool,
    pub(crate) paste_target: Option<PasteTarget>,
}

impl DashboardState {
    pub(crate) fn load_persisted_key() -> String
    pub(crate) fn save_key(&mut self, key: String)
}
```

State for the Dashboard tab.

- `config_text`: the editable multi-line text of the saved command list.
- `key`: the AES key string used when running commands.
- `show_key`: toggles the key field between password (masked) and plaintext display.
- `paste_target`: on desktop, paste is asynchronous (a `RequestPaste` viewport command is sent and
  the resulting `Paste` event arrives a frame later); this records whether that pending paste should
  land in the key or the config field. `take()`-n when the event is handled.

`load_persisted_key()`: on Android reads the `aes_key` SharedPreference via
`AndroidPrefs::get_string`; `Ok(None)` and any error both yield `String::new()` (errors are logged).
On non-Android it returns `String::new()` unconditionally.

`save_key(key)`: always sets `self.key`; on Android it additionally persists via
`AndroidPrefs::put_string("aes_key", ...)` (errors logged, non-fatal). The `KEY_PREF` constant
(`"aes_key"`) and the `error` import are Android-only.

## `src/ui/app/execute_state.rs`

```rust
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Status { Ok, Err }

#[derive(Hash, Eq, PartialEq, Clone)]
pub(crate) struct StatusKey { command, address, ip: String, ipv4, ipv6, permissive: bool }

impl From<&CommandData> for StatusKey { fn from(c: &CommandData) -> Self }

pub(crate) struct ExecuteState { pub(crate) status: HashMap<StatusKey, Status> }

impl ExecuteState {
    pub(crate) fn color_for(&self, cmd: &CommandData) -> egui::Color32
    pub(crate) fn set(&mut self, cmd: &CommandData, status: Status)
}
```

Tracks the last run outcome per command for the Execute tab.

- `StatusKey` is derived from a `CommandData`'s identifying fields (everything except `name`), so a
  command keeps its status even though `name` is recomputed/`#[serde(skip)]`. It is hashable and
  used as the `HashMap` key.
- `color_for`: returns `GREEN` for `Status::Ok`, `RED` for `Status::Err`, and `GRAY` when the
  command has no recorded status yet (never run this session).
- `set`: records a command's run result, inserting/overwriting by `StatusKey`.

The status map is in-memory only; it is not persisted, so all rows start gray on a fresh launch.

## `src/ui/app_frame.rs`

Implements the eframe entry point:

```rust
impl eframe::App for RurocoApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame)
}
```

Per-frame logic (see the [overview flowchart](overview.md)):

1. If `status_bar_dp > 0.0`, `ui.add_space(self.status_bar_dp)` to clear the Android status bar.
2. Under `cfg(all(target_os = "android", feature = "android-build"))`, call
   `AndroidKeyboard::ensure_visible(ui.ctx().egui_wants_keyboard_input())` to show/hide the soft
   keyboard, logging any error.
3. A top `egui::Panel::top("tabs")` with a horizontal row of three `selectable_value` toggles bound
   to `&mut self.active_tab`.
4. A `CentralPanel` that matches on `self.active_tab` and calls the corresponding tab `render`:
   - `Dashboard` -> `tabs::dashboard::render(&mut self.dashboard, &mut self.commands_list, ui)`
   - `Create` -> `tabs::create::render(&mut self.create, &mut self.commands_list, &mut self.dashboard.config_text, ui)`
   - `Execute` -> `tabs::execute::render(&mut self.execute, &mut self.commands_list, &self.dashboard.key, ui)`

The Create tab receives a mutable borrow of `dashboard.config_text` so that adding a command also
refreshes the Dashboard's config view; the Execute tab receives a shared borrow of `dashboard.key`
to use as the AES key when sending.
