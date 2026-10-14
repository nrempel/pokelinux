fn main() {
    let args: Vec<_> = std::env::args_os().collect();
    let parent = args
        .get(2)
        .and_then(|v| v.to_str())
        .and_then(|v| v.parse::<u32>().ok());
    if args.len() != 3
        || args[1] != "--parent"
        || parent.is_none_or(|p| p < 2 || p > i32::MAX as u32)
        || pokelinux_pam::ownership_fixture_worker(parent.unwrap()).is_err()
    {
        std::process::exit(1);
    }
}
