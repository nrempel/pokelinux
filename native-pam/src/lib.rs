//! Private native authentication worker. Not an app API or unlock authority.
mod entry;
mod pam;
pub mod wire;

pub fn worker(parent: u32) -> std::io::Result<()> {
    run(parent, pam::Policy::Production)
}

/// Fixed isolated integration policy, absent from the default production build.
#[cfg(feature = "ownership-fixture")]
pub fn ownership_fixture_worker(parent: u32) -> std::io::Result<()> {
    run(parent, pam::Policy::OwnershipFixture)
}

fn run(parent: u32, policy: pam::Policy) -> std::io::Result<()> {
    entry::initial(parent)?;
    let until = wire::start(&mut std::io::stdin().lock())?;
    let account = entry::enter(parent)?;
    // Own Send handles; never carry Rust's global stdio lock guards through PAM,
    // its foreign callback threads, or its fork/exec helper path.
    let input = std::fs::File::from(rustix::io::fcntl_dupfd_cloexec(std::io::stdin(), 3)?);
    let mut output = std::fs::File::from(rustix::io::fcntl_dupfd_cloexec(std::io::stdout(), 3)?);
    wire::ready(&mut output, until)?;
    let (outcome, mut input, mut output) =
        pam::authenticate(&account, input, output, until, policy);
    wire::finish(&mut input, &mut output, outcome, until)
}
