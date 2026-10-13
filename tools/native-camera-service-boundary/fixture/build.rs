fn main() {
    let frozen = std::env::var("CAMERA_FROZEN_SCHEMAS").expect("explicit verified frozen schemas");
    println!("cargo:rerun-if-env-changed=CAMERA_FROZEN_SCHEMAS");
    println!("cargo:rerun-if-changed={frozen}");
    println!("cargo:rerun-if-changed=../aidl");
    rsbinder_aidl::Builder::new()
        .set_async_support(false)
        .source(&frozen)
        .source(format!(
            "{frozen}/android/hardware/camera/device/ICameraDeviceCallback.aidl"
        ))
        .version(4)
        .hash("6e6878817de92e03fd52d5f10937714862ab4922")
        .source("../aidl/org/pokelinux/cameratest/ICameraOwnershipProbe.aidl")
        .version(1)
        .hash("0000000000000000000000000000000000000000")
        .output("camera.rs")
        .generate()
        .expect("generate public frozen cameraV4 and authored counter probe");
}
