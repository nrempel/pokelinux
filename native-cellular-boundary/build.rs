fn main() {
    let frozen = "aidl";
    println!("cargo:rerun-if-changed={frozen}");
    let mut builder = rsbinder_aidl::Builder::new()
        .set_async_support(false)
        .source(frozen);
    for name in ["IRadioData", "IRadioDataResponse", "IRadioDataIndication"] {
        builder = builder
            .source(format!("{frozen}/android/hardware/radio/data/{name}.aidl"))
            .version(3)
            .hash("cd8913a3f9d39f1cc0a5fcf9e90257be94ec38df");
    }
    if std::env::var_os("CARGO_FEATURE_TEST_FIXTURE").is_some() {
        let fixture = "../tools/native-cellular-data/fixture-aidl";
        println!("cargo:rerun-if-changed={fixture}");
        println!("cargo:rerun-if-changed=../tools/native-cellular-data/fixture-interface-hash.txt");
        let hash =
            std::fs::read_to_string("../tools/native-cellular-data/fixture-interface-hash.txt")
                .expect("read authored fixture interface hash");
        assert_eq!(hash.trim(), "b74bae7e2fba9b8fbb574174a24e7431dbb6281b");
        builder = builder
            .source(fixture)
            .source(format!(
                "{fixture}/org/pokelinux/datatest/IDataFixture.aidl"
            ))
            .version(1)
            .hash(hash.trim());
    }
    builder
        .output("cellular.rs")
        .generate()
        .expect("generate frozen public Data V3 ABI");
}
