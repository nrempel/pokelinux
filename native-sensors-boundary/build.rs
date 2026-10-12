fn main() {
    let root = "../artifacts/sensors-boundary/public/frozen-aidl";
    println!("cargo:rerun-if-changed={root}");
    println!("cargo:rerun-if-changed=../framework/sensorpeer/aidl");
    rsbinder_aidl::Builder::new()
        .set_async_support(false)
        .source("../framework/sensorpeer/aidl")
        .source(root)
        .include_dir(root)
        .source(format!("{root}/android/hardware/sensors/ISensors.aidl"))
        .version(3)
        .hash("b051523be857524c99057b69f253a7d0bd295400")
        .source(format!(
            "{root}/android/hardware/sensors/ISensorsCallback.aidl"
        ))
        .version(3)
        .hash("b051523be857524c99057b69f253a7d0bd295400")
        .output("sensor.rs")
        .generate()
        .expect("generate exact SensorsV3 and own controls");
}
