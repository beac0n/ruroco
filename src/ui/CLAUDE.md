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
