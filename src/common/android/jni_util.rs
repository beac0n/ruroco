use anyhow::Context;
use jni::objects::{Global, JObject, JString, JValue, JValueOwned};
use jni::signature::RuntimeMethodSignature;
use jni::strings::JNIString;
use jni::Env;

use super::util::AndroidUtil;

impl AndroidUtil {
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

    pub(super) fn new_object_impl(
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

    pub(super) fn unpack_result<'local>(
        result: jni::errors::Result<JValueOwned<'local>>,
    ) -> anyhow::Result<JObject<'local>> {
        Ok(result
            .context("Failed to call method")?
            .l()
            .context("Failed to unwrap method call result")?)
    }
}
