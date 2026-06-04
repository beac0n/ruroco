# src/common/android/

JNI bridge to Android platform APIs, only compiled under `cfg(target_os = "android")` (feature
`android-build`). Exposes `AndroidClipboard`, `AndroidKeyboard`, `AndroidPrefs`, `AndroidStatusBar`,
and `AndroidUtil` (shared JNI call/string helpers in `jni_util.rs`).

Used by the GUI (`src/ui/`) for clipboard, soft-keyboard, status-bar inset, and persisting the AES
key in SharedPreferences (desktop has no equivalent and no-ops these). JNI calls attach the current
thread via the VM from `ndk_context`; keep method signatures in sync with the Kotlin/Java side.
