#![cfg(target_os = "android")]

use anyhow::Context;
use jni::objects::{Global, JObject, JString, JValue, JValueOwned};
use jni::signature::{RuntimeFieldSignature, RuntimeMethodSignature};
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

    fn call_method_impl(
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

    fn call_static_method_impl(
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

    fn to_string_impl(
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

// Uses InputMethodManager.showSoftInput() which is more reliable than
// ANativeActivity_showSoftInput (the NDK function is ignored on many devices).
pub(crate) fn show_soft_keyboard() -> anyhow::Result<()> {
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
        // requestFocus returns boolean — call directly with proper JNI types
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
        // showSoftInput returns boolean — call directly with proper JNI types
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

/// Returns the system status bar height in dp (= egui logical points).
/// Safe to call at startup — reads a fixed system resource, no layout timing issues.
pub(crate) fn status_bar_height_dp() -> anyhow::Result<f32> {
    let ndk_ctx = ndk_context::android_context();
    let vm = unsafe { JavaVM::from_raw(ndk_ctx.vm().cast()) };
    vm.attach_current_thread(|env| {
        let activity = unsafe { JObject::from_raw(env, ndk_ctx.context().cast()) };
        let resources = AndroidUtil::call_method_impl(
            env,
            &activity,
            "getResources",
            "()Landroid/content/res/Resources;",
            &[],
        )?;
        let metrics = AndroidUtil::call_method_impl(
            env,
            resources.as_ref(),
            "getDisplayMetrics",
            "()Landroid/util/DisplayMetrics;",
            &[],
        )?;
        // Read the float field DisplayMetrics.density
        let density_sig = RuntimeFieldSignature::from_str("F").context("density sig")?;
        let density = env
            .get_field(metrics.as_ref(), JNIString::new("density"), density_sig.field_signature())
            .context("get density")?
            .f()
            .context("density as float")?;
        if density <= 0.0 {
            return Ok(0.0);
        }
        // Look up the status_bar_height dimension resource id
        let name_s = env.new_string("status_bar_height").context("new_string name")?;
        let type_s = env.new_string("dimen").context("new_string type")?;
        let pkg_s = env.new_string("android").context("new_string pkg")?;
        let gi_name = JNIString::new("getIdentifier");
        let gi_sig = RuntimeMethodSignature::from_str(
            "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I",
        )
        .context("getIdentifier sig")?;
        let res_id = env
            .call_method(
                resources.as_ref(),
                gi_name,
                gi_sig.method_signature(),
                &[
                    JValue::from(&name_s),
                    JValue::from(&type_s),
                    JValue::from(&pkg_s),
                ],
            )
            .context("getIdentifier")?
            .i()
            .context("res_id as int")?;
        if res_id == 0 {
            return Ok(0.0);
        }
        // Get the pixel size and convert to dp
        let dps_name = JNIString::new("getDimensionPixelSize");
        let dps_sig =
            RuntimeMethodSignature::from_str("(I)I").context("getDimensionPixelSize sig")?;
        let height_px = env
            .call_method(
                resources.as_ref(),
                dps_name,
                dps_sig.method_signature(),
                &[JValue::Int(res_id)],
            )
            .context("getDimensionPixelSize")?
            .i()
            .context("height_px as int")?;
        Ok(height_px as f32 / density)
    })
}

pub(crate) fn get_clipboard_text() -> anyhow::Result<String> {
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

        let coerce_sig =
            RuntimeMethodSignature::from_str("(Landroid/content/Context;)Ljava/lang/CharSequence;")
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

        let str_global =
            AndroidUtil::call_method_impl(env, &char_seq, "toString", "()Ljava/lang/String;", &[])?;
        AndroidUtil::to_string_impl(env, &str_global)
    })
}

pub(crate) fn hide_soft_keyboard() -> anyhow::Result<()> {
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
        // hideSoftInputFromWindow returns boolean — call directly with proper JNI types
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
