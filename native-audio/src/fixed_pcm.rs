//! Proposed authored fixed 440Hz preview: 192000 mono F32LE frames, finite 4s.
//! Integer-indexed Q15 lookup and exact power-of-two scaling avoid platform sin.
//! No runtime file, node, frequency or sample input comes from an application.
pub(crate) fn fixed_preview_pcm(level: crate::PreviewLevel) -> Vec<u8> {
    const SINE: [i16; 64] = [
        0, 3212, 6393, 9512, 12539, 15446, 18204, 20787, 23170, 25330, 27245, 28898, 30273, 31357,
        32138, 32610, 32767, 32610, 32138, 31357, 30273, 28898, 27245, 25330, 23170, 20787, 18204,
        15446, 12539, 9512, 6393, 3212, 0, -3212, -6393, -9512, -12539, -15446, -18204, -20787,
        -23170, -25330, -27245, -28898, -30273, -31357, -32138, -32610, -32767, -32610, -32138,
        -31357, -30273, -28898, -27245, -25330, -23170, -20787, -18204, -15446, -12539, -9512,
        -6393, -3212,
    ];
    let gain = level.gain();
    let mut bytes = Vec::with_capacity(4 * 48_000 * 4);
    for frame in 0..192_000_u64 {
        let index = ((frame * 440 * 64 / 48_000) % 64) as usize;
        let sample = if gain == 0.0 {
            0.0
        } else {
            f32::from(SINE[index]) / 131_072.0 * gain
        };
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    bytes
}
