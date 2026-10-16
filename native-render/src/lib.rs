//! Immutable CPU payload custody for the connected compositor import path.
//!
//! Rendering, output custody and execution are still separate implementation work.
//! Copying pixels does not prove upload, GPU completion, presentation or authority.
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

const MAX_EXTENT: u32 = 16_384;
const MAX_SNAPSHOT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_STORE_BYTES: u64 = 512 * 1024 * 1024;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShmDescription {
    pub resource_id: u64,
    pub generation: u64,
    pub commit_sequence: u64,
    pub drm_format: u32,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub bytes_per_pixel: u32,
    pub reserved: u32,
}

#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Invalid = 1,
    Capacity = 2,
    Allocation = 3,
}

impl ShmDescription {
    fn byte_len(self) -> Result<usize, Error> {
        if self.resource_id == 0
            || self.generation == 0
            || self.commit_sequence == 0
            || self.reserved != 0
            || self.drm_format == 0
            || !(1..=MAX_EXTENT).contains(&self.width)
            || !(1..=MAX_EXTENT).contains(&self.height)
            || ![1, 2, 3, 4, 6, 8, 12, 16].contains(&self.bytes_per_pixel)
            || u64::from(self.stride) < u64::from(self.width) * u64::from(self.bytes_per_pixel)
        {
            return Err(Error::Invalid);
        }
        let bytes = u64::from(self.stride) * u64::from(self.height);
        if bytes == 0 || bytes > MAX_SNAPSHOT_BYTES {
            return Err(Error::Capacity);
        }
        usize::try_from(bytes).map_err(|_| Error::Capacity)
    }
}

struct Budget {
    limit: u64,
    used: AtomicU64,
}
struct Reservation {
    budget: Arc<Budget>,
    bytes: u64,
}
impl Drop for Reservation {
    fn drop(&mut self) {
        self.budget.used.fetch_sub(self.bytes, Ordering::AcqRel);
    }
}
struct Pixels {
    bytes: Box<[u8]>,
    _reservation: Reservation,
}
pub struct SnapshotStore {
    budget: Arc<Budget>,
}
#[derive(Clone)]
pub struct Snapshot {
    description: ShmDescription,
    pixels: Arc<Pixels>,
}
impl SnapshotStore {
    pub fn new(max_bytes: u64) -> Result<Self, Error> {
        if max_bytes == 0 || max_bytes > MAX_STORE_BYTES {
            return Err(Error::Invalid);
        }
        Ok(Self {
            budget: Arc::new(Budget {
                limit: max_bytes,
                used: AtomicU64::new(0),
            }),
        })
    }
    pub fn used(&self) -> u64 {
        self.budget.used.load(Ordering::Acquire)
    }
    pub fn capture(&self, description: ShmDescription, source: &[u8]) -> Result<Snapshot, Error> {
        let size = description.byte_len()?;
        if source.len() != size {
            return Err(Error::Invalid);
        }
        let count = size as u64;
        self.budget
            .used
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |used| {
                used.checked_add(count)
                    .filter(|next| *next <= self.budget.limit)
            })
            .map_err(|_| Error::Capacity)?;
        let reservation = Reservation {
            budget: Arc::clone(&self.budget),
            bytes: count,
        };
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(size)
            .map_err(|_| Error::Allocation)?;
        bytes.extend_from_slice(source);
        Ok(Snapshot {
            description,
            pixels: Arc::new(Pixels {
                bytes: bytes.into_boxed_slice(),
                _reservation: reservation,
            }),
        })
    }
}
impl Snapshot {
    pub fn description(&self) -> ShmDescription {
        self.description
    }
    pub fn pixels(&self) -> &[u8] {
        &self.pixels.bytes
    }
}

#[repr(C)]
pub struct SnapshotView {
    description: ShmDescription,
    pixels: *const u8,
    pixel_bytes: u64,
}

/// # Safety
/// The returned store must be released once; other handles must not use it concurrently with release.
#[unsafe(no_mangle)]
pub extern "C" fn pr_snapshot_store_create(max_bytes: u64) -> *mut SnapshotStore {
    match SnapshotStore::new(max_bytes) {
        Ok(store) => Box::into_raw(Box::new(store)),
        Err(_) => std::ptr::null_mut(),
    }
}
/// # Safety
/// `store` is null or a live uniquely owned handle from create, released once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_snapshot_store_release(store: *mut SnapshotStore) {
    if !store.is_null() {
        unsafe {
            drop(Box::from_raw(store));
        }
    }
}
/// # Safety
/// `store` addresses a live store for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_snapshot_store_used(store: *const SnapshotStore) -> u64 {
    if store.is_null() {
        return 0;
    }
    unsafe { (&*store).used() }
}
/// # Safety
/// Inputs address live aligned objects, pixels are readable for pixel_bytes, and
/// result is a writable handle slot. All remain valid and unmodified for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_snapshot_capture(
    store: *const SnapshotStore,
    description: *const ShmDescription,
    pixels: *const u8,
    pixel_bytes: u64,
    result: *mut *mut Snapshot,
) -> i32 {
    if result.is_null() {
        return Error::Invalid as i32;
    }
    unsafe {
        result.write(std::ptr::null_mut());
    }
    if store.is_null() || description.is_null() || pixels.is_null() {
        return Error::Invalid as i32;
    }
    let description = unsafe { *description };
    let size = match description.byte_len() {
        Ok(size) => size,
        Err(e) => return e as i32,
    };
    if pixel_bytes != size as u64 || size > isize::MAX as usize {
        return Error::Invalid as i32;
    }
    let source = unsafe { std::slice::from_raw_parts(pixels, size) };
    match unsafe { &*store }.capture(description, source) {
        Ok(snapshot) => {
            unsafe {
                result.write(Box::into_raw(Box::new(snapshot)));
            }
            0
        }
        Err(error) => error as i32,
    }
}
/// # Safety
/// snapshot addresses a live handle and remains valid for this call. The returned
/// clone is separately owned and must be released once; its pixels are immutable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_snapshot_clone(snapshot: *const Snapshot) -> *mut Snapshot {
    if snapshot.is_null() {
        return std::ptr::null_mut();
    }
    Box::into_raw(Box::new(unsafe { &*snapshot }.clone()))
}
/// # Safety
/// snapshot addresses a live handle and result is writable/aligned. The returned
/// pixel borrow must not outlive the snapshot handle, including its release.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_snapshot_view(
    snapshot: *const Snapshot,
    result: *mut SnapshotView,
) -> i32 {
    if snapshot.is_null() || result.is_null() {
        return Error::Invalid as i32;
    }
    let snapshot = unsafe { &*snapshot };
    unsafe {
        result.write(SnapshotView {
            description: snapshot.description(),
            pixels: snapshot.pixels().as_ptr(),
            pixel_bytes: snapshot.pixels().len() as u64,
        });
    }
    0
}
/// # Safety
/// snapshot is null or a live uniquely owned handle from capture/clone, released once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pr_snapshot_release(snapshot: *mut Snapshot) {
    if !snapshot.is_null() {
        unsafe {
            drop(Box::from_raw(snapshot));
        }
    }
}

pub mod commit;

pub mod executor;
pub mod frame;
