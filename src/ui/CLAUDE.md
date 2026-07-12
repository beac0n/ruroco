# src/ui/

egui (eframe) GUI, a thin view layer over `src/client/` (it calls the client's send logic
directly; there is no separate UI networking). Subdirs: `app/` (the `RurocoApp` root + state),
`tabs/` (views). Loose: `app_frame.rs` (window chrome + tab dispatch), `colors.rs`,
`saved_command_list.rs`, `command_data.rs`, `android.rs`.

Entry points: desktop `run_ui()` opens an eframe window and constructs `RurocoApp`; Android
`android.rs::android_main` (gated by `cfg(target_os = "android")` + feature `android-build`) runs
with the wgpu/native-activity backend and passes the status-bar inset height in.

Desktop vs Android differences are localized: clipboard and the AES key use egui/filesystem on
desktop and the `common/android` JNI bridge (SharedPreferences, soft keyboard) on Android.

`command_data.rs`'s `data_to_command`/`command_to_data` round-trip a `CommandData` through a
CLI-like text line (used by the dashboard's raw config editor and legacy `commands_list.toml`
migration); the line format is plain `split_whitespace`, no quoting, so `command` (and `address`)
must not contain whitespace - `create.rs`'s "Add Command" enforces this on the one field that could
otherwise hold it (`command`; `address`/`ip` never do by construction). `CommandData.command` is
not just a display value: `execute.rs` sends it verbatim as the real command name, so a
space would produce a wrong hash lookup on the server too, not just a UI round-trip bug. The config
editor's save button also drops blank/whitespace-only lines before parsing, so they don't become
empty entries.
