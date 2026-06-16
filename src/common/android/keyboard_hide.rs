use anyhow::Context;
use jni::objects::{JObject, JValue};
use jni::signature::RuntimeMethodSignature;
use jni::strings::JNIString;
use jni::JavaVM;

use super::keyboard::AndroidKeyboard;
use super::util::AndroidUtil;

impl AndroidKeyboard {
    pub(super) fn hide() -> anyhow::Result<()> {
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
            let token = AndroidUtil::call_method_impl(
                env,
                decor_view.as_ref(),
                "getWindowToken",
                "()Landroid/os/IBinder;",
                &[],
            )?;
            let svc = env.new_string("input_method").context("new_string")?;
            let imm = AndroidUtil::call_method_impl(
                env,
                &activity,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[JValue::from(&svc)],
            )?;
            let name = JNIString::new("hideSoftInputFromWindow");
            let sig = RuntimeMethodSignature::from_str("(Landroid/os/IBinder;I)Z")
                .context("hideSoftInputFromWindow sig")?;
            env.call_method(
                imm.as_ref(),
                name,
                sig.method_signature(),
                &[JValue::from(token.as_ref()), JValue::Int(0)],
            )
            .context("hideSoftInputFromWindow")?;
            Ok(())
        })
    }
}
