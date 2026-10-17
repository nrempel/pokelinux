//! Authored synthetic regular-file/class-symlink I/O. Never host sysfs or hardware.
use pokelinux_brightness::{Backend, BackendKind, Observation};
use std::{
    fs,
    os::unix::fs::symlink,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

static SERIAL: AtomicU64 = AtomicU64::new(0);
struct Tree {
    root: PathBuf,
}
impl Tree {
    fn new(max: u32, requested: u32, actual: Option<u32>) -> Self {
        let root = std::env::temp_dir().join(format!(
            "pokelinux-brightness-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("class")).unwrap();
        fs::create_dir(root.join("device")).unwrap();
        symlink("../device", root.join("class/panel0")).unwrap();
        let tree = Self { root };
        tree.write("max_brightness", &format!("{max}\n"));
        tree.write("brightness", &format!("{requested}\n"));
        if let Some(actual) = actual {
            tree.write("actual_brightness", &format!("{actual}\n"));
        }
        tree
    }
    fn class(&self) -> PathBuf {
        self.root.join("class")
    }
    fn attr(&self, name: &str) -> PathBuf {
        self.root.join("device").join(name)
    }
    fn write(&self, name: &str, value: &str) {
        fs::write(self.attr(name), value).unwrap();
    }
    fn backend(&self) -> Backend {
        Backend::new(self.class(), "panel0".into(), BackendKind::SyntheticTree).unwrap()
    }
    fn bytes(&self, name: &str) -> Option<Vec<u8>> {
        match fs::read(self.attr(name)) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => panic!("fixture read failed: {error}"),
        }
    }
    fn all_bytes(&self) -> [Option<Vec<u8>>; 3] {
        [
            self.bytes("max_brightness"),
            self.bytes("brightness"),
            self.bytes("actual_brightness"),
        ]
    }
}
impl Drop for Tree {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

#[test]
fn real_class_reads_zero_full_and_optional_actual_without_writes() {
    for (max, requested, actual) in [
        (100, 0, Some(0)),
        (100, 100, Some(99)),
        (100, 25, None),
        (u32::MAX, u32::MAX, Some(u32::MAX)),
    ] {
        let tree = Tree::new(max, requested, actual);
        let before = tree.all_bytes();
        assert_eq!(
            tree.backend()
                .read()
                .expect("valid synthetic attributes must be readable"),
            Observation {
                max,
                requested,
                actual
            }
        );
        assert_eq!(tree.all_bytes(), before, "read must not modify attributes");
    }
}

#[test]
fn actual_25_to_75_zero_full_writes_have_independent_readback() {
    let tree = Tree::new(100, 25, Some(22));
    let backend = tree.backend();
    for target in [75, 0, 100] {
        let result = backend
            .set_percent(target)
            .expect("one valid write must succeed");
        assert_eq!(result.target, u32::from(target));
        assert_eq!(
            result.observed,
            Observation {
                max: 100,
                requested: u32::from(target),
                actual: Some(22)
            }
        );
        assert_eq!(
            tree.bytes("brightness"),
            Some(format!("{target}\n").into_bytes())
        );
        assert_eq!(backend.read().unwrap(), result.observed);
        assert_eq!(tree.bytes("max_brightness"), Some(b"100\n".to_vec()));
        assert_eq!(tree.bytes("actual_brightness"), Some(b"22\n".to_vec()));
    }
}

#[test]
fn percent_scaling_rounds_nearest_without_u32_overflow() {
    // Literal expected vectors are independent of the implementation's converter.
    for (max, percent, target) in [
        (7, 25, 2),
        (7, 75, 5),
        (7, 50, 4),
        (1, 50, 1),
        (u32::MAX, 25, 1_073_741_824),
        (u32::MAX, 75, 3_221_225_471),
        (u32::MAX, 100, u32::MAX),
    ] {
        let tree = Tree::new(max, 0, None);
        let result = tree.backend().set_percent(percent).unwrap();
        assert_eq!(result.target, target);
        assert_eq!(
            result.observed,
            Observation {
                max,
                requested: target,
                actual: None
            }
        );
        assert_eq!(
            tree.bytes("brightness"),
            Some(format!("{target}\n").into_bytes())
        );
    }
}

#[test]
fn absent_malformed_or_out_of_range_attributes_refuse_before_any_write() {
    for (name, invalid) in [
        ("max_brightness", "0\n"),
        ("max_brightness", "01\n"),
        ("max_brightness", "+100\n"),
        ("max_brightness", "100 \n"),
        ("max_brightness", "4294967296\n"),
        ("max_brightness", "100"),
        ("max_brightness", "100\nextra\n"),
        ("brightness", "101\n"),
        ("brightness", "-1\n"),
        ("brightness", "01\n"),
        ("brightness", "25.0\n"),
        ("brightness", "25"),
        ("actual_brightness", "101\n"),
        ("actual_brightness", "-1\n"),
        ("actual_brightness", "01\n"),
        ("actual_brightness", "22"),
    ] {
        let tree = Tree::new(100, 25, Some(22));
        tree.write(name, invalid);
        let before = tree.all_bytes();
        let backend = tree.backend();
        assert!(backend.read().is_err(), "accepted {name}={invalid:?}");
        assert!(
            backend.set_percent(75).is_err(),
            "wrote after invalid {name}"
        );
        assert_eq!(tree.all_bytes(), before, "failure must precede every write");
    }
    for name in ["max_brightness", "brightness"] {
        let tree = Tree::new(100, 25, None);
        fs::remove_file(tree.attr(name)).unwrap();
        let before = tree.all_bytes();
        let backend = tree.backend();
        assert!(backend.read().is_err());
        assert!(backend.set_percent(75).is_err());
        assert_eq!(
            tree.all_bytes(),
            before,
            "missing files must not be created"
        );
    }
    let tree = Tree::new(100, 25, None);
    tree.write("max_brightness", &"9".repeat(4097));
    let before = tree.all_bytes();
    assert!(tree.backend().read().is_err());
    assert!(tree.backend().set_percent(75).is_err());
    assert_eq!(tree.all_bytes(), before);
}

#[test]
fn percent_above_hundred_refuses_without_touching_files() {
    let tree = Tree::new(100, 25, Some(22));
    let before = tree.all_bytes();
    for percent in [101, 255] {
        assert!(tree.backend().set_percent(percent).is_err());
        assert_eq!(tree.all_bytes(), before);
    }
}

#[test]
fn explicit_deployment_and_linux_kind_refuse_synthetic_tree_before_write() {
    let tree = Tree::new(100, 25, None);
    let before = tree.all_bytes();
    assert!(
        Backend::new(tree.class(), "panel0".into(), BackendKind::LinuxBacklight).is_err(),
        "production Linux selection must reject this non-sysfs temp tree"
    );
    assert_eq!(tree.all_bytes(), before);
    for device in ["", ".", "..", "../device", "panel/0", "/panel0", "panel\0x"] {
        assert!(Backend::new(tree.class(), device.into(), BackendKind::SyntheticTree).is_err());
    }
    assert!(
        Backend::new(
            "relative/class".into(),
            "panel0".into(),
            BackendKind::SyntheticTree
        )
        .is_err()
    );
    assert!(
        Backend::new(
            tree.class().join(".."),
            "panel0".into(),
            BackendKind::SyntheticTree
        )
        .is_err()
    );
    assert_eq!(tree.all_bytes(), before);
}

#[test]
fn synthetic_linked_attributes_detect_changed_max_after_single_write() {
    // Deliberately authored regular-file topology, never a Linux driver claim.
    // The one brightness write also changes max via a hardlink; post-readback
    // must detect that change, refuse success and perform no rescaled retry.
    let tree = Tree::new(100, 100, None);
    fs::remove_file(tree.attr("brightness")).unwrap();
    fs::hard_link(tree.attr("max_brightness"), tree.attr("brightness")).unwrap();
    assert!(
        tree.backend().set_percent(75).is_err(),
        "changed max must refuse success"
    );
    assert_eq!(
        tree.bytes("brightness"),
        Some(b"75\n".to_vec()),
        "exactly one write, no rescale/retry"
    );
    assert_eq!(tree.bytes("max_brightness"), Some(b"75\n".to_vec()));
}
