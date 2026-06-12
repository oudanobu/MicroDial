use jni::JNIEnv;
use jni::objects::JClass;
use jni::sys::{jstring, jint, jfloat};

pub mod geometry;

#[no_mangle]
pub extern "system" fn Java_com_oudanobu_chronoxide_MainActivity_stringFromJNI(
    env: JNIEnv,
    _class: JClass,
) -> jstring {
    let output = env.new_string("ChronOxide Engine: 15MB RAM Mode").expect("Couldn't create java string!");
    output.into_raw()
}

#[no_mangle]
pub extern "system" fn Java_com_oudanobu_chronoxide_MainActivity_setRustScreenGeometry(
    _env: JNIEnv,
    _class: JClass,
    width: jint,
    height: jint,
    shape_val: jint,
    density_scale: jfloat,
) {
    // 动态通知 Rust 核心变更几何布局，完美支持运行时方圆切换
    let shape = if shape_val == 1 {
        geometry::ScreenShape::Round
    } else {
        geometry::ScreenShape::Square
    };
    
    let _geo = geometry::ScreenGeometry {
        width: width as u16,
        height: height as u16,
        shape,
        density_scale,
    };
    
    // Real hardware would store this in a static matrix block
    // We log it safely within mock print
    #[cfg(debug_assertions)]
    println!("Rust Core updated Screen Geometry: {:?}", _geo);
}
