fn main() {
    let frozen = "../../../artifacts/sms-binder/frozen-aidl";
    let probe = "../aidl/org/pokelinux/messagesincomingtest/IIncomingMessagesProbe.aidl";
    println!("cargo:rerun-if-changed={frozen}");
    println!("cargo:rerun-if-changed={probe}");
    let mut builder = rsbinder_aidl::Builder::new()
        .set_async_support(false)
        .source(frozen);
    for name in [
        "IRadioMessaging",
        "IRadioMessagingResponse",
        "IRadioMessagingIndication",
    ] {
        builder = builder
            .source(format!(
                "{frozen}/android/hardware/radio/messaging/{name}.aidl"
            ))
            .version(3)
            .hash("30b0bc0e84679bc3b5ccb3a52da34c47cda6b7eb");
    }
    builder
        .source(probe)
        .version(1)
        .hash("0000000000000000000000000000000000000000")
        .output("incoming.rs")
        .generate()
        .expect("generate frozen public messaging V3 fixture");
}
