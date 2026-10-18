//! Checked level inputs and fixed authored PCM only; no process or acoustic claim.
use crate::{PreviewLevel, fixed_pcm::fixed_preview_pcm};

const FRAMES: usize = 192_000;

fn pcm(percent: u64, muted: bool) -> Vec<u8> {
    let bytes = fixed_preview_pcm(PreviewLevel::new(percent, muted).unwrap());
    assert_eq!(bytes.len(), FRAMES * 4);
    assert!(samples(&bytes).all(f32::is_finite));
    bytes
}

fn samples(bytes: &[u8]) -> impl Iterator<Item = f32> + '_ {
    bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|x| f32::from_le_bytes(*x))
}

#[test]
fn preview_level_accepts_only_checked_percent_and_preserves_mute() {
    for muted in [false, true] {
        for percent in 0..=100 {
            let level = PreviewLevel::new(percent, muted).unwrap();
            assert_eq!(u64::from(level.percent()), percent);
            assert_eq!(level.muted(), muted);
        }
        for percent in [101, 256, u64::MAX] {
            assert!(PreviewLevel::new(percent, muted).is_none());
        }
    }
}

#[test]
fn full_level_is_bit_exact_original_four_second_pcm() {
    let bytes = pcm(100, false);
    // Frozen old fixed_pcm.rs Q15/131072 baseline, all 768000 F32LE bytes.
    let digest = bytes.iter().fold(0xcbf29ce484222325_u64, |h, byte| {
        (h ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    });
    assert_eq!(digest, 0x679275c32772c925_u64);
}

#[test]
fn half_level_scales_every_finite_frame_by_exactly_one_half() {
    let full = pcm(100, false);
    let half = pcm(50, false);
    for (original, scaled) in samples(&full).zip(samples(&half)) {
        assert_eq!(scaled.to_bits(), (original * 0.5).to_bits());
    }
}

#[test]
fn zero_and_mute_are_exact_silence_for_the_entire_four_seconds() {
    for (percent, muted) in [(0, false), (0, true), (50, true), (100, true)] {
        let bytes = pcm(percent, muted);
        assert_eq!(samples(&bytes).count(), FRAMES);
        assert!(samples(&bytes).all(|sample| sample == 0.0));
    }
}
