#![cfg(target_os = "android")]

use jni::objects::{GlobalRef, JObject, JString, JValue, JValueOwned};
use jni::{AttachGuard, JavaVM};
use std::ops::Deref;

pub const J_STRING: &str = "()Ljava/lang/String;";
pub const J_FILE: &str = "()Ljava/io/File;";

pub struct AndroidUtil {
    pub ctx: JObject<'static>,
    pub vm: JavaVM,
}

impl AndroidUtil {
    pub fn create() -> AndroidUtil {
        let ctx = ndk_context::android_context();
        let obj = unsafe { JObject::from_raw(ctx.context().cast()) };
        let vm = (unsafe { JavaVM::from_raw(ctx.vm().cast()) }).unwrap();
        AndroidUtil { ctx: obj, vm }
    }

    /// see https://developer.android.com/reference/android/content/Context#startActivity(android.content.Intent)
    pub fn start_activity(&self, intent: &JObject) -> Result<GlobalRef, String> {
        self.call_ctx_method(
            "startActivity",
            "(Landroid/content/Intent;)V",
            &[JValue::from(&intent)],
        )
    }

    /// see https://developer.android.com/reference/android/content/Intent
    pub fn new_view_intent<'a>(&'a self, uri: &'a GlobalRef) -> Result<JObject<'a>, String> {
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
    pub fn uri_parse(&self, url: String) -> Result<GlobalRef, String> {
        self.call_static_method(
            "android/net/Uri",
            "parse",
            "(Ljava/lang/String;)Landroid/net/Uri;",
            &[JValue::from(&self.new_string(&url)?)],
        )
    }

    pub fn call_ctx_method(
        &self,
        name: &str,
        sig: &str,
        args: &[JValue],
    ) -> Result<GlobalRef, String> {
        self.call_method(&self.ctx, name, sig, args)
    }

    pub fn call_method<'a, O>(
        &self,
        obj: O,
        name: &str,
        sig: &str,
        args: &[JValue],
    ) -> Result<GlobalRef, String>
    where
        O: AsRef<JObject<'a>>,
    {
        let mut env = self.get_env()?;
        let call_result = env.call_method(obj, name, sig, args);
        Self::create_global_ref(env, Self::unpack_call_result(call_result)?)
    }

    pub fn call_static_method(
        &self,
        class: &str,
        name: &str,
        sig: &str,
        args: &[JValue],
    ) -> Result<GlobalRef, String> {
        let mut env = self.get_env()?;
        let call_result = env.call_static_method(class, name, sig, args);
        Self::create_global_ref(env, Self::unpack_call_result(call_result)?)
    }

    pub fn new_string(&self, string: &str) -> Result<JString, String> {
        let env = self.get_env()?;
        env.new_string(string).map_err(|err| format!("Failed to create new string: {err:?}"))
    }

    pub fn new_object(&self, class: &str, sig: &str, args: &[JValue]) -> Result<JObject, String> {
        let mut env = self.get_env()?;
        let intent = env
            .new_object(class, sig, args)
            .map_err(|err| format!("Failed to create new object: {err:?}"))?;
        Ok(intent)
    }

    pub fn global_ref_to_string(&self, global_ref: GlobalRef) -> Result<String, String> {
        let mut env = self.get_env()?;
        let j_str: &JString =
            global_ref.deref().try_into().map_err(|err| format!("Failed to deref: {err:?}"))?;
        let rust_str: String =
            env.get_string(&j_str).map_err(|err| format!("Failed to get_string: {err:?}"))?.into();
        Ok(rust_str)
    }

    fn create_global_ref(env: AttachGuard, o: JObject) -> Result<GlobalRef, String> {
        Ok(env.new_global_ref(o).map_err(|err| format!("Failed to create global ref: {err}"))?)
    }

    fn get_env(&self) -> Result<AttachGuard, String> {
        Ok(self
            .vm
            .attach_current_thread()
            .map_err(|err| format!("Failed to attach vm to current thread: {err}"))?)
    }

    fn unpack_call_result(result: jni::errors::Result<JValueOwned>) -> Result<JObject, String> {
        Ok(result
            .map_err(|err| format!("Failed to call method: {err}"))?
            .l()
            .map_err(|err| format!("Failed to unwrap method call result: {err}"))?)
    }
}
