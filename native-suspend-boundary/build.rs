fn main() {
    println!("cargo:rerun-if-changed=../framework/suspendpeer/aidl");
    println!("cargo:rerun-if-changed=../artifacts/suspend-boundary/frozen-aidl");
    rsbinder_aidl::Builder::new()
        .set_async_support(false)
        .source("../framework/suspendpeer/aidl")
        .source("../artifacts/suspend-boundary/frozen-aidl")
        .source(
            "../artifacts/suspend-boundary/frozen-aidl/android/system/suspend/ISystemSuspend.aidl",
        )
        .version(1)
        .hash("082a58c2505b703702f34ab8abcb1d1b5e890be4")
        .source("../artifacts/suspend-boundary/frozen-aidl/android/system/suspend/IWakeLock.aidl")
        .version(1)
        .hash("082a58c2505b703702f34ab8abcb1d1b5e890be4")
        .output("suspend.rs")
        .generate()
        .expect("exact frozen V1 suspend bindings");
}
