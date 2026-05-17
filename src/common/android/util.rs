use anyhow::Context;
use jni::objects::{Global, JObject, JString, JValue, JValueOwned};
use jni::signature::RuntimeMethodSignature;
use jni::strings::JNIString;
use jni::{Env, JavaVM};
use std::path::PathBuf;

pub(crate) const J_STRING: &str = "()Ljava/lang/String;";
pub(crate) const J_FILE: &str = "()Ljava/io/File;";

pub(crate) struct AndroidUtil {
    ctx: Global<JObject<'static>>,
    vm: JavaVM,
}

impl AndroidUtil {
    pub(crate) fn create() -> anyhow::Result<AndroidUtil> {
        let ndk_ctx = ndk_context::android_context();
        let vm = unsafe { JavaVM::from_raw(ndk_ctx.vm().cast()) };
        let ctx = vm.attach_current_thread(|env| {
            let obj = unsafe { JObject::from_raw(env, ndk_ctx.context().cast()) };
            env.new_global_ref(&obj).context("Could not create global ref for context")
        })?;
        Ok(AndroidUtil { ctx, vm })
    }

    pub(crate) fn get_conf_dir(&self) -> anyhow::Result<PathBuf> {
        let ctx = &self.ctx;
        self.vm.attach_current_thread(|env| {
            let files_dir = Self::call_method_impl(env, ctx.as_ref(), "getFilesDir", J_FILE, &[])?;
            let abs_path =
                Self::call_method_impl(env, files_dir.as_ref(), "getAbsolutePath", J_STRING, &[])?;
            Self::to_string_impl(env, &abs_path).map(PathBuf::from)
        })
    }

    /// see https://developer.android.com/reference/android/content/Context#startActivity(android.content.Intent)
    pub(crate) fn start_activity(
        &self,
        intent: &Global<JObject<'static>>,
    ) -> anyhow::Result<Global<JObject<'static>>> {
        let ctx = &self.ctx;
        self.vm.attach_current_thread(|env| {
            Self::call_method_impl(
                env,
                ctx.as_ref(),
                "startActivity",
                "(Landroid/content/Intent;)V",
                &[JValue::from(intent.as_ref())],
            )
        })
    }

    /// see https://developer.android.com/reference/android/content/Intent
    pub(crate) fn new_view_intent(
        &self,
        uri: &Global<JObject<'static>>,
    ) -> anyhow::Result<Global<JObject<'static>>> {
        self.vm.attach_current_thread(|env| {
            let action =
                env.new_string("android.intent.action.VIEW").context("Failed to create action")?;
            let intent = Self::new_object_impl(
                env,
                "android/content/Intent",
                "(Ljava/lang/String;Landroid/net/Uri;)V",
                &[JValue::from(&action), JValue::from(uri.as_ref())],
            )?;
            // FLAG_ACTIVITY_NEW_TASK required when starting activity from non-Activity context
            Self::call_method_impl(
                env,
                intent.as_ref(),
                "addFlags",
                "(I)Landroid/content/Intent;",
                &[JValue::Int(0x10000000)],
            )?;
            Ok(intent)
        })
    }

    /// see https://developer.android.com/reference/android/net/Uri#parse(java.lang.String)
    pub(crate) fn uri_parse(&self, url: String) -> anyhow::Result<Global<JObject<'static>>> {
        self.vm.attach_current_thread(|env| {
            let url_str = env.new_string(&url).context("Failed to create URL string")?;
            Self::call_static_method_impl(
                env,
                "android/net/Uri",
                "parse",
                "(Ljava/lang/String;)Landroid/net/Uri;",
                &[JValue::from(&url_str)],
            )
        })
    }

    pub(crate) fn call_method_impl(
        env: &mut Env<'_>,
        obj: &JObject<'_>,
        name: &str,
        sig: &str,
        args: &[JValue<'_>],
    ) -> anyhow::Result<Global<JObject<'static>>> {
        let name = JNIString::new(name);
        let sig =
            RuntimeMethodSignature::from_str(sig).context("Failed to parse method signature")?;
        let result = env.call_method(obj, name, sig.method_signature(), args);
        let obj = Self::unpack_result(result)?;
        env.new_global_ref(&obj).context("Failed to create global ref")
    }

    pub(crate) fn call_static_method_impl(
        env: &mut Env<'_>,
        class: &str,
        name: &str,
        sig: &str,
        args: &[JValue<'_>],
    ) -> anyhow::Result<Global<JObject<'static>>> {
        let class = JNIString::new(class);
        let name = JNIString::new(name);
        let sig =
            RuntimeMethodSignature::from_str(sig).context("Failed to parse method signature")?;
        let result = env.call_static_method(class, name, sig.method_signature(), args);
        let obj = Self::unpack_result(result)?;
        env.new_global_ref(&obj).context("Failed to create global ref")
    }

    fn new_object_impl(
        env: &mut Env<'_>,
        class: &str,
        sig: &str,
        args: &[JValue<'_>],
    ) -> anyhow::Result<Global<JObject<'static>>> {
        let class = JNIString::new(class);
        let sig =
            RuntimeMethodSignature::from_str(sig).context("Failed to parse method signature")?;
        let obj = env
            .new_object(class, sig.method_signature(), args)
            .context("Failed to create new object")?;
        env.new_global_ref(&obj).context("Failed to create global ref")
    }

    pub(crate) fn to_string_impl(
        env: &Env<'_>,
        global_ref: &Global<JObject<'static>>,
    ) -> anyhow::Result<String> {
        // Safety: we know this JObject is actually a java.lang.String
        let cast = unsafe { env.as_cast_unchecked::<JString>(global_ref.as_ref()) };
        let chars = cast.mutf8_chars(env).context("Failed to get string chars")?;
        Ok(chars.into())
    }

    fn unpack_result<'local>(
        result: jni::errors::Result<JValueOwned<'local>>,
    ) -> anyhow::Result<JObject<'local>> {
        Ok(result
            .context("Failed to call method")?
            .l()
            .context("Failed to unwrap method call result")?)
    }
}
