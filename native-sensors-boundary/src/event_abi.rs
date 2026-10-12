//! Audited ABI leaf: FMQ copies fixed-size original generated NDK Event objects.
//! Rust maps opaque bytes only, never the generated Rust enum/object representation.
//! The pinned C++ compiler supplies the quantum; its Event is @FixedSize/standard-layout.
//! The byte-array type has no padding or invalid bit patterns, and aligns to eight bytes.
//! Decoding checks the supported tag/type and reads scalars from original-probed offsets.
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../artifacts/sensors-boundary/event-size.rs"
));
#[repr(C, align(8))]
#[derive(Clone, Copy, Debug)]
pub struct RawEvent {
    pub bytes: [u8; EVENT_SIZE],
}
impl Default for RawEvent {
    fn default() -> Self {
        Self {
            bytes: [0; EVENT_SIZE],
        }
    }
}
const _: () =
    assert!(EVENT_SIZE.is_multiple_of(8) && std::mem::size_of::<RawEvent>() == EVENT_SIZE);
#[allow(
    unsafe_code,
    reason = "All bit patterns of an aligned opaque byte array are valid; size is pinned to original NDK fixed-size Event, with no Rust enum or padding."
)]
unsafe impl rsbinder::fmq::Element for RawEvent {}
pub fn decode(event: &RawEvent, abi: &[i32]) -> Result<pokelinux_sensors::Sample, &'static str> {
    if abi.len() != 11 || abi[0] as usize != EVENT_SIZE || abi[1] != 8 || abi[6] != 1 {
        return Err("unqualified Event ABI");
    }
    let read4 = |index: usize| -> Result<[u8; 4], &'static str> {
        let offset = usize::try_from(abi[index]).map_err(|_| "negative offset")?;
        event
            .bytes
            .get(offset..offset + 4)
            .ok_or("offset outside Event")?
            .try_into()
            .map_err(|_| "scalar shape")
    };
    if i32::from_le_bytes(read4(3)?) != 77
        || i32::from_le_bytes(read4(4)?) != 1
        || *event
            .bytes
            .get(usize::try_from(abi[5]).map_err(|_| "negative tag offset")?)
            .ok_or("tag outside Event")?
            != 0
    {
        return Err("unsupported sensor/type/payload");
    }
    let offset = usize::try_from(abi[2]).map_err(|_| "negative timestamp offset")?;
    let timestamp_ns = i64::from_le_bytes(
        event
            .bytes
            .get(offset..offset + 8)
            .ok_or("timestamp outside Event")?
            .try_into()
            .map_err(|_| "timestamp shape")?,
    );
    let acceleration = [
        f32::from_le_bytes(read4(7)?),
        f32::from_le_bytes(read4(8)?),
        f32::from_le_bytes(read4(9)?),
    ];
    let status = *event
        .bytes
        .get(usize::try_from(abi[10]).map_err(|_| "negative status offset")?)
        .ok_or("status outside Event")? as i8;
    if !(-1..=3).contains(&status)
        || timestamp_ns <= 0
        || acceleration.iter().any(|n| !n.is_finite())
    {
        return Err("malformed sample");
    }
    Ok(pokelinux_sensors::Sample {
        timestamp_ns,
        acceleration,
    })
}
