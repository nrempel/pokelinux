use std::fs::File;
fn main() {
    let mut args = std::env::args_os();
    args.next();
    let Some(path) = args.next() else {
        eprintln!("usage: pokelinux-boot-inputs IMAGE");
        std::process::exit(2);
    };
    if args.next().is_some() {
        eprintln!("usage: pokelinux-boot-inputs IMAGE");
        std::process::exit(2);
    }
    let result = File::open(path)
        .map_err(|_| "cannot open image".to_string())
        .and_then(|mut f| pokelinux_boot_inputs::inspect(&mut f).map_err(|e| e.to_string()));
    match result {
        Ok(report) => match serde_json::to_writer(std::io::stdout().lock(), &report) {
            Ok(()) => println!(),
            Err(_) => {
                eprintln!("cannot write report");
                std::process::exit(2)
            }
        },
        Err(error) => {
            eprintln!("refused: {error}");
            std::process::exit(1);
        }
    }
}
