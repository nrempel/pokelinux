//! Stateless format operation planning. Native adapters supply observations only.
//!
//! A plan requests one primitive attempt; it proves no native success or authority.
use std::mem::size_of;

const INVALID: i32 = 1;
const CAPACITY: i32 = 2;
const ABI: u32 = 2;
const MOD_INVALID: u64 = 0x00ff_ffff_ffff_ffff;
const UNKNOWN: u32 = 0;
const OK: u32 = 1;
const ERROR: u32 = 2;
const PRESENT: u32 = 3;
const REFUSED: u32 = 1;
const PREPARE_ATTEMPT: u32 = 2;
const IMPLICIT: u32 = 1;
const EXPLICIT: u32 = 2;
const EMPTY_NATIVE: u32 = 1;
const EMPTY_MODIFIERS: u32 = 2;
const EXPLICIT_MODIFIER_INVALID: u32 = 3;
const QUERY_UNKNOWN: u32 = 4;
const QUERY_ERROR: u32 = 5;
const PROPERTY_ABSENT: u32 = 6;
const INCONSISTENT: u32 = 8;
const NO_ROUTE: u32 = 9;
const NO_EXPLICIT_MATCH: u32 = 10;
const BACKEND_UNSUPPORTED: u32 = 11;
const DEVICE_MISMATCH: u32 = 12;
const OPTIONS_UNSUPPORTED: u32 = 13;
const NO_IMPLICIT_MATCH: u32 = 14;
const NATIVE: u32 = 1;
const CAP: u32 = 2;
const ROUTE: u32 = 3;
const PLANE: u32 = 4;
const TYPE: u32 = 5;
const IN_FORMATS: u32 = 6;
const BLOB: u32 = 7;
const BACKEND: u32 = 8;
const ALLOCATOR_DEVICE: u32 = 9;
const KMS_DEVICE: u32 = 10;
const DEVICE: u32 = 11;
const OPTIONS: u32 = 12;
const CONTEXT: u32 = 13;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Context {
    pub session_generation: u64,
    pub discovery_sequence: u64,
    pub device_lease_id: u64,
    pub device_generation: u64,
    pub rdev: u64,
    pub output_id: u64,
    pub output_generation: u64,
    pub connector_id: u32,
    pub reserved: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NativeFormat {
    pub drm_format: u32,
    pub modifier_first: u32,
    pub modifier_count: u32,
    pub reserved: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ExplicitPair {
    pub drm_format: u32,
    pub reserved: u32,
    pub modifier: u64,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Property {
    pub state: u32,
    pub error_number: i32,
    pub value: u64,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Capability {
    pub state: u32,
    pub return_value: i32,
    pub error_number: i32,
    pub reserved: u32,
    pub value: u64,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Plane {
    pub plane_id: u32,
    pub current_crtc_id: u32,
    pub possible_crtcs: u32,
    pub query_state: u32,
    pub query_errno: i32,
    pub legacy_first: u32,
    pub legacy_count: u32,
    pub pair_first: u32,
    pub pair_count: u32,
    pub blob_state: u32,
    pub blob_errno: i32,
    pub blob_length: u32,
    pub type_property: Property,
    pub in_formats: Property,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Call {
    pub state: u32,
    pub return_value: i32,
    pub error_number: i32,
    pub reserved: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Facts {
    pub abi_version: u32,
    pub struct_size: u32,
    pub context: Context,
    pub cap: Capability,
    pub route_state: u32,
    pub route_errno: i32,
    pub current_encoder_state: u32,
    pub current_encoder_errno: i32,
    pub current_encoder_id: u32,
    pub current_crtc_id: u32,
    pub native_format_count: u32,
    pub modifier_count: u32,
    pub plane_count: u32,
    pub legacy_format_count: u32,
    pub explicit_pair_count: u32,
    pub crtc_count: u32,
    pub encoder_count: u32,
    pub reserved_count: u32,
    pub native_formats: *const NativeFormat,
    pub modifiers: *const u64,
    pub planes: *const Plane,
    pub legacy_formats: *const u32,
    pub explicit_pairs: *const ExplicitPair,
    pub crtc_ids: *const u32,
    pub encoder_possible_crtcs: *const u32,
    pub reserved: [u64; 2],
    pub allocator_kind: u32,
    pub output_backend_kind: u32,
    pub width: u32,
    pub height: u32,
    pub scanout: u32,
    pub cursor: u32,
    pub multigpu: u32,
    pub ordinary_reserved: u32,
    pub allocator_device_query: Call,
    pub kms_device_query: Call,
    pub allocator_rdev: u64,
    pub kms_rdev: u64,
    pub devices_equal: i32,
    pub devices_equal_count: u32,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Selection {
    pub abi_version: u32,
    pub struct_size: u32,
    pub decision: u32,
    pub reason: u32,
    pub context: Context,
    pub plane_id: u32,
    pub drm_format: u32,
    pub refusal_source: u32,
    pub buffer_mode: u32,
    pub modifier: u64,
    pub width: u32,
    pub height: u32,
}

// Empty borrowed arrays require null. The caller still guarantees live memory;
// these checks detect shape/alignment/overflow, not arbitrary address validity.
fn region<T>(pointer: *const T, count: usize) -> Result<(usize, usize), i32> {
    if count == 0 {
        return if pointer.is_null() {
            Ok((0, 0))
        } else {
            Err(INVALID)
        };
    }
    if pointer.is_null() || !pointer.is_aligned() {
        return Err(INVALID);
    }
    let bytes = count.checked_mul(size_of::<T>()).ok_or(INVALID)?;
    if bytes > isize::MAX as usize {
        return Err(INVALID);
    }
    let first = pointer as usize;
    Ok((first, first.checked_add(bytes).ok_or(INVALID)?))
}
fn overlaps(a: (usize, usize), b: (usize, usize)) -> bool {
    a.0 < b.1 && b.0 < a.1
}
unsafe fn borrowed<'a, T>(pointer: *const T, count: u32) -> &'a [T] {
    if count == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(pointer, count as usize) }
    }
}
fn span(first: u32, count: u32, total: u32) -> bool {
    first <= total && count <= total - first
}
fn read_state(state: u32, error: i32) -> bool {
    error >= 0
        && match state {
            UNKNOWN | OK => error == 0,
            ERROR => true,
            _ => false,
        }
}
fn property(value: Property) -> bool {
    value.error_number >= 0
        && match value.state {
            UNKNOWN | OK => value.error_number == 0 && value.value == 0,
            ERROR => value.value == 0,
            PRESENT => value.error_number == 0,
            _ => false,
        }
}
fn context(value: Context) -> bool {
    value.session_generation != 0
        && value.discovery_sequence != 0
        && value.device_lease_id != 0
        && value.device_generation != 0
        && value.output_id != 0
        && value.output_generation != 0
        && value.connector_id != 0
        && value.reserved == 0
}
fn cap(value: Capability) -> bool {
    value.reserved == 0
        && value.error_number >= 0
        && match value.state {
            UNKNOWN => value.return_value == 0 && value.error_number == 0 && value.value == 0,
            OK => value.return_value == 0 && value.error_number == 0,
            ERROR => value.return_value != 0 && value.value == 0,
            _ => false,
        }
}
fn call(value: Call) -> bool {
    value.reserved == 0
        && value.error_number >= 0
        && match value.state {
            UNKNOWN | OK => value.return_value == 0 && value.error_number == 0,
            ERROR => value.return_value != 0,
            _ => false,
        }
}
fn ordinary(f: &Facts) -> bool {
    f.ordinary_reserved == 0
        && (1..=16384).contains(&f.width)
        && (1..=16384).contains(&f.height)
        && f.scanout <= 1
        && f.cursor <= 1
        && f.multigpu <= 1
        && call(f.allocator_device_query)
        && call(f.kms_device_query)
        && match f.devices_equal_count {
            0 => f.devices_equal == 0,
            1 => f.allocator_device_query.state == OK && f.kms_device_query.state == OK,
            _ => false,
        }
}
fn encoder(f: &Facts) -> bool {
    f.current_encoder_errno >= 0
        && match f.current_encoder_state {
            UNKNOWN => f.current_encoder_errno == 0 && f.current_crtc_id == 0,
            OK => {
                f.current_encoder_id == 0 && f.current_crtc_id == 0 && f.current_encoder_errno == 0
            }
            ERROR => f.current_encoder_id != 0 && f.current_crtc_id == 0,
            PRESENT => f.current_encoder_id != 0 && f.current_encoder_errno == 0,
            _ => false,
        }
}
struct Views<'a> {
    native: &'a [NativeFormat],
    modifiers: &'a [u64],
    planes: &'a [Plane],
    legacy: &'a [u32],
    pairs: &'a [ExplicitPair],
    crtcs: &'a [u32],
    encoders: &'a [u32],
}
impl Views<'_> {
    fn valid(&self, f: &Facts) -> bool {
        if !context(f.context)
            || !ordinary(f)
            || !cap(f.cap)
            || !encoder(f)
            || !read_state(f.route_state, f.route_errno)
            || f.reserved_count != 0
            || f.reserved != [0; 2]
            || self.native.iter().any(|n| {
                n.drm_format == 0
                    || n.reserved != 0
                    || !span(n.modifier_first, n.modifier_count, f.modifier_count)
            })
            || self
                .pairs
                .iter()
                .any(|p| p.drm_format == 0 || p.reserved != 0)
            || self.legacy.contains(&0)
            || self
                .crtcs
                .iter()
                .enumerate()
                .any(|(i, id)| *id == 0 || self.crtcs[..i].contains(id))
        {
            return false;
        }
        for (i, p) in self.planes.iter().enumerate() {
            if p.plane_id == 0
                || self.planes[..i].iter().any(|q| q.plane_id == p.plane_id)
                || !read_state(p.query_state, p.query_errno)
                || !read_state(p.blob_state, p.blob_errno)
                || !property(p.type_property)
                || !property(p.in_formats)
                || !span(p.legacy_first, p.legacy_count, f.legacy_format_count)
                || !span(p.pair_first, p.pair_count, f.explicit_pair_count)
            {
                return false;
            }
            if p.query_state != OK
                && (p.current_crtc_id != 0
                    || p.possible_crtcs != 0
                    || p.legacy_first != 0
                    || p.legacy_count != 0)
            {
                return false;
            }
            if p.blob_state == OK {
                if !(1..=1_048_576).contains(&p.blob_length) {
                    return false;
                }
            } else if p.blob_length != 0 || p.pair_first != 0 || p.pair_count != 0 {
                return false;
            }
            if (p.in_formats.state != PRESENT || p.in_formats.value == 0) && p.blob_state != UNKNOWN
            {
                return false;
            }
        }
        true
    }
}
fn refuse(context: Context, reason: u32, source: u32, plane_id: u32) -> Selection {
    Selection {
        abi_version: ABI,
        struct_size: size_of::<Selection>() as u32,
        decision: REFUSED,
        reason,
        context,
        plane_id,
        refusal_source: source,
        ..Selection::default()
    }
}
fn query_reason(state: u32) -> u32 {
    match state {
        UNKNOWN => QUERY_UNKNOWN,
        OK => PROPERTY_ABSENT,
        _ => QUERY_ERROR,
    }
}
fn choose(f: &Facts, v: &Views<'_>) -> Selection {
    let reject = |reason, source, plane| refuse(f.context, reason, source, plane);
    if f.allocator_kind == UNKNOWN || f.output_backend_kind == UNKNOWN {
        return reject(QUERY_UNKNOWN, BACKEND, 0);
    }
    if f.allocator_kind != 1 || f.output_backend_kind != 1 {
        return reject(BACKEND_UNSUPPORTED, BACKEND, 0);
    }
    if f.scanout != 1 || f.cursor != 0 || f.multigpu != 0 {
        return reject(OPTIONS_UNSUPPORTED, OPTIONS, 0);
    }
    for (query, source) in [
        (f.allocator_device_query, ALLOCATOR_DEVICE),
        (f.kms_device_query, KMS_DEVICE),
    ] {
        if query.state != OK {
            return reject(query_reason(query.state), source, 0);
        }
    }
    if f.devices_equal_count == 0 {
        return reject(QUERY_UNKNOWN, DEVICE, 0);
    }
    match f.devices_equal {
        0 => return reject(DEVICE_MISMATCH, DEVICE, 0),
        1 => (),
        _ => return reject(INCONSISTENT, DEVICE, 0),
    }
    if f.kms_rdev != f.context.rdev {
        return reject(INCONSISTENT, CONTEXT, 0);
    }
    let Some(first) = v.native.first() else {
        return reject(EMPTY_NATIVE, NATIVE, 0);
    };
    if f.cap.state != OK {
        return reject(query_reason(f.cap.state), CAP, 0);
    }
    let buffer_mode = match f.cap.value {
        0 => IMPLICIT,
        1 => EXPLICIT,
        _ => return reject(INCONSISTENT, CAP, 0),
    };
    let modifier = if buffer_mode == EXPLICIT {
        if first.modifier_count == 0 {
            return reject(EMPTY_MODIFIERS, NATIVE, 0);
        }
        let modifier = v.modifiers[first.modifier_first as usize];
        if modifier == MOD_INVALID {
            return reject(EXPLICIT_MODIFIER_INVALID, NATIVE, 0);
        }
        modifier
    } else {
        // The implicit mode is a request, never a layout inferred from raw modifiers.
        0
    };
    if f.route_state != OK {
        return reject(query_reason(f.route_state), ROUTE, 0);
    }
    if matches!(f.current_encoder_state, UNKNOWN | ERROR) {
        return reject(query_reason(f.current_encoder_state), ROUTE, 0);
    }
    let real_mask = if v.crtcs.len() == 32 {
        u32::MAX
    } else {
        (1u32 << v.crtcs.len()) - 1
    };
    let mut compatible = v.encoders.iter().fold(0, |mask, value| mask | value) & real_mask;
    if f.current_encoder_state == PRESENT && f.current_crtc_id != 0 {
        compatible &= v
            .crtcs
            .iter()
            .position(|id| *id == f.current_crtc_id)
            .map_or(0, |i| 1u32 << i);
    }
    if compatible == 0 {
        return reject(NO_ROUTE, ROUTE, 0);
    }
    for p in v.planes {
        if p.query_state != OK {
            return reject(query_reason(p.query_state), PLANE, p.plane_id);
        }
        if p.possible_crtcs & compatible == 0 {
            continue;
        }
        let legacy = &v.legacy[p.legacy_first as usize..(p.legacy_first + p.legacy_count) as usize];
        if !legacy.contains(&first.drm_format) {
            continue;
        }
        if p.type_property.state != PRESENT {
            return reject(query_reason(p.type_property.state), TYPE, p.plane_id);
        }
        if p.type_property.value != 1 {
            continue;
        }
        if !(p.in_formats.state == OK && buffer_mode == IMPLICIT) {
            if p.in_formats.state != PRESENT {
                return reject(query_reason(p.in_formats.state), IN_FORMATS, p.plane_id);
            }
            if p.in_formats.value == 0 {
                return reject(INCONSISTENT, IN_FORMATS, p.plane_id);
            }
            if p.blob_state != OK {
                return reject(query_reason(p.blob_state), BLOB, p.plane_id);
            }
            if buffer_mode == EXPLICIT {
                let pairs = &v.pairs[p.pair_first as usize..(p.pair_first + p.pair_count) as usize];
                if !pairs
                    .iter()
                    .any(|pair| pair.drm_format == first.drm_format && pair.modifier == modifier)
                {
                    continue;
                }
            }
        }
        return Selection {
            abi_version: ABI,
            struct_size: size_of::<Selection>() as u32,
            decision: PREPARE_ATTEMPT,
            context: f.context,
            plane_id: p.plane_id,
            drm_format: first.drm_format,
            buffer_mode,
            modifier,
            width: f.width,
            height: f.height,
            ..Selection::default()
        };
    }
    reject(
        if buffer_mode == IMPLICIT {
            NO_IMPLICIT_MATCH
        } else {
            NO_EXPLICIT_MATCH
        },
        PLANE,
        0,
    )
}

/// Return one owned operation plan without retaining any caller memory.
///
/// # Safety
/// Advertised nonnull ranges must be live for the call, read-only for inputs and
/// writable for result. Shape/alignment/overlap errors are detected before writes;
/// no general validity check of arbitrary addresses is possible. See the C header.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_format_select_v2(facts: *const Facts, result: *mut Selection) -> i32 {
    match unsafe { select(facts, result) } {
        Ok(value) => {
            unsafe {
                result.write(value);
            }
            0
        }
        Err(status) => status,
    }
}
unsafe fn select(facts: *const Facts, result: *mut Selection) -> Result<Selection, i32> {
    let input = region(facts, 1)?;
    let output = region(result.cast_const(), 1)?;
    if overlaps(input, output) {
        return Err(INVALID);
    }
    let f = unsafe { facts.read() };
    if f.abi_version != ABI || f.struct_size as usize != size_of::<Facts>() {
        return Err(INVALID);
    }
    if f.native_format_count > 128
        || f.modifier_count > 4096
        || f.plane_count > 128
        || f.legacy_format_count > 4096
        || f.explicit_pair_count > 4096
        || f.crtc_count > 32
        || f.encoder_count > 128
    {
        return Err(CAPACITY);
    }
    let ranges = [
        region(f.native_formats, f.native_format_count as usize)?,
        region(f.modifiers, f.modifier_count as usize)?,
        region(f.planes, f.plane_count as usize)?,
        region(f.legacy_formats, f.legacy_format_count as usize)?,
        region(f.explicit_pairs, f.explicit_pair_count as usize)?,
        region(f.crtc_ids, f.crtc_count as usize)?,
        region(f.encoder_possible_crtcs, f.encoder_count as usize)?,
    ];
    if ranges.into_iter().any(|range| overlaps(range, output)) {
        return Err(INVALID);
    }
    let views = Views {
        native: unsafe { borrowed(f.native_formats, f.native_format_count) },
        modifiers: unsafe { borrowed(f.modifiers, f.modifier_count) },
        planes: unsafe { borrowed(f.planes, f.plane_count) },
        legacy: unsafe { borrowed(f.legacy_formats, f.legacy_format_count) },
        pairs: unsafe { borrowed(f.explicit_pairs, f.explicit_pair_count) },
        crtcs: unsafe { borrowed(f.crtc_ids, f.crtc_count) },
        encoders: unsafe { borrowed(f.encoder_possible_crtcs, f.encoder_count) },
    };
    if !views.valid(&f) {
        return Err(INVALID);
    }
    Ok(choose(&f, &views))
}
