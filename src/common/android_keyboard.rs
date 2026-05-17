#![cfg(target_os = "android")]

use crate::common::android_util::AndroidUtil;
use anyhow::Context;
use jni::objects::{JObject, JValue};
use jni::signature::RuntimeMethodSignature;
use jni::strings::JNIString;
use jni::JavaVM;

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
