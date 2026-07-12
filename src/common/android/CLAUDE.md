# src/common/android/

JNI bridge to Android platform APIs, only compiled under `cfg(target_os = "android")` (feature
`android-build`). Exposes `AndroidClipboard`, `AndroidKeyboard`, `AndroidPrefs`, `AndroidStatusBar`,
and `AndroidUtil` (shared JNI call/string helpers in `jni_util.rs`).

Used by the GUI (`src/ui/`) for clipboard, soft-keyboard, status-bar inset, and persisting the AES
key in SharedPreferences (desktop has no equivalent - it never persists the key, re-entered every
launch instead). `AndroidPrefs` stores it there in plaintext, not via the Android Keystore; see
SECURITY.md's Key lifecycle section for why this is accepted rather than fixed. JNI calls attach
the current thread via the VM from `ndk_context`; keep method signatures in sync with the
Kotlin/Java side.

Gotcha: the app sandbox has no writable `/tmp` and no `$TMPDIR`, so `std::env::temp_dir()` and
anything built on it (`tempfile::NamedTempFile::new()`, `tempfile::tempdir()` with no explicit
directory) silently resolves to an unwritable path and fails on a real device - this has no
Android-specific fallback in Rust's std, unlike macOS/iOS. Any client/UI code that also compiles
for Android must write through `get_conf_dir()` (`client/config/mod.rs`, JNI `getFilesDir()` on
Android) or `NamedTempFile::new_in(<that dir>)`, never the bare platform temp dir.
