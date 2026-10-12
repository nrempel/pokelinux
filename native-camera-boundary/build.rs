fn main() {
    let schemas = "../framework/camerapeer/aidl";
    println!("cargo:rerun-if-changed={schemas}");
    println!("cargo:rerun-if-changed=../artifacts/camera-boundary/frozen-aidl");
    rsbinder_aidl::Builder::new()
        .set_async_support(false)
        .source(schemas)
        .source("../artifacts/camera-boundary/frozen-aidl")
        .source("../artifacts/camera-boundary/frozen-aidl/android/hardware/camera/device/ICameraDeviceCallback.aidl")
        .version(4)
        .hash("6e6878817de92e03fd52d5f10937714862ab4922")
        .output("camera.rs")
        .generate()
        .expect("generate synthetic probe and frozen public NativeHandle V2");
}
