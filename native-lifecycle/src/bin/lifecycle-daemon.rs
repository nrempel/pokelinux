fn main() -> std::io::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() == 4 && args[0] == "--isolated-storage-reap-error" {
        return pokelinux_lifecycle::storage::isolated_reap_error_test(
            std::path::Path::new(&args[1]),
            std::path::Path::new(&args[2]),
            args[3].to_str().ok_or(std::io::ErrorKind::InvalidInput)?,
        );
    }
    if args.first().is_some_and(|a| a == "--storage-worker") {
        return pokelinux_lifecycle::storage::worker(&args[1..]);
    }
    let test =
        if args.len() == 5 && std::env::var("NATIVE_LIFECYCLE_ISOLATED").as_deref() == Ok("1") {
            args[4]
                .to_str()
                .and_then(|s| s.strip_prefix("--storage-test="))
        } else {
            None
        };
    if args.len() != 4 && test.is_none() {
        return Err(std::io::ErrorKind::InvalidInput.into());
    }
    pokelinux_lifecycle::service::run(
        std::path::Path::new(&args[0]),
        std::path::Path::new(&args[1]),
        std::path::Path::new(&args[2]),
        std::path::Path::new(&args[3]),
        test,
    )
}
