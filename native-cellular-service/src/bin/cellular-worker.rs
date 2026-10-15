//! One owned native Data attempt. All potentially blocking Binder calls stay here.
use pokelinux_cellular_boundary::IRadioData;
use pokelinux_cellular_owner::DataOwner;
use pokelinux_cellular_service::{config::Config, now_ns, output, wire::Frame};
use pokelinux_process_owner::bootstrap::{BoundParent, WORKER_SELECTOR};
use rsbinder::{
    FromIBinder, ProcessState, Proxy, Strong,
    hub::android_16::{BpServiceManager, IServiceManager},
};
use std::{io, path::Path, time::Duration};
fn fatal() -> ! {
    std::process::exit(70)
}
fn must<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| fatal())
}
fn before(until: u64) {
    if now_ns() >= until {
        fatal();
    }
}
fn run() -> io::Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 7
        || args[0] != WORKER_SELECTOR
        || args[1] != "--config"
        || args[3] != "--until-ns"
        || args[5] != "--config-sha256"
        || !Path::new(&args[2]).is_absolute()
    {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let until: u64 = args[4].parse().map_err(|_| io::ErrorKind::InvalidInput)?;
    // Main thread, final credentials, before config/Binder/thread creation.
    let parent = BoundParent::from_stdin()?;
    output::configure()?;
    if args[6].len() != 64
        || !args[6]
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let mut identity = [0u8; 32];
    for (index, byte) in identity.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&args[6][index * 2..index * 2 + 2], 16)
            .map_err(|_| io::ErrorKind::InvalidInput)?;
    }
    let config = Config::load_matching(Path::new(&args[2]), identity)?;
    if now_ns() >= until {
        return Err(io::ErrorKind::TimedOut.into());
    }
    body(parent, config, until)
}
fn body(parent: BoundParent, config: Config, until: u64) -> ! {
    // Fatal paths terminate the owned process directly. They must not unwind
    // through DataOwner::drop/unlink_to_death or another potentially blocked SDK call.
    let process = must(ProcessState::init(
        must(config.device.to_str().ok_or(())),
        3,
    ));
    ProcessState::start_thread_pool();
    let context = must(process.context_object());
    if !context.is_remote() || context.descriptor() != "android.os.IServiceManager" {
        fatal();
    }
    let manager = must(BpServiceManager::from_binder(context).ok_or(()));
    let binder = must(must(manager.checkService(&config.service)).ok_or(()));
    if !binder.is_remote() || binder.descriptor() != "android.hardware.radio.data.IRadioData" {
        fatal();
    }
    let endpoint: Strong<dyn IRadioData> = must(FromIBinder::try_from(binder));
    if must(endpoint.getInterfaceVersion()) != 3
        || must(endpoint.getInterfaceHash()) != "cd8913a3f9d39f1cc0a5fcf9e90257be94ec38df"
    {
        fatal();
    }
    before(until);
    let serial = config.setup.serial;
    let mut owner = must(DataOwner::new(
        endpoint,
        config.expected_callback_uid,
        serial,
        until,
    ));
    let mut revision = 1;
    let initial = must(Frame::from_owner(
        parent.identity(),
        revision,
        serial,
        until,
        now_ns(),
        &owner,
        None,
    ));
    let _ = must(output::publish(&must(initial.encode())));
    let (response, indication) = owner.callbacks();
    before(until);
    must(owner.endpoint().setResponseFunctions(response, indication));
    must(owner.mark_may_have_submitted());
    revision += 1;
    let pending = must(Frame::from_owner(
        parent.identity(),
        revision,
        serial,
        until,
        now_ns(),
        &owner,
        None,
    ));
    must(output::publish_required(&must(pending.encode())));
    let setup = config.setup;
    // No deadline renewal and no rollback from conservative uncertainty.
    before(until);
    let mut fault = owner
        .endpoint()
        .setupDataCall(
            serial,
            setup.access_network,
            &setup.profile,
            setup.roaming_allowed,
            setup.reason,
            &setup.addresses,
            &setup.dnses,
            setup.pdu_session_id,
            setup.slice.as_ref(),
            setup.match_all_rule_allowed,
        )
        .err()
        .map(|_| "setup-transport".to_string());
    loop {
        if fault.is_none()
            && let Err(error) = owner.poll()
            && pokelinux_cellular_service::worker_policy::terminal_poll_error(error)
        {
            fault = Some("data-owner-terminal".to_string());
        }
        if fault.is_some() {
            owner.stop();
        }
        revision = must(revision.checked_add(1).ok_or(()));
        let frame = must(Frame::from_owner(
            parent.identity(),
            revision,
            serial,
            until,
            now_ns(),
            &owner,
            fault.clone(),
        ));
        let _ = must(output::publish(&must(frame.encode())));
        std::thread::sleep(Duration::from_millis(20));
    }
}
fn main() {
    std::process::exit(if run().is_ok() { 0 } else { 70 });
}
