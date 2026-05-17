#![cfg(target_os = "android")]

use crate::common::android_util::AndroidUtil;
use anyhow::Context;
use jni::objects::{JObject, JValue};
use jni::signature::{RuntimeFieldSignature, RuntimeMethodSignature};
use jni::strings::JNIString;
use jni::JavaVM;

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
        let density_sig = RuntimeFieldSignature::from_str("F").context("density sig")?;
        let density = env
            .get_field(metrics.as_ref(), JNIString::new("density"), density_sig.field_signature())
            .context("get density")?
            .f()
            .context("density as float")?;
        if density <= 0.0 {
            return Ok(0.0);
        }
        let name_s = env.new_string("status_bar_height").context("new_string name")?;
        let type_s = env.new_string("dimen").context("new_string type")?;
        let pkg_s = env.new_string("android").context("new_string pkg")?;
        let gi_sig = RuntimeMethodSignature::from_str(
            "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I",
        )
        .context("getIdentifier sig")?;
        let res_id = env
            .call_method(
                resources.as_ref(),
                JNIString::new("getIdentifier"),
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
        let dps_sig =
            RuntimeMethodSignature::from_str("(I)I").context("getDimensionPixelSize sig")?;
        let height_px = env
            .call_method(
                resources.as_ref(),
                JNIString::new("getDimensionPixelSize"),
                dps_sig.method_signature(),
                &[JValue::Int(res_id)],
            )
            .context("getDimensionPixelSize")?
            .i()
            .context("height_px as int")?;
        Ok(height_px as f32 / density)
    })
}
