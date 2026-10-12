fn main() {
    let schemas = "../framework/camerapeer/aidl";
    println!("cargo:rerun-if-changed={schemas}");
    rsbinder_aidl::Builder::new()
        .set_async_support(false)
        .source(schemas)
        .output("camera.rs")
        .generate()
        .expect("generate synthetic probe and frozen public NativeHandle V2");
}
