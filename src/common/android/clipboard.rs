use super::util::AndroidUtil;
use anyhow::Context;
use jni::objects::{JObject, JValue};
use jni::signature::RuntimeMethodSignature;
use jni::strings::JNIString;
use jni::JavaVM;

pub(crate) struct AndroidClipboard;

impl AndroidClipboard {
    pub(crate) fn set_text(text: &str) -> anyhow::Result<()> {
        let ndk_ctx = ndk_context::android_context();
        let vm = unsafe { JavaVM::from_raw(ndk_ctx.vm().cast()) };
        vm.attach_current_thread(|env| {
            let activity = unsafe { JObject::from_raw(env, ndk_ctx.context().cast()) };

            let svc = env.new_string("clipboard").context("new_string clipboard")?;
            let clipboard_mgr = AndroidUtil::call_method_impl(
                env,
                &activity,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[JValue::from(&svc)],
            )?;

            let label = env.new_string("text").context("new_string label")?;
            let content = env.new_string(text).context("new_string content")?;
            let clip_obj = AndroidUtil::call_static_method_impl(
                env,
                "android/content/ClipData",
                "newPlainText",
                "(Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Landroid/content/ClipData;",
                &[JValue::from(&label), JValue::from(&content)],
            )?;

            let sig = RuntimeMethodSignature::from_str("(Landroid/content/ClipData;)V")
                .context("setPrimaryClip sig")?;
            env.call_method(
                clipboard_mgr.as_ref(),
                JNIString::new("setPrimaryClip"),
                sig.method_signature(),
                &[JValue::from(clip_obj.as_ref())],
            )
            .context("setPrimaryClip")?;

            Ok(())
        })
    }
}
