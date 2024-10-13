use crate::ui::run_ui;
use jni::objects::{JObject, JString};

#[no_mangle]
unsafe fn android_main(app: slint::android::AndroidApp) {
    // TODO: add network permissions: https://developer.android.com/develop/connectivity/network-ops/connecting?hl=de

    let private_files_path =
        get_files_path().map_err(|e| format!("Could not get files path: {e:?}")).unwrap();

    slint::android::init(app).map_err(|e| format!("Could not init slint: {e}")).unwrap();
    run_ui(private_files_path).map_err(|e| format!("Could not run UI: {e}")).unwrap();
}

fn get_files_path() -> Result<String, Box<dyn std::error::Error>> {
    let ctx = ndk_context::android_context();
    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }?;
    let context = unsafe { JObject::from_raw(ctx.context().cast()) };
    let mut env = vm.attach_current_thread()?;

    let files_dir = env.call_method(context, "getFilesDir", "()Ljava/io/File;", &[])?.l()?;

    let path_object =
        env.call_method(files_dir, "getAbsolutePath", "()Ljava/lang/String;", &[])?.l()?;

    let path_jstring: JString = path_object.try_into()?;
    let file_path = env.get_string(&path_jstring)?.into();
    Ok(file_path)
}
