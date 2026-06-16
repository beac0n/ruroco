use super::util::AndroidUtil;
use anyhow::Context;
use jni::objects::{JObject, JValue};
use jni::signature::RuntimeMethodSignature;
use jni::strings::JNIString;
use jni::JavaVM;
use std::sync::atomic::{AtomicBool, Ordering};

static KEYBOARD_HIDDEN: AtomicBool = AtomicBool::new(true);

pub(crate) struct AndroidKeyboard;

impl AndroidKeyboard {
    pub(crate) fn ensure_visible(want: bool) -> anyhow::Result<()> {
        if want {
            // Always call show: handles re-show after user externally dismisses the keyboard
            // while a text field remains focused (wants stays true, no transition to detect).
            Self::show()?;
            KEYBOARD_HIDDEN.store(false, Ordering::Relaxed);
        } else if !KEYBOARD_HIDDEN.load(Ordering::Relaxed) {
            // Only call hide on transition — avoids 60 JNI calls/sec while no input is focused.
            Self::hide()?;
            KEYBOARD_HIDDEN.store(true, Ordering::Relaxed);
        }
        Ok(())
    }

    // Uses InputMethodManager.showSoftInput() which is more reliable than
    // ANativeActivity_showSoftInput (the NDK function is ignored on many devices).
    fn show() -> anyhow::Result<()> {
        let ndk_ctx = ndk_context::android_context();
        let vm = unsafe { JavaVM::from_raw(ndk_ctx.vm().cast()) };
        vm.attach_current_thread(|env| {
            let activity = unsafe { JObject::from_raw(env, ndk_ctx.context().cast()) };
            let window = AndroidUtil::call_method_impl(
                env,
                &activity,
                "getWindow",
                "()Landroid/view/Window;",
                &[],
            )?;
            let decor_view = AndroidUtil::call_method_impl(
                env,
                window.as_ref(),
                "getDecorView",
                "()Landroid/view/View;",
                &[],
            )?;
            let name = JNIString::new("requestFocus");
            let sig = RuntimeMethodSignature::from_str("()Z").context("requestFocus sig")?;
            env.call_method(decor_view.as_ref(), name, sig.method_signature(), &[])
                .context("requestFocus")?;
            let svc = env.new_string("input_method").context("new_string")?;
            let imm = AndroidUtil::call_method_impl(
                env,
                &activity,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[JValue::from(&svc)],
            )?;
            let name = JNIString::new("showSoftInput");
            let sig = RuntimeMethodSignature::from_str("(Landroid/view/View;I)Z")
                .context("showSoftInput sig")?;
            env.call_method(
                imm.as_ref(),
                name,
                sig.method_signature(),
                &[JValue::from(decor_view.as_ref()), JValue::Int(0)],
            )
            .context("showSoftInput")?;
            Ok(())
        })
    }
}
