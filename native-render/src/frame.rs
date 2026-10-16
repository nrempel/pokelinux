//! Owned immutable value frame. Borrowed C input storage never survives capture.
//! A frame/visitor is payload custody only, not a renderer or output capability.
use crate::{Error, ShmDescription, Snapshot};
use std::ffi::c_void;
use std::mem::{align_of, size_of};
use std::ptr;

pub const PR_FRAME_ABI_V1: u32 = 1;
pub const PR_FRAME_MAX_DRAWS: usize = 256;
pub const PR_FRAME_MAX_RESOURCES: usize = 256;
pub const PR_FRAME_MAX_RECTS: usize = 4096;
pub const PR_FRAME_MAX_METADATA_BYTES: usize = 1024 * 1024;
pub const PR_DRAW_CLEAR: u32 = 0;
pub const PR_DRAW_RECT: u32 = 1;
pub const PR_DRAW_SHM: u32 = 2;
pub const PR_FRAME_COMMIT: u32 = 1;
pub const PR_FRAME_FULL_REPAINT: u32 = 2;
pub const PR_TARGET_SHADER_CM_AVAILABLE: u32 = 1;
pub const PR_MAT_NEAREST: u32 = 1;
pub const PR_MAT_BLEND: u32 = 2;
pub const PR_MAT_RGBX: u32 = 4;
pub const PR_MAT_TINT: u32 = 8;
pub const PR_MAT_DISCARD_ACTIVE: u32 = 16;
pub const PR_MAT_DISCARD_OPAQUE: u32 = 32;
pub const PR_MAT_DISCARD_ALPHA: u32 = 64;
pub const PR_CM_ENABLED: u32 = 1;
pub const PR_CM_TONEMAP: u32 = 2;
pub const PR_CM_SDR_MOD: u32 = 4;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameRectV1 {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FrameBoxV1 {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub rotation: f64,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FrameTargetV1 {
    pub output_id: u64,
    pub output_generation: u64,
    pub width: u32,
    pub height: u32,
    pub transformed_width: u32,
    pub transformed_height: u32,
    pub output_transform: u32,
    pub flags: u32,
    pub scale: f64,
    pub target_projection: [f32; 9],
    pub scale_matrix: [f32; 9],
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FrameColorManagementV1 {
    pub flags: u32,
    pub source_tf: u32,
    pub target_tf: u32,
    pub tonemap_mode: u32,
    pub src_tf_min: f32,
    pub src_tf_max: f32,
    pub dst_tf_min: f32,
    pub dst_tf_max: f32,
    pub src_ref_luminance: f32,
    pub dst_ref_luminance: f32,
    pub max_luminance: f32,
    pub dst_max_luminance: f32,
    pub convert_matrix: [f32; 9],
    pub dst_primaries_xyz: [f32; 9],
    pub sdr_saturation: f32,
    pub sdr_brightness: f32,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameMaterialV1 {
    pub alpha: f32,
    pub rounding_px: f32,
    pub rounding_power: f32,
    pub discard_opacity: f32,
    pub flags: u32,
    pub wrap_x: u32,
    pub wrap_y: u32,
    pub reserved: u32,
    pub tint_rgb: [f32; 3],
    pub cm: FrameColorManagementV1,
}
impl Default for FrameMaterialV1 {
    fn default() -> Self {
        Self {
            alpha: 1.0,
            rounding_px: 0.0,
            rounding_power: 2.0,
            discard_opacity: 0.0,
            flags: PR_MAT_BLEND,
            wrap_x: 0,
            wrap_y: 0,
            reserved: 0,
            tint_rgb: [1.0; 3],
            cm: FrameColorManagementV1 {
                sdr_saturation: 1.0,
                sdr_brightness: 1.0,
                ..Default::default()
            },
        }
    }
}
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameDrawV1 {
    pub kind: u32,
    pub resource_index: u32,
    pub scissor_first: u32,
    pub scissor_count: u32,
    pub box_: FrameBoxV1,
    pub rounding_box: FrameBoxV1,
    pub projection: [f32; 9],
    pub uv: [f32; 4],
    pub converted_color: [f32; 4],
    pub color_srgb: [f32; 4],
    pub material: FrameMaterialV1,
    pub surface_id: u64,
    pub commit_sequence: u64,
}
impl Default for FrameDrawV1 {
    fn default() -> Self {
        Self {
            kind: PR_DRAW_CLEAR,
            resource_index: u32::MAX,
            scissor_first: 0,
            scissor_count: 0,
            box_: Default::default(),
            rounding_box: Default::default(),
            projection: [0.0; 9],
            uv: [0.0, 0.0, 1.0, 1.0],
            converted_color: [0.0; 4],
            color_srgb: [0.0; 4],
            material: Default::default(),
            surface_id: 0,
            commit_sequence: 0,
        }
    }
}
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FrameResourceBorrowV1 {
    pub description: ShmDescription,
    pub snapshot: *const Snapshot,
}
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FrameBorrowV1 {
    pub abi_version: u32,
    pub struct_size: u32,
    pub flags: u32,
    pub reserved: u32,
    pub scene_sequence: u64,
    pub security_generation: u64,
    pub target: FrameTargetV1,
    pub damage_first: u32,
    pub damage_count: u32,
    pub final_damage_first: u32,
    pub final_damage_count: u32,
    pub draw_count: u32,
    pub resource_count: u32,
    pub rect_count: u32,
    pub reserved2: u32,
    pub draws: *const FrameDrawV1,
    pub resources: *const FrameResourceBorrowV1,
    pub rects: *const FrameRectV1,
}
pub type FrameCMV1 = FrameColorManagementV1;
pub type PrFrameRectV1 = FrameRectV1;
pub type PrFrameBoxV1 = FrameBoxV1;
pub type PrFrameTargetV1 = FrameTargetV1;
pub type PrFrameCMV1 = FrameColorManagementV1;
pub type PrFrameMaterialV1 = FrameMaterialV1;
pub type PrFrameDrawV1 = FrameDrawV1;
pub type PrFrameResourceBorrowV1 = FrameResourceBorrowV1;
pub type PrFrameBorrowV1 = FrameBorrowV1;
pub type FrameViewFnV1 = unsafe extern "C" fn(*const FrameBorrowV1, *mut c_void) -> i32;

/// Immutable copied values and Arc-cloned Snapshot values; no borrowed C pointers.
pub struct Frame {
    flags: u32,
    scene_sequence: u64,
    security_generation: u64,
    target: FrameTargetV1,
    damage_first: u32,
    damage_count: u32,
    final_damage_first: u32,
    final_damage_count: u32,
    draws: Vec<FrameDrawV1>,
    resources: Vec<Snapshot>,
    rects: Vec<FrameRectV1>,
}
impl Frame {
    pub fn flags(&self) -> u32 {
        self.flags
    }
    pub fn scene_sequence(&self) -> u64 {
        self.scene_sequence
    }
    pub fn security_generation(&self) -> u64 {
        self.security_generation
    }
    pub fn target(&self) -> &FrameTargetV1 {
        &self.target
    }
    /// Snapshot handles and value-array pointers are valid only inside `visitor`.
    pub fn with_borrow<R>(&self, visitor: impl FnOnce(&FrameBorrowV1) -> R) -> Result<R, Error> {
        let mut resources = Vec::new();
        resources
            .try_reserve_exact(self.resources.len())
            .map_err(|_| Error::Allocation)?;
        metadata_bytes(
            self.draws.capacity(),
            self.rects.capacity(),
            self.resources.capacity(),
            resources.capacity(),
        )?;
        for snapshot in &self.resources {
            resources.push(FrameResourceBorrowV1 {
                description: snapshot.description(),
                snapshot: ptr::from_ref(snapshot),
            });
        }
        let view = FrameBorrowV1 {
            abi_version: PR_FRAME_ABI_V1,
            struct_size: size_of::<FrameBorrowV1>() as u32,
            flags: self.flags,
            reserved: 0,
            scene_sequence: self.scene_sequence,
            security_generation: self.security_generation,
            target: self.target,
            damage_first: self.damage_first,
            damage_count: self.damage_count,
            final_damage_first: self.final_damage_first,
            final_damage_count: self.final_damage_count,
            draw_count: self.draws.len() as u32,
            resource_count: resources.len() as u32,
            rect_count: self.rects.len() as u32,
            reserved2: 0,
            draws: self.draws.as_ptr(),
            resources: resources.as_ptr(),
            rects: self.rects.as_ptr(),
        };
        Ok(visitor(&view))
    }
}

fn aligned<T>(pointer: *const T) -> bool {
    !pointer.is_null() && (pointer as usize).is_multiple_of(align_of::<T>())
}

fn finite(values: &[f32]) -> bool {
    values.iter().all(|v| v.is_finite())
}
fn finite_box(value: FrameBoxV1) -> bool {
    [value.x, value.y, value.width, value.height, value.rotation]
        .iter()
        .all(|v| v.is_finite())
}
fn metadata_bytes(
    draws: usize,
    rectangles: usize,
    snapshots: usize,
    borrows: usize,
) -> Result<usize, Error> {
    let mut bytes = size_of::<Frame>();
    for (count, width) in [
        (draws, size_of::<FrameDrawV1>()),
        (rectangles, size_of::<FrameRectV1>()),
        (snapshots, size_of::<Snapshot>()),
        (borrows, size_of::<FrameResourceBorrowV1>()),
    ] {
        bytes = bytes
            .checked_add(count.checked_mul(width).ok_or(Error::Capacity)?)
            .ok_or(Error::Capacity)?;
    }
    if bytes > PR_FRAME_MAX_METADATA_BYTES {
        return Err(Error::Capacity);
    }
    Ok(bytes)
}
fn valid_target(target: &FrameTargetV1) -> bool {
    target.output_id != 0
        && target.output_generation != 0
        && target.output_transform <= 7
        && target.flags & !PR_TARGET_SHADER_CM_AVAILABLE == 0
        && [
            target.width,
            target.height,
            target.transformed_width,
            target.transformed_height,
        ]
        .iter()
        .all(|v| (1..=16_384).contains(v))
        && target.scale.is_finite()
        && target.scale > 0.0
        && finite(&target.target_projection)
        && finite(&target.scale_matrix)
}
fn valid_cm(cm: &FrameColorManagementV1) -> bool {
    if cm.flags & !(PR_CM_ENABLED | PR_CM_TONEMAP | PR_CM_SDR_MOD) != 0
        || !finite(&[
            cm.src_tf_min,
            cm.src_tf_max,
            cm.dst_tf_min,
            cm.dst_tf_max,
            cm.src_ref_luminance,
            cm.dst_ref_luminance,
            cm.max_luminance,
            cm.dst_max_luminance,
            cm.sdr_saturation,
            cm.sdr_brightness,
        ])
        || !finite(&cm.convert_matrix)
        || !finite(&cm.dst_primaries_xyz)
        || cm.source_tf > i32::MAX as u32
        || cm.target_tf > i32::MAX as u32
        || cm.tonemap_mode > 3
    {
        return false;
    }
    if cm.flags & PR_CM_ENABLED == 0 {
        return cm.flags & (PR_CM_TONEMAP | PR_CM_SDR_MOD) == 0;
    }
    cm.src_tf_max > cm.src_tf_min
        && cm.dst_tf_max > cm.dst_tf_min
        && cm.src_ref_luminance > 0.0
        && cm.dst_ref_luminance > 0.0
        && cm.max_luminance > 0.0
        && cm.dst_max_luminance > 0.0
        && cm.sdr_saturation >= 0.0
        && cm.sdr_brightness >= 0.0
        && (cm.flags & PR_CM_TONEMAP == 0 || (1..=3).contains(&cm.tonemap_mode))
}
fn valid_material(material: &FrameMaterialV1) -> bool {
    material.flags
        & !(PR_MAT_NEAREST
            | PR_MAT_BLEND
            | PR_MAT_RGBX
            | PR_MAT_TINT
            | PR_MAT_DISCARD_ACTIVE
            | PR_MAT_DISCARD_OPAQUE
            | PR_MAT_DISCARD_ALPHA)
        == 0
        && material.reserved == 0
        && material.wrap_x <= 1
        && material.wrap_y <= 1
        && finite(&[
            material.alpha,
            material.rounding_px,
            material.rounding_power,
            material.discard_opacity,
        ])
        && material.rounding_px >= 0.0
        && material.rounding_power > 0.0
        && finite(&material.tint_rgb)
        && valid_cm(&material.cm)
}
fn range(
    first: u32,
    count: u32,
    length: usize,
    used: &mut [bool; PR_FRAME_MAX_RECTS],
    total: &mut usize,
) -> Result<(), Error> {
    let first = usize::try_from(first).map_err(|_| Error::Invalid)?;
    let count = usize::try_from(count).map_err(|_| Error::Invalid)?;
    *total = total.checked_add(count).ok_or(Error::Capacity)?;
    if *total > PR_FRAME_MAX_RECTS {
        return Err(Error::Capacity);
    }
    let end = first.checked_add(count).ok_or(Error::Invalid)?;
    if first > length || end > length {
        return Err(Error::Invalid);
    }
    for flag in &mut used[first..end] {
        if *flag {
            return Err(Error::Invalid);
        }
        *flag = true;
    }
    Ok(())
}
/// # Safety
/// The nonempty range must remain live, aligned, readable and immutable for the returned borrow.
unsafe fn borrowed_slice<'a, T>(pointer: *const T, count: usize) -> Result<&'a [T], Error> {
    if count == 0 {
        return Ok(&[]);
    }
    if !aligned(pointer)
        || count
            .checked_mul(size_of::<T>())
            .is_none_or(|n| n > isize::MAX as usize)
    {
        return Err(Error::Invalid);
    }
    Ok(unsafe { std::slice::from_raw_parts(pointer, count) })
}
impl Frame {
    /// # Safety
    /// Arrays/handles advertised by input must be live, aligned, readable and unchanged for this call.
    /// Snapshot handles are borrowed only; neither this function nor failure consumes them.
    pub unsafe fn capture_borrow(input: &FrameBorrowV1) -> Result<Self, Error> {
        if input.abi_version != PR_FRAME_ABI_V1
            || input.struct_size as usize != size_of::<FrameBorrowV1>()
            || input.reserved != 0
            || input.reserved2 != 0
            || input.flags & !(PR_FRAME_COMMIT | PR_FRAME_FULL_REPAINT) != 0
            || input.scene_sequence == 0
            || input.security_generation == 0
            || !valid_target(&input.target)
        {
            return Err(Error::Invalid);
        }
        let draw_count = input.draw_count as usize;
        let resource_count = input.resource_count as usize;
        let rect_count = input.rect_count as usize;
        if draw_count > PR_FRAME_MAX_DRAWS
            || resource_count > PR_FRAME_MAX_RESOURCES
            || rect_count > PR_FRAME_MAX_RECTS
        {
            return Err(Error::Capacity);
        }
        metadata_bytes(draw_count, rect_count, resource_count, resource_count)?;
        let draws = unsafe { borrowed_slice(input.draws, draw_count)? };
        let resources = unsafe { borrowed_slice(input.resources, resource_count)? };
        let rects = unsafe { borrowed_slice(input.rects, rect_count)? };
        for resource in resources {
            if !aligned(resource.snapshot) {
                return Err(Error::Invalid);
            }
            let snapshot = unsafe { &*resource.snapshot };
            if resource.description != snapshot.description()
                || resource.description.byte_len()? != snapshot.pixels().len()
            {
                return Err(Error::Invalid);
            }
        }
        for rect in rects {
            if rect.x1 < 0
                || rect.y1 < 0
                || rect.x2 <= rect.x1
                || rect.y2 <= rect.y1
                || rect.x2 as u32 > input.target.width
                || rect.y2 as u32 > input.target.height
            {
                return Err(Error::Invalid);
            }
        }
        let mut used = [false; PR_FRAME_MAX_RECTS];
        let mut total = 0;
        range(
            input.damage_first,
            input.damage_count,
            rect_count,
            &mut used,
            &mut total,
        )?;
        range(
            input.final_damage_first,
            input.final_damage_count,
            rect_count,
            &mut used,
            &mut total,
        )?;
        for draw in draws {
            if draw.kind > PR_DRAW_SHM
                || !finite_box(draw.box_)
                || !finite_box(draw.rounding_box)
                || !finite(&draw.projection)
                || !finite(&draw.uv)
                || !finite(&draw.converted_color)
                || !finite(&draw.color_srgb)
                || !valid_material(&draw.material)
            {
                return Err(Error::Invalid);
            }
            if draw.kind != PR_DRAW_CLEAR && (draw.box_.width <= 0.0 || draw.box_.height <= 0.0) {
                return Err(Error::Invalid);
            }
            if draw.material.rounding_px > 0.0
                && (draw.rounding_box.width <= 0.0
                    || draw.rounding_box.height <= 0.0
                    || !finite(&[
                        draw.rounding_box.x as f32,
                        draw.rounding_box.y as f32,
                        draw.rounding_box.width as f32,
                        draw.rounding_box.height as f32,
                    ]))
            {
                return Err(Error::Invalid);
            }
            if draw.kind == PR_DRAW_SHM {
                let resource = resources
                    .get(draw.resource_index as usize)
                    .ok_or(Error::Invalid)?;
                if draw.surface_id == 0
                    || draw.commit_sequence != resource.description.commit_sequence
                {
                    return Err(Error::Invalid);
                }
            } else if draw.resource_index != u32::MAX {
                return Err(Error::Invalid);
            }
            range(
                draw.scissor_first,
                draw.scissor_count,
                rect_count,
                &mut used,
                &mut total,
            )?;
        }
        let mut copied_draws = Vec::new();
        let mut copied_rects = Vec::new();
        let mut snapshots = Vec::new();
        copied_draws
            .try_reserve_exact(draw_count)
            .map_err(|_| Error::Allocation)?;
        copied_rects
            .try_reserve_exact(rect_count)
            .map_err(|_| Error::Allocation)?;
        snapshots
            .try_reserve_exact(resource_count)
            .map_err(|_| Error::Allocation)?;
        metadata_bytes(
            copied_draws.capacity(),
            copied_rects.capacity(),
            snapshots.capacity(),
            resource_count,
        )?;
        copied_draws.extend_from_slice(draws);
        copied_rects.extend_from_slice(rects);
        for resource in resources {
            snapshots.push(unsafe { &*resource.snapshot }.clone());
        }
        Ok(Self {
            flags: input.flags,
            scene_sequence: input.scene_sequence,
            security_generation: input.security_generation,
            target: input.target,
            damage_first: input.damage_first,
            damage_count: input.damage_count,
            final_damage_first: input.final_damage_first,
            final_damage_count: input.final_damage_count,
            draws: copied_draws,
            resources: snapshots,
            rects: copied_rects,
        })
    }
}

/// # Safety
/// input/result address advertised live aligned objects; nested arrays and snapshot handles remain
/// live/readable/unchanged for this synchronous call. result is a separate writable handle slot.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_frame_capture_v1(
    input: *const FrameBorrowV1,
    result: *mut *mut Frame,
) -> i32 {
    if !aligned(result) {
        return Error::Invalid as i32;
    }
    unsafe {
        result.write(ptr::null_mut());
    }
    if !aligned(input) {
        return Error::Invalid as i32;
    }
    let frame = match unsafe { Frame::capture_borrow(&*input) } {
        Ok(frame) => frame,
        Err(error) => return error as i32,
    };
    // Checked fixed handle allocation, compatible with Box deallocation on release.
    let allocation = unsafe { std::alloc::alloc(std::alloc::Layout::new::<Frame>()) } as *mut Frame;
    if allocation.is_null() {
        return Error::Allocation as i32;
    }
    unsafe {
        allocation.write(frame);
        result.write(allocation);
    }
    0
}
/// # Safety
/// frame is null or a live uniquely owned capture handle, released once. No borrow/callback/worker
/// may concurrently view, release or transfer this same handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_frame_release(frame: *mut Frame) {
    if !frame.is_null() {
        unsafe {
            drop(Box::from_raw(frame));
        }
    }
}
/// # Safety
/// frame remains live and exclusively owned for this entire call. visitor/stack_token remain valid
/// synchronously; visitor must not retain pointers, release/transfer frame, throw or unwind.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_frame_with_view_v1(
    frame: *const Frame,
    visitor: Option<FrameViewFnV1>,
    stack_token: *mut c_void,
) -> i32 {
    if !aligned(frame) {
        return Error::Invalid as i32;
    }
    let Some(visitor) = visitor else {
        return Error::Invalid as i32;
    };
    match unsafe { &*frame }
        .with_borrow(|view| unsafe { visitor(ptr::from_ref(view), stack_token) })
    {
        Ok(status) => status,
        Err(error) => error as i32,
    }
}
