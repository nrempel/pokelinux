fn main() {
    println!("cargo:rerun-if-changed=aidl/RadioExternalResponseInfo.aidl");
    rsbinder_aidl::Builder::new()
        .set_async_support(false)
        .source("aidl/RadioExternalResponseInfo.aidl")
        .output("radio.rs")
        .generate()
        .expect("generate recovered radio response schema");
}
