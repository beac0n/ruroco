//! JNI FFI to the Android platform (JavaVM::from_raw, JObject::from_raw, and similar raw handle
//! casts). Only compiled under cfg(target_os = "android"); the raw pointers come from ndk_context
//! and are valid for the life of the activity, so the crate-wide `#![deny(unsafe_code)]` is
//! relaxed here rather than annotated per call site.
#![allow(unsafe_code)]

use anyhow::Context;
use jni::objects::{Global, JObject, JValue};
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
}
