use serde::Serialize;
use std::io::{Read, Seek};
#[derive(Debug, Default, PartialEq, Eq, Serialize)]
pub struct Extent {
    pub offset: u64,
    pub size: u64,
}
#[derive(Debug, Default, PartialEq, Eq, Serialize)]
pub struct Fragment {
    pub extent: Extent,
    pub ramdisk_type: u32,
    pub name: Vec<u8>,
    pub board_id: [u32; 16],
}
#[derive(Debug, Default, PartialEq, Eq, Serialize)]
pub struct ImageReport {
    pub kind: String,
    pub header_version: u32,
    pub header_size: u32,
    pub page_size: u32,
    pub file_size: u64,
    pub kernel: Extent,
    pub ramdisk: Extent,
    pub signature: Extent,
    pub dtb: Extent,
    pub table: Extent,
    pub bootconfig: Extent,
    pub fragments: Vec<Fragment>,
    pub cmdline: Vec<u8>,
    pub product_name: Vec<u8>,
    pub os_version: u32,
    pub kernel_address: u32,
    pub ramdisk_address: u32,
    pub tags_address: u32,
    pub dtb_address: u64,
    pub trailing: Extent,
}
#[derive(Debug)]
pub struct InspectError(pub &'static str);

impl std::fmt::Display for InspectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for InspectError {}
const MAX_IMAGE_SIZE: u64 = 512 * 1024 * 1024;
const MAX_FRAGMENTS: u32 = 4096;
fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}
fn u64_at(bytes: &[u8], at: usize) -> u64 {
    u64::from(u32_at(bytes, at)) | (u64::from(u32_at(bytes, at + 4)) << 32)
}
fn cstring(bytes: &[u8]) -> Result<Vec<u8>, InspectError> {
    let end = bytes
        .iter()
        .position(|b| *b == 0)
        .ok_or(InspectError("unterminated string"))?;
    if bytes[end..].iter().any(|b| *b != 0) {
        return Err(InspectError("nonzero string padding"));
    }
    Ok(bytes[..end].to_vec())
}
fn aligned(size: u64, page: u64) -> Result<u64, InspectError> {
    size.checked_add(page - 1)
        .map(|s| s & !(page - 1))
        .ok_or(InspectError("alignment overflow"))
}
fn section(cursor: &mut u64, size: u64, page: u64, file_size: u64) -> Result<Extent, InspectError> {
    let extent = Extent {
        offset: *cursor,
        size,
    };
    *cursor = cursor
        .checked_add(aligned(size, page)?)
        .ok_or(InspectError("extent overflow"))?;
    if *cursor > file_size {
        return Err(InspectError("section outside image"));
    }
    Ok(extent)
}
fn read_at<R: Read + Seek>(
    input: &mut R,
    offset: u64,
    bytes: &mut [u8],
) -> Result<(), InspectError> {
    input
        .seek(std::io::SeekFrom::Start(offset))
        .map_err(|_| InspectError("seek failed"))?;
    input
        .read_exact(bytes)
        .map_err(|_| InspectError("truncated metadata"))
}
/// Inspect current Android v4 boot/vendor-boot geometry without reading payloads.
/// Trailing partition bytes are reported as opaque; this does not authenticate AVB,
/// select a bootloader mode, validate compressed ramdisks, or qualify a kernel ABI.
/// The caller supplies a cooperative immutable seekable input.
pub fn inspect<R: Read + Seek>(input: &mut R) -> Result<ImageReport, InspectError> {
    let file_size = input
        .seek(std::io::SeekFrom::End(0))
        .map_err(|_| InspectError("size seek failed"))?;
    if !(8..=MAX_IMAGE_SIZE).contains(&file_size) {
        return Err(InspectError("image size outside bound"));
    }
    let mut magic = [0u8; 8];
    read_at(input, 0, &mut magic)?;
    match &magic {
        b"ANDROID!" => inspect_boot(input, file_size),
        b"VNDRBOOT" => inspect_vendor(input, file_size),
        _ => Err(InspectError("unknown image magic")),
    }
}
fn inspect_boot<R: Read + Seek>(
    input: &mut R,
    file_size: u64,
) -> Result<ImageReport, InspectError> {
    let mut h = [0u8; 1584];
    read_at(input, 0, &mut h)?;
    if u32_at(&h, 40) != 4 {
        return Err(InspectError("unsupported boot header version"));
    }
    if u32_at(&h, 20) != 1584 {
        return Err(InspectError("wrong boot header size"));
    }
    if h[24..40].iter().any(|b| *b != 0) {
        return Err(InspectError("nonzero reserved header word"));
    }
    let mut cursor = 4096;
    let kernel = section(&mut cursor, u64::from(u32_at(&h, 8)), 4096, file_size)?;
    let ramdisk = section(&mut cursor, u64::from(u32_at(&h, 12)), 4096, file_size)?;
    let signature = section(&mut cursor, u64::from(u32_at(&h, 1580)), 4096, file_size)?;
    Ok(ImageReport {
        kind: "boot-v4".into(),
        header_version: 4,
        header_size: 1584,
        page_size: 4096,
        file_size,
        kernel,
        ramdisk,
        signature,
        cmdline: cstring(&h[44..1580])?,
        os_version: u32_at(&h, 16),
        trailing: Extent {
            offset: cursor,
            size: file_size - cursor,
        },
        ..ImageReport::default()
    })
}
fn inspect_vendor<R: Read + Seek>(
    input: &mut R,
    file_size: u64,
) -> Result<ImageReport, InspectError> {
    let mut h = [0u8; 2128];
    read_at(input, 0, &mut h)?;
    if u32_at(&h, 8) != 4 {
        return Err(InspectError("unsupported vendor header version"));
    }
    if u32_at(&h, 2096) != 2128 {
        return Err(InspectError("wrong vendor header size"));
    }
    let page_size = u32_at(&h, 12);
    if ![2048, 4096, 8192, 16384].contains(&page_size) {
        return Err(InspectError("unsupported vendor container page size"));
    }
    let page = u64::from(page_size);
    let count = u32_at(&h, 2116);
    if count > MAX_FRAGMENTS {
        return Err(InspectError("too many vendor fragments"));
    }
    if u32_at(&h, 2120) != 108 {
        return Err(InspectError("wrong vendor table entry size"));
    }
    if u32_at(&h, 2112) != count * 108 {
        return Err(InspectError("vendor table size mismatch"));
    }
    let mut cursor = aligned(2128, page)?;
    let ramdisk = section(&mut cursor, u64::from(u32_at(&h, 24)), page, file_size)?;
    let dtb = section(&mut cursor, u64::from(u32_at(&h, 2100)), page, file_size)?;
    let table = section(&mut cursor, u64::from(u32_at(&h, 2112)), page, file_size)?;
    let bootconfig = section(&mut cursor, u64::from(u32_at(&h, 2124)), page, file_size)?;
    let mut fragments = Vec::with_capacity(count as usize);
    let mut names = std::collections::BTreeSet::new();
    let mut fragment_end = 0u64;
    for index in 0..count {
        let mut entry = [0u8; 108];
        read_at(input, table.offset + u64::from(index) * 108, &mut entry)?;
        let size = u64::from(u32_at(&entry, 0));
        let offset = u64::from(u32_at(&entry, 4));
        let end = offset
            .checked_add(size)
            .ok_or(InspectError("fragment extent overflow"))?;
        if offset != fragment_end || end > ramdisk.size || size == 0 {
            return Err(InspectError("fragment extent gap overlap or bounds"));
        }
        fragment_end = end;
        let ramdisk_type = u32_at(&entry, 8);
        if ramdisk_type > 3 {
            return Err(InspectError("unknown vendor ramdisk type"));
        }
        let name = cstring(&entry[12..44])?;
        if !names.insert(name.clone()) {
            return Err(InspectError("duplicate vendor ramdisk name"));
        }
        let mut board_id = [0u32; 16];
        for (at, value) in board_id.iter_mut().enumerate() {
            *value = u32_at(&entry, 44 + at * 4);
        }
        fragments.push(Fragment {
            extent: Extent {
                offset: ramdisk.offset + offset,
                size,
            },
            ramdisk_type,
            name,
            board_id,
        });
    }
    if fragment_end != ramdisk.size {
        return Err(InspectError("vendor ramdisk bytes absent from table"));
    }
    Ok(ImageReport {
        kind: "vendor-boot-v4".into(),
        header_version: 4,
        header_size: 2128,
        page_size,
        file_size,
        ramdisk,
        dtb,
        table,
        bootconfig,
        fragments,
        cmdline: cstring(&h[28..2076])?,
        product_name: cstring(&h[2080..2096])?,
        kernel_address: u32_at(&h, 16),
        ramdisk_address: u32_at(&h, 20),
        tags_address: u32_at(&h, 2076),
        dtb_address: u64_at(&h, 2104),
        trailing: Extent {
            offset: cursor,
            size: file_size - cursor,
        },
        ..ImageReport::default()
    })
}
