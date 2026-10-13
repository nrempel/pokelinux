use pokelinux_boot_inputs::{Extent, inspect};
use std::io::{Cursor, Read, Seek, SeekFrom};
fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::PathBuf::from(
            std::env::var_os("BOOT_REFERENCE_FIXTURES")
                .expect("original AOSP-produced fixtures required"),
        )
        .join(name),
    )
    .unwrap()
}
fn put(v: &mut [u8], at: usize, value: u32) {
    v[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
fn reject(v: Vec<u8>) {
    assert!(inspect(&mut Cursor::new(v)).is_err());
}
#[test]
fn original_boot_layout() {
    let r = inspect(&mut Cursor::new(fixture("boot-v4.img"))).unwrap();
    assert_eq!(r.kind, "boot-v4");
    assert_eq!(r.page_size, 4096);
    assert_eq!(r.header_size, 1584);
    assert_eq!(
        r.kernel,
        Extent {
            offset: 4096,
            size: 8197
        }
    );
    assert_eq!(
        r.ramdisk,
        Extent {
            offset: 16384,
            size: 4097
        }
    );
    assert_eq!(r.cmdline, b"console=ttyS0");
}
#[test]
fn original_vendor_layout() {
    let r = inspect(&mut Cursor::new(fixture("vendor-v4.img"))).unwrap();
    assert_eq!(r.kind, "vendor-boot-v4");
    assert_eq!(
        r.ramdisk,
        Extent {
            offset: 4096,
            size: 9124
        }
    );
    assert_eq!(
        r.dtb,
        Extent {
            offset: 16384,
            size: 127
        }
    );
    assert_eq!(
        r.table,
        Extent {
            offset: 20480,
            size: 216
        }
    );
    assert_eq!(
        r.bootconfig,
        Extent {
            offset: 24576,
            size: 9
        }
    );
    assert_eq!(r.fragments.len(), 2);
    assert_eq!(
        r.fragments[1].extent,
        Extent {
            offset: 4219,
            size: 9001
        }
    );
    assert_eq!(r.fragments[1].name, b"modules");
    assert_eq!(r.fragments[1].ramdisk_type, 3);
    assert_eq!(r.fragments[1].board_id[0], 17);
}
#[test]
fn rejects_unknown_magic() {
    let mut v = fixture("boot-v4.img");
    v[0] = 0;
    reject(v);
}
#[test]
fn rejects_short_header() {
    reject(fixture("boot-v4.img")[..100].to_vec());
}
#[test]
fn rejects_boot_previous_version() {
    let mut v = fixture("boot-v4.img");
    put(&mut v, 40, 3);
    reject(v);
}
#[test]
fn rejects_vendor_future_version() {
    let mut v = fixture("vendor-v4.img");
    put(&mut v, 8, 5);
    reject(v);
}
#[test]
fn rejects_boot_header_size() {
    let mut v = fixture("boot-v4.img");
    put(&mut v, 20, 1580);
    reject(v);
}
#[test]
fn rejects_vendor_header_size() {
    let mut v = fixture("vendor-v4.img");
    put(&mut v, 2096, 2129);
    reject(v);
}
#[test]
fn rejects_reserved_word() {
    let mut v = fixture("boot-v4.img");
    put(&mut v, 24, 1);
    reject(v);
}
#[test]
fn rejects_boot_payload_truncation() {
    let mut v = fixture("boot-v4.img");
    v.truncate(20480);
    reject(v);
}
#[test]
fn rejects_boot_unbounded_extent() {
    let mut v = fixture("boot-v4.img");
    put(&mut v, 8, u32::MAX);
    reject(v);
}
#[test]
fn rejects_boot_signature_extent() {
    let mut v = fixture("boot-v4.img");
    put(&mut v, 1580, 8192);
    reject(v);
}
#[test]
fn rejects_bad_vendor_page() {
    let mut v = fixture("vendor-v4.img");
    put(&mut v, 12, 4097);
    reject(v);
}
#[test]
fn rejects_too_small_vendor_page() {
    let mut v = fixture("vendor-v4.img");
    put(&mut v, 12, 1024);
    reject(v);
}
#[test]
fn rejects_vendor_payload_truncation() {
    let mut v = fixture("vendor-v4.img");
    v.truncate(24576);
    reject(v);
}
#[test]
fn rejects_table_size_disagreement() {
    let mut v = fixture("vendor-v4.img");
    put(&mut v, 2112, 215);
    reject(v);
}
#[test]
fn rejects_unbounded_table_count() {
    let mut v = fixture("vendor-v4.img");
    put(&mut v, 2116, 4097);
    reject(v);
}
#[test]
fn rejects_wrong_table_entry_size() {
    let mut v = fixture("vendor-v4.img");
    put(&mut v, 2120, 109);
    reject(v);
}
#[test]
fn rejects_fragment_outside_ramdisk() {
    let mut v = fixture("vendor-v4.img");
    put(&mut v, 20480 + 108 + 4, 9000);
    reject(v);
}
#[test]
fn rejects_fragment_overlap() {
    let mut v = fixture("vendor-v4.img");
    put(&mut v, 20480 + 108 + 4, 0);
    reject(v);
}
#[test]
fn rejects_fragment_gap() {
    let mut v = fixture("vendor-v4.img");
    put(&mut v, 20480 + 108 + 4, 124);
    reject(v);
}
#[test]
fn rejects_unknown_ramdisk_type() {
    let mut v = fixture("vendor-v4.img");
    put(&mut v, 20480 + 8, 4);
    reject(v);
}
#[test]
fn rejects_duplicate_fragment_name() {
    let mut v = fixture("vendor-v4.img");
    let name = v[20480 + 12..20480 + 44].to_vec();
    v[20480 + 108 + 12..20480 + 108 + 44].copy_from_slice(&name);
    reject(v);
}
#[test]
fn rejects_nonterminated_commandline() {
    let mut v = fixture("boot-v4.img");
    v[44..1580].fill(b'x');
    reject(v);
}
#[test]
fn accepts_uninterpreted_trailing_partition_bytes() {
    let mut v = fixture("boot-v4.img");
    v.extend_from_slice(b"authored trailing footer");
    let r = inspect(&mut Cursor::new(v)).unwrap();
    assert_eq!(
        r.trailing,
        Extent {
            offset: 24576,
            size: 24
        }
    );
}
struct Meter {
    c: Cursor<Vec<u8>>,
    max: usize,
    total: usize,
}
impl Read for Meter {
    fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
        self.max = self.max.max(b.len());
        let n = self.c.read(b)?;
        self.total += n;
        Ok(n)
    }
}
impl Seek for Meter {
    fn seek(&mut self, p: SeekFrom) -> std::io::Result<u64> {
        self.c.seek(p)
    }
}
#[test]
fn metadata_reads_are_bounded() {
    let mut m = Meter {
        c: Cursor::new(fixture("vendor-v4.img")),
        max: 0,
        total: 0,
    };
    let r = inspect(&mut m).unwrap();
    assert_eq!(r.fragments.len(), 2);
    assert!(m.max <= 4096);
    assert!(m.total <= 4096 + 216);
}
#[test]
fn original_vendor_2048_page_layout() {
    let r = inspect(&mut Cursor::new(fixture("vendor-2048-v4.img"))).unwrap();
    assert_eq!(r.page_size, 2048);
    assert_eq!(
        r.ramdisk,
        Extent {
            offset: 4096,
            size: 123
        }
    );
    assert_eq!(
        r.table,
        Extent {
            offset: 6144,
            size: 108
        }
    );
}
