use super::util::AndroidUtil;
use anyhow::Context;
use jni::objects::{JObject, JValue};
use jni::signature::RuntimeMethodSignature;
use jni::strings::JNIString;
use jni::JavaVM;

const PREFS_NAME: &str = "ruroco";
const MODE_PRIVATE: i32 = 0;
const GET_PREFS_SIG: &str = "(Ljava/lang/String;I)Landroid/content/SharedPreferences;";
const GET_STRING_SIG: &str = "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;";
const EDIT_SIG: &str = "()Landroid/content/SharedPreferences$Editor;";
const PUT_STRING_SIG: &str =
    "(Ljava/lang/String;Ljava/lang/String;)Landroid/content/SharedPreferences$Editor;";

pub(crate) struct AndroidPrefs;

impl AndroidPrefs {
    pub(crate) fn get_string(key: &str) -> anyhow::Result<Option<String>> {
        let ndk_ctx = ndk_context::android_context();
        let vm = unsafe { JavaVM::from_raw(ndk_ctx.vm().cast()) };
        vm.attach_current_thread(|env| {
            let activity = unsafe { JObject::from_raw(env, ndk_ctx.context().cast()) };

            let name = env.new_string(PREFS_NAME).context("new_string prefs name")?;
            let prefs = AndroidUtil::call_method_impl(
                env,
                &activity,
                "getSharedPreferences",
                GET_PREFS_SIG,
                &[JValue::from(&name), JValue::Int(MODE_PRIVATE)],
            )?;

            let key_j = env.new_string(key).context("new_string key")?;
            let default_j = env.new_string("").context("new_string default")?;
            let result = AndroidUtil::call_method_impl(
                env,
                prefs.as_ref(),
                "getString",
                GET_STRING_SIG,
                &[JValue::from(&key_j), JValue::from(&default_j)],
            )?;

            let s = AndroidUtil::to_string_impl(env, &result)?;
            Ok(if s.is_empty() { None } else { Some(s) })
        })
    }

    pub(crate) fn put_string(key: &str, value: &str) -> anyhow::Result<()> {
        let ndk_ctx = ndk_context::android_context();
        let vm = unsafe { JavaVM::from_raw(ndk_ctx.vm().cast()) };
        vm.attach_current_thread(|env| {
            let activity = unsafe { JObject::from_raw(env, ndk_ctx.context().cast()) };

            let name = env.new_string(PREFS_NAME).context("new_string prefs name")?;
            let prefs = AndroidUtil::call_method_impl(
                env,
                &activity,
                "getSharedPreferences",
                GET_PREFS_SIG,
                &[JValue::from(&name), JValue::Int(MODE_PRIVATE)],
            )?;

            let editor = AndroidUtil::call_method_impl(env, prefs.as_ref(), "edit", EDIT_SIG, &[])?;

            let key_j = env.new_string(key).context("new_string key")?;
            let value_j = env.new_string(value).context("new_string value")?;
            AndroidUtil::call_method_impl(
                env,
                editor.as_ref(),
                "putString",
                PUT_STRING_SIG,
                &[JValue::from(&key_j), JValue::from(&value_j)],
            )?;

            let apply_sig = RuntimeMethodSignature::from_str("()V").context("apply sig")?;
            env.call_method(
                editor.as_ref(),
                JNIString::new("apply"),
                apply_sig.method_signature(),
                &[],
            )
            .context("apply")?;

            Ok(())
        })
    }
}
