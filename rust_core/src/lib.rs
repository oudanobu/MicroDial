use jni::JNIEnv;
use jni::objects::JClass;
use jni::sys::jstring;

#[no_mangle]
pub extern "system" fn Java_com_oudanobu_chronoxide_MainActivity_stringFromJNI(
    env: JNIEnv,
    _class: JClass,
) -> jstring {
    let output = env.new_string("ChronOxide Engine: 15MB RAM Mode").expect("Couldn't create java string!");
    output.into_raw()
}
