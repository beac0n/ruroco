#![cfg(target_os = "android")]

use anyhow::Context;
use jni::objects::{GlobalRef, JObject, JString, JValue, JValueOwned};
use jni::{AttachGuard, JavaVM};
use std::ops::Deref;
use std::path::PathBuf;

pub(crate) const J_STRING: &str = "()Ljava/lang/String;";
pub(crate) const J_FILE: &str = "()Ljava/io/File;";

pub(crate) struct AndroidUtil {
    ctx: JObject<'static>,
    vm: JavaVM,
}

impl AndroidUtil {
    pub(crate) fn create() -> anyhow::Result<AndroidUtil> {
        let ctx = ndk_context::android_context();
        let obj = unsafe { JObject::from_raw(ctx.context().cast()) };
        let vm =
            (unsafe { JavaVM::from_raw(ctx.vm().cast()) }).with_context(|| "Could not get JavaVM from raw")?;
        Ok(AndroidUtil { ctx: obj, vm })
    }

    pub(crate) fn get_conf_dir(&self) -> anyhow::Result<PathBuf> {
        let files_dir_obj = self.call_ctx_method("getFilesDir", J_FILE, &[])?;
        let abs_path_ref = self.call_method(files_dir_obj, "getAbsolutePath", J_STRING, &[])?;
        Ok(PathBuf::from(self.global_ref_to_string(abs_path_ref)?))
    }

    /// see https://developer.android.com/reference/android/content/Context#startActivity(android.content.Intent)
    pub(crate) fn start_activity(&self, intent: &JObject) -> anyhow::Result<GlobalRef> {
        self.call_ctx_method(
            "startActivity",
            "(Landroid/content/Intent;)V",
            &[JValue::from(&intent)],
        )
    }

    /// see https://developer.android.com/reference/android/content/Intent
    pub(crate) fn new_view_intent<'a>(
        &'a self,
        uri: &'a GlobalRef,
    ) -> anyhow::Result<JObject<'a>> {
        self.new_object(
            "android/content/Intent",
            "(Ljava/lang/String;Landroid/net/Uri;)V",
            &[
                JValue::from(&self.new_string("android.intent.action.VIEW")?),
                JValue::from(&uri),
            ],
        )
    }

    /// see https://developer.android.com/reference/android/net/Uri#parse(java.lang.String)
    pub(crate) fn uri_parse(&self, url: String) -> anyhow::Result<GlobalRef> {
        self.call_static_method(
            "android/net/Uri",
            "parse",
            "(Ljava/lang/String;)Landroid/net/Uri;",
            &[JValue::from(&self.new_string(&url)?)],
        )
    }

    pub(crate) fn call_ctx_method(
        &self,
        name: &str,
        sig: &str,
        args: &[JValue],
    ) -> anyhow::Result<GlobalRef> {
        self.call_method(&self.ctx, name, sig, args)
    }

    pub(crate) fn call_method<'a, O>(
        &self,
        obj: O,
        name: &str,
        sig: &str,
        args: &[JValue],
    ) -> anyhow::Result<GlobalRef>
    where
        O: AsRef<JObject<'a>>,
    {
        let mut env = self.get_env()?;
        let call_result = env.call_method(obj, name, sig, args);
        Self::create_global_ref(env, Self::unpack_call_result(call_result)?)
    }

    fn call_static_method(
        &self,
        class: &str,
        name: &str,
        sig: &str,
        args: &[JValue],
    ) -> anyhow::Result<GlobalRef> {
        let mut env = self.get_env()?;
        let call_result = env.call_static_method(class, name, sig, args);
        Self::create_global_ref(env, Self::unpack_call_result(call_result)?)
    }

    fn new_string(&self, string: &str) -> anyhow::Result<JString> {
        let env = self.get_env()?;
        env.new_string(string)
            .with_context(|| "Failed to create new string")
    }

    fn new_object(&self, class: &str, sig: &str, args: &[JValue]) -> anyhow::Result<JObject> {
        let mut env = self.get_env()?;
        let intent = env
            .new_object(class, sig, args)
            .with_context(|| "Failed to create new object")?;
        Ok(intent)
    }

    pub(crate) fn global_ref_to_string(&self, global_ref: GlobalRef) -> anyhow::Result<String> {
        let mut env = self.get_env()?;
        let j_str: &JString = global_ref
            .deref()
            .try_into()
            .with_context(|| "Failed to deref")?;
        let rust_str: String = env
            .get_string(&j_str)
            .with_context(|| "Failed to get_string")?
            .into();
        Ok(rust_str)
    }

    fn create_global_ref(env: AttachGuard, o: JObject) -> anyhow::Result<GlobalRef> {
        Ok(env
            .new_global_ref(o)
            .with_context(|| "Failed to create global ref")?)
    }

    fn get_env(&self) -> anyhow::Result<AttachGuard> {
        Ok(self
            .vm
            .attach_current_thread()
            .with_context(|| "Failed to attach vm to current thread")?)
    }

    fn unpack_call_result(result: jni::errors::Result<JValueOwned>) -> anyhow::Result<JObject> {
        Ok(result
            .with_context(|| "Failed to call method")?
            .l()
            .with_context(|| "Failed to unwrap method call result")?)
    }
}
