//! Private future-method requirements. Add as a child module of the existing
//! preview_process::tests, preserving all four existing tests byte-for-byte.
//! Missing set_live_muted is compile absence, never an executed behavioral RED.
//! Actual frontend/actor/PCM feature RED and these process-unit facts are separate.
use super::{Case, now, terminal};
use crate::{PreviewLevel, fixed_pcm::fixed_preview_pcm};
use pokelinux_power::clock::BootTime;
use std::fs;
use std::io::Write;
use std::thread;
use std::time::Duration;

const BYTES: usize = 4 * 48_000 * 4;

fn base() -> Vec<u8> {
    let bytes = fixed_preview_pcm(PreviewLevel::new(50, false).unwrap());
    assert_eq!(bytes.len(), BYTES);
    bytes
}
fn frame_after(sent: usize) -> usize {
    sent.div_ceil(4) * 4
}
fn intent_until(case: &Case) -> BootTime {
    now()
        .checked_after(Duration::from_secs(1))
        .unwrap()
        .min(case.work_until)
}
fn release(case: &Case, name: &str) {
    fs::write(case.root.join(name), b"one explicit fixture gate\n").unwrap();
}
fn capture_done(case: &mut Case) -> (bool, Vec<u8>) {
    case.marker("consumed", true);
    release(case, "go-2");
    let successful = case.finish();
    let captured = fs::read(case.root.join("captured.pcm")).unwrap();
    (successful, captured)
}

#[test]
fn live_mute_unmute_changes_only_future_complete_frames_of_same_owned_process() {
    let original = base();
    // Explicit controlled partial-sample seam through a REAL owned pipe.
    // This is not claimed to be a naturally observed production poll short write.
    assert_ne!(
        original[11], 0,
        "partial nonzero F32 high byte is discriminating"
    );
    let mut case = Case::start("capture-gated", original.clone(), 6000);
    case.marker("ready", false);
    let pid = case.process.pid();
    let work_until = case.work_until;
    let written = case
        .process
        .input
        .as_mut()
        .unwrap()
        .write(&original[..11])
        .unwrap();
    assert_eq!(written, 11, "actual empty-pipe write supplies the seam");
    // Account ONLY the actual returned byte count, as the real poll writer does.
    case.process.sent += written;
    let mute_begin = frame_after(case.process.sent);
    let mute_until = intent_until(&case);
    let muted = case.process.set_live_muted(true, mute_until).is_ok();
    let mute_timely = now() < mute_until;
    release(&case, "go-0");
    case.marker("stage-0", true);
    let unmute_begin = frame_after(case.process.sent);
    let still_same = case.process.pid() == pid
        && case.process.alive()
        && !terminal(&case.pidfd)
        && case.process.work_until == work_until;
    let unmute_until = intent_until(&case);
    let unmuted = case.process.set_live_muted(false, unmute_until).is_ok();
    let unmute_timely = now() < unmute_until;
    release(&case, "go-1");
    let (successful, captured) = capture_done(&mut case);
    let unchanged_work = case.process.work_until == work_until;
    let mut expected = original.clone();
    expected[mute_begin..unmute_begin].fill(0);
    let actual_partial_sample_preserved = captured.get(..12) == Some(&original[..12]);
    let real_silent_interval = unmute_begin > mute_begin
        && captured.get(mute_begin..unmute_begin) == Some(&expected[mute_begin..unmute_begin]);
    let real_unmuted_tail =
        unmute_begin < BYTES && captured.get(unmute_begin..) == Some(&original[unmute_begin..]);
    let exact = captured == expected;
    drop(case); // Actual wait, stderr EOF and pipe/pidfd closure precede assertion.
    assert!(
        muted
            && unmuted
            && mute_timely
            && unmute_timely
            && successful
            && still_same
            && unchanged_work
            && actual_partial_sample_preserved
            && real_silent_interval
            && real_unmuted_tail
            && exact,
        "same finite owner: preserve partial F32, mute only whole unsent frames, restore frozen50% tail"
    );
}

#[test]
fn initially_silent_local_owner_unmutes_to_its_frozen_half_level_base() {
    let original = base();
    let mut case = Case::start("capture-gated", original.clone(), 6000);
    case.marker("ready", false);
    let pid = case.process.pid();
    let work_until = case.work_until;
    assert_eq!(case.process.sent, 0);
    // Local initial-silence seam before any poll/write. This is not an executed
    // production start(level.muted=true) or initial-Muted-Play integration claim.
    let mute_until = intent_until(&case);
    let muted = case.process.set_live_muted(true, mute_until).is_ok();
    let mute_timely = now() < mute_until;
    release(&case, "go-0");
    case.marker("stage-0", true);
    let unmute_begin = frame_after(case.process.sent);
    let still_same = case.process.pid() == pid
        && case.process.alive()
        && !terminal(&case.pidfd)
        && case.process.work_until == work_until;
    let unmute_until = intent_until(&case);
    let unmuted = case.process.set_live_muted(false, unmute_until).is_ok();
    let unmute_timely = now() < unmute_until;
    release(&case, "go-1");
    let (successful, captured) = capture_done(&mut case);
    let unchanged_work = case.process.work_until == work_until;
    let mut expected = original.clone();
    expected[..unmute_begin].fill(0);
    let finite_half_tail =
        unmute_begin < BYTES && captured.get(unmute_begin..) == Some(&original[unmute_begin..]);
    let exact = captured == expected && captured.len() == BYTES;
    drop(case);
    assert!(
        muted
            && unmuted
            && mute_timely
            && unmute_timely
            && successful
            && still_same
            && unchanged_work
            && finite_half_tail
            && exact,
        "unmute restores the immutable50% base, never all-zero initial PCM or100%/new playback"
    );
}

#[test]
fn live_mute_refuses_retired_expired_work_and_expired_request_without_mutation() {
    let mut reports = Vec::new();
    for mode in ["retired", "work-expired", "request-expired"] {
        // The expired request cutoff is sampled BEFORE construction/readiness.
        let expired_request_until = now();
        let work_ms = if mode == "work-expired" { 250 } else { 6000 };
        let mut case = Case::start("capture-gated", base(), work_ms);
        case.marker("ready", false);
        let work_until = case.work_until;
        let before = case.process.pcm.clone();
        let until = match mode {
            "retired" => {
                case.process.retire();
                intent_until(&case)
            }
            "work-expired" => {
                while now() < work_until {
                    thread::sleep(Duration::from_millis(1));
                }
                work_until
            }
            _ => expired_request_until,
        };
        let refused = case.process.set_live_muted(true, until).is_err();
        let no_mutation = case.process.pcm == before && case.process.work_until == work_until;
        // Rejection is not a claim that capacity retired. Own actual cleanup.
        case.process.retire();
        let unsuccessful = !case.finish();
        drop(case);
        reports.push(refused && no_mutation && unsuccessful);
    }
    assert_eq!(
        reports,
        [true, true, true],
        "retired, original work expiry, and stale request cannot mutate future PCM or renew work"
    );
}

#[test]
fn live_mute_refuses_fully_queued_input_even_while_exact_child_is_alive() {
    let original = base();
    let mut case = Case::start("capture-gated", original.clone(), 6000);
    case.marker("ready", false);
    let pid = case.process.pid();
    let work_until = case.work_until;
    release(&case, "go-0");
    case.marker("stage-0", true);
    release(&case, "go-1");
    case.marker("consumed", true);
    let fully_queued_live = case.process.sent == BYTES
        && case.process.input.is_none()
        && case.process.pid() == pid
        && case.process.alive()
        && !terminal(&case.pidfd)
        && !case.process.settled();
    let until = intent_until(&case);
    let refused = case.process.set_live_muted(true, until).is_err();
    let no_mutation = case.process.pcm == original && case.process.work_until == work_until;
    release(&case, "go-2");
    let successful = case.finish();
    let captured = fs::read(case.root.join("captured.pcm")).unwrap();
    drop(case);
    assert!(
        fully_queued_live && refused && no_mutation && successful && captured == original,
        "no future complete frame means refusal, not replay/process replacement or mutation of queued sound"
    );
}

#[test]
fn bounded_pipe_backpressure_limits_already_queued_tone_before_live_mute() {
    let original = base();
    let mut case = Case::start("capture-gated", original.clone(), 6000);
    case.marker("ready", false);
    // The real child is gated before reading. Repeated production polls must
    // backpressure rather than queue a large fraction of the finite asset.
    for _ in 0..32 {
        assert!(case.process.poll().is_none());
    }
    let capacity = rustix::pipe::fcntl_getpipe_size(case.process.input.as_ref().unwrap()).unwrap();
    let queued = case.process.sent;
    let first_muted = frame_after(queued);
    let bounded = capacity <= 16_384 && queued <= 16_384 && queued > 0;
    let mute_until = intent_until(&case);
    let accepted = case.process.set_live_muted(true, mute_until).is_ok();
    release(&case, "go-0");
    case.marker("stage-0", true);
    release(&case, "go-1");
    let (successful, captured) = capture_done(&mut case);
    let prefix_preserved = captured.get(..first_muted) == Some(&original[..first_muted]);
    let future_quiet =
        captured.len() == BYTES && captured[first_muted..].iter().all(|value| *value == 0);
    drop(case);
    assert!(
        bounded && accepted && successful && prefix_preserved && future_quiet,
        "actual retained pipe capacity={capacity} queued={queued}: bound lookahead, preserve queued prefix, mute the future feed"
    );
}
