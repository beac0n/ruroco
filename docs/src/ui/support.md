# Support Types and Widgets

This chapter covers the GUI's shared building blocks: `command_data.rs` (the command model and
CLI string conversions), `saved_command_list.rs` (`CommandsList`, persistence), `widgets.rs`
(reusable egui helpers), and `colors.rs` (the palette).

## `src/ui/command_data.rs`

```rust
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct CommandData {
    pub(crate) address: String,
    pub(crate) command: String,
    pub(crate) permissive: bool,
    pub(crate) ip: String,
    pub(crate) ipv4: bool,
    pub(crate) ipv6: bool,
    #[serde(skip)]
    pub(crate) name: String,
}

pub(crate) fn data_to_command(data: &CommandData, key: Option<String>) -> String
pub(crate) fn command_to_data(input: &str) -> CommandData
pub(crate) fn add_command_name(mut data: CommandData) -> CommandData
```

`CommandData` is the in-memory model of one saved command. `name` is a derived display label and is
`#[serde(skip)]`, so it is recomputed (via `add_command_name`) after every load rather than stored.

- `data_to_command`: serializes a `CommandData` into a `send` CLI string, emitting only non-empty /
  true fields (`--address`, `--command`, `--ip`, `--ipv4`, `--ipv6`, `--permissive`). A `Some(key)`
  appends `--key <k>` (used when actually sending; persisted/display forms pass `None`). Trailing
  whitespace is trimmed.
- `command_to_data`: the inverse parser. Tokenizes on whitespace; recognizes the flags above (taking
  the next token as the value for `--address` / `--command` / `--ip`) and ignores unknown tokens.
  Always finishes by calling `add_command_name`. Gotcha: it does not validate, so malformed input
  silently yields empty/default fields.
- `add_command_name`: builds `name` as `"{command}@{address}"` plus ` permissive` / ` ipv4` /
  ` ipv6` suffixes for whichever flags are set, then stores it on the struct.

## `src/ui/saved_command_list.rs`

```rust
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub(crate) struct CommandsList {
    list: Vec<CommandData>,
    #[serde(skip)]
    path: PathBuf,
}

impl fmt::Display for CommandsList { ... }

impl CommandsList {
    pub(crate) fn create(cfg_dir: &Path) -> CommandsList
    pub(crate) fn get(&self) -> &[CommandData]
    pub(crate) fn add(&mut self, cmd: CommandData)
    pub(crate) fn set(&mut self, list: Vec<CommandData>)
    pub(crate) fn remove(&mut self, cmd: &CommandData)
    fn sort(&mut self)                 // by (command, address)
    fn read_raw_from_path(path: &Path) -> String
    fn save(&self)
}
```

The persistent store of saved commands, backed by `<cfg_dir>/commands_list.toml`.

- `Display`: renders the list as newline-separated `send ...` strings (`data_to_command(c, None)`).
  This is what feeds the Dashboard's editable config text.
- `create`: resolves `<cfg_dir>/commands_list.toml`, reads it (missing/unreadable file -> empty
  string), and parses it. Parsing tries the current TOML schema first; on failure it falls back to a
  **legacy** schema (`list: Vec<String>` of CLI invocations), mapping each string through
  `command_to_data`. If both fail and the raw text was non-empty, it logs a parse error and **starts
  empty while leaving the file on disk untouched** (no overwrite until the next mutation). It then
  sets `path`, recomputes every `name` via `add_command_name`, and sorts.
- `get`: borrow the current slice.
- `add` / `set` / `remove`: mutate the list and immediately call `save()`. `add` and `set` also
  `sort()` first (`remove` preserves order). There is no batching: every mutation writes to disk.
- `save`: serializes to TOML and writes via `write_atomic` (temp file + fsync + rename). Both
  serialization and write errors are logged, never panic, so a bad path degrades gracefully.

Sorting is by `(command, address)` lexicographically.

## `src/ui/tabs/widgets.rs`

```rust
pub(crate) struct Widgets<'a> { ui: &'a mut egui::Ui }

impl<'a> Widgets<'a> {
    pub(crate) fn new(ui: &'a mut egui::Ui) -> Self
    pub(crate) fn bordered(color: egui::Color32, inner_margin: f32) -> egui::Frame   // associated, no self
    pub(crate) fn icon_button(&mut self, color: egui::Color32, label: &str) -> egui::Response
    pub(crate) fn equal_buttons(&mut self, labels: &[&str]) -> Vec<bool>
    pub(crate) fn copy_text(&self, text: &str)
    pub(crate) fn paste_button(&mut self, dashboard: &mut DashboardState, target: PasteTarget)
}
```

A thin wrapper around a borrowed `egui::Ui`, holding shared layout helpers.

- `bordered`: an associated fn (no `Self` instance) returning an `egui::Frame` with a 2px stroke in
  `color`, 5px corner radius, and the given inner margin. Used both for icon buttons and the Execute
  tab's status box.
- `icon_button`: a fixed `46x46` button inside a `bordered(color, 1.0)` frame; returns the button's
  `Response`. The closure body always runs, so the `expect("frame body always runs")` cannot fire.
- `equal_buttons`: lays out `labels.len()` buttons of equal width (accounting for 8px gaps) in a
  horizontal row, each `50.0` tall, and returns a `Vec<bool>` of click states indexed to match the
  input labels.
- `copy_text`: clipboard copy. On Android calls `AndroidClipboard::set_text` (logging errors); on
  desktop `self.ui.ctx().copy_text(...)`.
- `paste_button`: clipboard paste. On Android calls `AndroidClipboard::get_text` and applies the
  text immediately to the key or config field. On desktop it instead records
  `dashboard.paste_target = Some(target)` and fires `ViewportCommand::RequestPaste`; the actual text
  arrives as a `Paste` event handled next frame in `tabs/dashboard.rs`.

## `src/ui/colors.rs`

```rust
pub(crate) const BLUE: Color32  = Color32::from_rgb(25, 118, 210);
pub(crate) const GREEN: Color32 = Color32::from_rgb(56, 142, 60);
pub(crate) const RED: Color32   = Color32::from_rgb(211, 47, 47);
pub(crate) const GRAY: Color32  = Color32::from_rgb(204, 204, 204);
```

The four palette colors. `BLUE` is the run button, `RED` the delete button and error status,
`GREEN` the success status, `GRAY` the not-yet-run / neutral status (see `ExecuteState::color_for`).
