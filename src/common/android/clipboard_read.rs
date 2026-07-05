//! JNI FFI to the Android platform (JavaVM::from_raw, JObject::from_raw, and similar raw handle
//! casts). Only compiled under cfg(target_os = "android"); the raw pointers come from ndk_context
//! and are valid for the life of the activity, so the crate-wide `#![deny(unsafe_code)]` is
//! relaxed here rather than annotated per call site.
#![allow(unsafe_code)]

use anyhow::Context;
use jni::objects::{JObject, JValue};
use jni::signature::RuntimeMethodSignature;
use jni::strings::JNIString;
use jni::JavaVM;

use super::clipboard::AndroidClipboard;
use super::util::AndroidUtil;

impl AndroidClipboard {
    pub(crate) fn get_text() -> anyhow::Result<String> {
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

            let clip_sig = RuntimeMethodSignature::from_str("()Landroid/content/ClipData;")
                .context("getPrimaryClip sig")?;
            let clip_obj = env
                .call_method(
                    clipboard_mgr.as_ref(),
                    JNIString::new("getPrimaryClip"),
                    clip_sig.method_signature(),
                    &[],
                )
                .context("getPrimaryClip")?
                .l()
                .context("getPrimaryClip as obj")?;

            if clip_obj.is_null() {
                anyhow::bail!("Clipboard is empty");
            }

            let count_sig = RuntimeMethodSignature::from_str("()I").context("getItemCount sig")?;
            let count = env
                .call_method(
                    &clip_obj,
                    JNIString::new("getItemCount"),
                    count_sig.method_signature(),
                    &[],
                )
                .context("getItemCount")?
                .i()
                .context("count as int")?;

            if count <= 0 {
                anyhow::bail!("Clipboard is empty");
            }

            let item_sig = RuntimeMethodSignature::from_str("(I)Landroid/content/ClipData$Item;")
                .context("getItemAt sig")?;
            let item_obj = env
                .call_method(
                    &clip_obj,
                    JNIString::new("getItemAt"),
                    item_sig.method_signature(),
                    &[JValue::Int(0)],
                )
                .context("getItemAt")?
                .l()
                .context("getItemAt as obj")?;

            let coerce_sig = RuntimeMethodSignature::from_str(
                "(Landroid/content/Context;)Ljava/lang/CharSequence;",
            )
            .context("coerceToText sig")?;
            let char_seq = env
                .call_method(
                    &item_obj,
                    JNIString::new("coerceToText"),
                    coerce_sig.method_signature(),
                    &[JValue::from(&activity)],
                )
                .context("coerceToText")?
                .l()
                .context("coerceToText as obj")?;

            let str_global = AndroidUtil::call_method_impl(
                env,
                &char_seq,
                "toString",
                "()Ljava/lang/String;",
                &[],
            )?;
            AndroidUtil::to_string_impl(env, &str_global)
        })
    }
}
