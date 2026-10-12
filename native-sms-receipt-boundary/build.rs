fn main() {
    let frozen = "../artifacts/sms-binder/frozen-aidl";
    println!("cargo:rerun-if-changed={frozen}");
    println!("cargo:rerun-if-changed=../framework/smsreceiptpeer/aidl");
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
        .source(
            "../framework/smsreceiptpeer/aidl/org/pokelinux/smsreceipttest/ISmsReceiptProbe.aidl",
        )
        .version(1)
        .hash("0000000000000000000000000000000000000000")
        .output("sms_receipt.rs")
        .generate()
        .expect("generate frozen public messaging V3 boundary");
}
