//! Fixed presentation metadata handoff, separate from session permission authority.
#![forbid(unsafe_code)]
#[path = "../presentation_environment.rs"]
mod presentation_environment;
fn main() {
    if let Err(error) = presentation_environment::run() {
        eprintln!("presentation-environment: {error}");
        std::process::exit(1);
    }
}
