use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Seek, SeekFrom},
    os::unix::fs::MetadataExt,
    path::{Component, Path},
};

pub const REQUIRED_GATES: &[&str] = &[
    "factory-integrity",
    "kernel-kmi",
    "vendor-userspace-reduction",
    "bionic-abi",
    "native-init",
    "selinux",
    "backend-discovery",
    "permissions",
    "native-service-integration",
    "physical-device",
];

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub schema: u32,
    pub id: String,
    pub device: String,
    pub architecture: String,
    pub component_set: String,
    pub evidence: String,
    pub kernel_release: String,
    pub artifacts: Vec<ArtifactPin>,
    pub resources: Vec<String>,
    pub service_contracts: Vec<ServiceContract>,
    pub gates: Vec<Gate>,
    pub capabilities: Vec<Capability>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceContract {
    pub id: String,
    pub owns: Vec<String>,
    pub dependencies: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactPin {
    pub id: String,
    pub kind: String,
    pub sha256: Option<String>,
    pub source: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Gate {
    pub id: String,
    pub scope: String,
    pub state: String,
    pub evidence: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Capability {
    pub id: String,
    pub available: bool,
    pub qualification: String,
    pub detail: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Inventory {
    pub schema: u32,
    pub profile: String,
    pub device: String,
    pub architecture: String,
    pub component_set: String,
    pub artifacts: Vec<Artifact>,
    pub packages: Vec<Package>,
    pub files: Vec<Payload>,
    pub services: Vec<Service>,
    pub enable_capabilities: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub id: String,
    pub path: String,
    pub component_set: String,
    pub kernel_release: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
    pub name: String,
    pub version: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub architecture: String,
    pub component_set: String,
    pub metadata: String,
    pub dependencies: Vec<Dependency>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Payload {
    pub path: String,
    pub sha256: String,
    pub mode: u32,
    pub package: Option<String>,
    pub kind: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Service {
    pub id: String,
    pub runtime: String,
    pub executable: String,
    pub package: String,
    pub owns: Vec<String>,
    pub dependencies: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Issue {
    pub code: String,
    pub subject: String,
    pub scope: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Report {
    pub inventory_valid: bool,
    pub deployment_admitted: bool,
    pub issues: Vec<Issue>,
}

const MAX_ITEMS: usize = 4096;
const MAX_FILE: u64 = 128 * 1024 * 1024;
const MAX_DEPTH: usize = 32;

struct Check {
    issues: Vec<Issue>,
    inventory_valid: bool,
}
impl Check {
    fn error(&mut self, code: &str, subject: &str) {
        self.inventory_valid = false;
        self.block(code, subject, "inventory");
    }
    fn block(&mut self, code: &str, subject: &str, scope: &str) {
        self.issues.push(Issue {
            code: code.into(),
            subject: subject.into(),
            scope: scope.into(),
        });
    }
}

/// Read-only consistency report for an immutable staging tree. It is not a
/// reusable launch permit, signature verification, or runtime ownership proof.
pub fn validate(profile: &Profile, inventory: &Inventory, root: &Path) -> Report {
    let mut c = Check {
        issues: vec![],
        inventory_valid: true,
    };
    if profile.schema != 1 || inventory.schema != 1 {
        c.error("schema", "current schema is 1");
    }
    if inventory.profile != profile.id
        || inventory.device != profile.device
        || inventory.architecture != profile.architecture
    {
        c.error("profile-mismatch", &inventory.profile);
    }
    if inventory.component_set != profile.component_set {
        c.error("component-set", "inventory");
    }
    if !matches!(profile.architecture.as_str(), "aarch64" | "x86_64")
        || profile.id.is_empty()
        || profile.device.is_empty()
        || profile.component_set.is_empty()
        || profile.kernel_release.is_empty()
    {
        c.error("profile-shape", &profile.id);
    }
    if !matches!(
        profile.evidence.as_str(),
        "recorded-original" | "authored-fixture"
    ) {
        c.error("profile-evidence", &profile.evidence);
    }
    if profile.evidence == "authored-fixture" {
        c.block("fixture-evidence", &profile.id, "qualification");
    }
    let sizes = [
        profile.artifacts.len(),
        profile.service_contracts.len(),
        profile.resources.len(),
        profile.gates.len(),
        profile.capabilities.len(),
        inventory.artifacts.len(),
        inventory.packages.len(),
        inventory.files.len(),
        inventory.services.len(),
        inventory.enable_capabilities.len(),
    ];
    if sizes.iter().any(|&n| n > MAX_ITEMS) {
        c.error("inventory-bound", "maximum 4096 items per collection");
        return finish(c);
    }
    if profile.artifacts.is_empty()
        || profile.resources.is_empty()
        || inventory.packages.is_empty()
        || inventory.files.is_empty()
        || inventory.services.is_empty()
    {
        c.error("empty-inventory", "required collection");
    }
    let mut gates = BTreeSet::new();
    for g in &profile.gates {
        let expected_scope = if g.id == "physical-device" {
            "physical"
        } else {
            "offline"
        };
        if !REQUIRED_GATES.contains(&g.id.as_str())
            || !gates.insert(g.id.as_str())
            || g.scope != expected_scope
            || g.evidence.is_empty()
            || !matches!(g.state.as_str(), "qualified" | "unresolved")
        {
            c.error("gate-shape", &g.id);
        }
        if g.state != "qualified" {
            c.block("unresolved-gate", &g.id, expected_scope);
        }
    }
    for id in REQUIRED_GATES {
        if !gates.contains(id) {
            c.block(
                "missing-gate",
                id,
                if *id == "physical-device" {
                    "physical"
                } else {
                    "offline"
                },
            );
        }
    }
    let mut caps = BTreeMap::new();
    for cap in &profile.capabilities {
        if caps.insert(cap.id.as_str(), cap).is_some()
            || cap.id.is_empty()
            || cap.detail.is_empty()
            || !matches!(
                cap.qualification.as_str(),
                "unavailable" | "component-test" | "installed-test" | "hardware-verified"
            )
        {
            c.error("capability-shape", &cap.id);
        }
        // A component/installed fixture never grants physical device availability.
        if cap.available && cap.qualification != "hardware-verified" {
            c.error("capability-evidence", &cap.id);
        }
    }
    let mut enabled = BTreeSet::new();
    for id in &inventory.enable_capabilities {
        if !enabled.insert(id.as_str())
            || !caps
                .get(id.as_str())
                .is_some_and(|cap| cap.available && cap.qualification == "hardware-verified")
        {
            c.error("unsupported-capability", id);
        }
    }
    let mut files = BTreeMap::new();
    let metadata_targets: BTreeMap<_, _> = inventory
        .packages
        .iter()
        .map(|package| (package.metadata.as_str(), package))
        .collect();
    let mut metadata_checks = BTreeMap::new();
    for f in &inventory.files {
        if !safe_path(&f.path) {
            c.error("unsafe-path", &f.path);
            continue;
        }
        if files.insert(f.path.as_str(), f).is_some() {
            c.error("duplicate-file", &f.path);
            continue;
        }
        if !digest_shape(&f.sha256) {
            c.error("file-hash", &f.path);
        }
        if !matches!(
            f.kind.as_str(),
            "kernel" | "module" | "firmware" | "native-executable" | "native-resource"
        ) {
            c.error("file-kind", &f.path);
        }
        if f.mode & !0o777 != 0 || f.mode & 0o022 != 0 || f.mode & 0o400 == 0 {
            c.error("file-mode", &f.path);
        }
        match inspect_regular(
            root,
            &f.path,
            &profile.architecture,
            f.kind == "native-executable",
            metadata_targets.get(f.path.as_str()).copied(),
        ) {
            Ok(inspection) => {
                if inspection.mode != f.mode {
                    c.error("file-mode", &f.path);
                }
                if inspection.sha256 != f.sha256 {
                    c.error("file-hash", &f.path);
                }
                if f.kind == "native-executable"
                    && (!inspection.native_elf || inspection.mode & 0o111 == 0)
                {
                    c.error("native-elf", &f.path);
                }
                if metadata_targets.contains_key(f.path.as_str()) {
                    metadata_checks.insert(f.path.as_str(), inspection.metadata_valid);
                }
            }
            Err(code) => c.error(code, &f.path),
        }
    }
    let mut actual = BTreeSet::new();
    if let Err(code) = walk(root, root, 0, &mut actual) {
        c.error(code, "staging-root");
    }
    for name in actual {
        if !files.contains_key(name.as_str()) {
            c.error("unlisted-file", &name);
        }
    }
    let mut pins = BTreeMap::new();
    for pin in &profile.artifacts {
        if pins.insert(pin.id.as_str(), pin).is_some()
            || pin.id.is_empty()
            || !matches!(pin.kind.as_str(), "kernel" | "module" | "firmware")
            || pin.source.is_empty()
        {
            c.error("artifact-shape", &pin.id);
        }
        if !pin.sha256.as_deref().is_some_and(digest_shape) {
            c.block("unpinned-artifact", &pin.id, "offline");
        }
    }
    let mut artifacts = BTreeSet::new();
    let mut artifact_paths = BTreeSet::new();
    for a in &inventory.artifacts {
        if !artifacts.insert(a.id.as_str()) || !artifact_paths.insert(a.path.as_str()) {
            c.error("duplicate-artifact", &a.id);
        }
        if a.component_set != profile.component_set {
            c.error("component-set", &a.id);
        }
        if a.kernel_release != profile.kernel_release {
            c.error("kernel-release", &a.id);
        }
        match (pins.get(a.id.as_str()), files.get(a.path.as_str())) {
            (Some(pin), Some(f)) if pin.kind == f.kind && f.package.is_none() => {
                if pin.sha256.as_ref().is_some_and(|hash| hash != &f.sha256) {
                    c.error("artifact-hash", &a.id);
                }
            }
            _ => c.error("artifact-binding", &a.id),
        }
    }
    for id in pins.keys() {
        if !artifacts.contains(id) {
            c.error("missing-artifact", id);
        }
    }
    for f in &inventory.files {
        if matches!(f.kind.as_str(), "kernel" | "module" | "firmware")
            && !artifact_paths.contains(f.path.as_str())
        {
            c.error("artifact-binding", &f.path);
        }
    }
    let mut packages = BTreeMap::new();
    let mut metadata_paths = BTreeSet::new();
    for p in &inventory.packages {
        if p.name.is_empty()
            || p.version.is_empty()
            || packages.insert(p.name.as_str(), p).is_some()
            || !metadata_paths.insert(p.metadata.as_str())
        {
            c.error("package-shape", &p.name);
        }
        if p.component_set != profile.component_set {
            c.error("component-set", &p.name);
        }
        if p.architecture != profile.architecture {
            c.error("package-architecture", &p.name);
        }
        if !files.get(p.metadata.as_str()).is_some_and(|f| {
            f.kind == "native-resource" && f.package.as_deref() == Some(p.name.as_str())
        }) {
            c.error("package-metadata", &p.name);
        }
        if metadata_checks.get(p.metadata.as_str()) != Some(&true) {
            c.error("package-metadata", &p.name);
        }
    }
    for p in &inventory.packages {
        let mut deps = BTreeSet::new();
        for d in &p.dependencies {
            if !deps.insert(d.name.as_str())
                || d.name == p.name
                || !packages
                    .get(d.name.as_str())
                    .is_some_and(|other| other.version == d.version)
            {
                c.error("package-dependency", &p.name);
            }
        }
    }
    for f in &inventory.files {
        if matches!(f.kind.as_str(), "native-executable" | "native-resource")
            && !f
                .package
                .as_ref()
                .is_some_and(|name| packages.contains_key(name.as_str()))
        {
            c.error("file-package", &f.path);
        }
    }
    let mut services = BTreeMap::new();
    let mut owners: BTreeMap<&str, usize> = BTreeMap::new();
    let mut resources = BTreeSet::new();
    for id in &profile.resources {
        if id.is_empty() || !resources.insert(id.as_str()) {
            c.error("resource-shape", id);
        }
    }
    for s in &inventory.services {
        if s.id.is_empty() || services.insert(s.id.as_str(), s).is_some() {
            c.error("service-shape", &s.id);
        }
        if s.runtime != "native-rust" {
            c.error("non-native-runtime", &s.id);
        }
        if !packages.contains_key(s.package.as_str())
            || !files.get(s.executable.as_str()).is_some_and(|f| {
                f.kind == "native-executable" && f.package.as_deref() == Some(s.package.as_str())
            })
        {
            c.error("service-executable", &s.id);
        }
        if s.owns.is_empty() || s.owns.len() > MAX_ITEMS || s.dependencies.len() > MAX_ITEMS {
            c.error("service-shape", &s.id);
        }
        for id in &s.owns {
            *owners.entry(id.as_str()).or_default() += 1;
            if !resources.contains(id.as_str()) {
                c.error("resource-owner", id);
            }
        }
    }
    for id in resources {
        if owners.get(id) != Some(&1) {
            c.error("resource-owner", id);
        }
    }
    for s in &inventory.services {
        let mut deps = BTreeSet::new();
        for id in &s.dependencies {
            if id == &s.id || !deps.insert(id.as_str()) || !services.contains_key(id.as_str()) {
                c.error("service-dependency", &s.id);
            }
        }
    }
    let mut contracts = BTreeMap::new();
    let mut contract_owners: BTreeMap<&str, usize> = BTreeMap::new();
    if profile.service_contracts.is_empty() {
        c.error("service-contract", "required profile contracts");
    }
    for contract in &profile.service_contracts {
        if contract.id.is_empty()
            || contracts.insert(contract.id.as_str(), contract).is_some()
            || contract.owns.is_empty()
            || contract.owns.len() > MAX_ITEMS
            || contract.dependencies.len() > MAX_ITEMS
        {
            c.error("service-contract", &contract.id);
            continue;
        }
        for id in &contract.owns {
            *contract_owners.entry(id.as_str()).or_default() += 1;
            if !profile.resources.iter().any(|r| r == id) {
                c.error("service-contract", id);
            }
        }
        let expected_owners: BTreeSet<_> = contract.owns.iter().collect();
        let expected_deps: BTreeSet<_> = contract.dependencies.iter().collect();
        if expected_owners.len() != contract.owns.len()
            || expected_deps.len() != contract.dependencies.len()
            || !services.get(contract.id.as_str()).is_some_and(|service| {
                service.owns.len() == expected_owners.len()
                    && service.dependencies.len() == expected_deps.len()
                    && service.owns.iter().collect::<BTreeSet<_>>() == expected_owners
                    && service.dependencies.iter().collect::<BTreeSet<_>>() == expected_deps
            })
        {
            c.error("service-contract", &contract.id);
        }
    }
    for id in &profile.resources {
        if contract_owners.get(id.as_str()) != Some(&1) {
            c.error("service-contract", id);
        }
    }
    for contract in &profile.service_contracts {
        if contract
            .dependencies
            .iter()
            .any(|id| id == &contract.id || !contracts.contains_key(id.as_str()))
        {
            c.error("service-contract", &contract.id);
        }
    }
    for id in services.keys() {
        if !contracts.contains_key(id) {
            c.error("service-contract", id);
        }
    }
    // Bounded topological elimination avoids recursion on authored dependency input.
    let mut remaining: BTreeSet<_> = services.keys().copied().collect();
    loop {
        let ready: Vec<_> = remaining
            .iter()
            .copied()
            .filter(|id| {
                services[id]
                    .dependencies
                    .iter()
                    .all(|d| !remaining.contains(d.as_str()))
            })
            .collect();
        if ready.is_empty() {
            break;
        }
        for id in ready {
            remaining.remove(id);
        }
    }
    if !remaining.is_empty() {
        c.error("service-cycle", "service dependencies");
    }
    finish(c)
}
fn finish(c: Check) -> Report {
    Report {
        inventory_valid: c.inventory_valid,
        deployment_admitted: c.inventory_valid && c.issues.is_empty(),
        issues: c.issues,
    }
}
fn digest_shape(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn safe_path(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 4096
        && !s.contains('\0')
        && !s.contains('\\')
        && !s.starts_with('/')
        && s.split('/').all(|p| !p.is_empty() && p != "." && p != "..")
        && Path::new(s)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}
struct Inspection {
    sha256: String,
    mode: u32,
    native_elf: bool,
    metadata_valid: bool,
}

fn inspect_regular(
    root: &Path,
    name: &str,
    architecture: &str,
    inspect_elf: bool,
    metadata: Option<&Package>,
) -> Result<Inspection, &'static str> {
    const MAX_METADATA: u64 = 1024 * 1024;
    let mut path = root.to_path_buf();
    let root_meta = fs::symlink_metadata(root).map_err(|_| "missing-file")?;
    if !root_meta.is_dir() || root_meta.file_type().is_symlink() {
        return Err("unsafe-file");
    }
    let parts: Vec<_> = name.split('/').collect();
    for (i, p) in parts.iter().enumerate() {
        path.push(p);
        let meta = fs::symlink_metadata(&path).map_err(|_| "missing-file")?;
        if meta.file_type().is_symlink()
            || (i + 1 < parts.len() && !meta.is_dir())
            || (i + 1 == parts.len() && !meta.is_file())
        {
            return Err("unsafe-file");
        }
    }
    let mut file = fs::File::open(&path).map_err(|_| "unreadable-file")?;
    let before = file.metadata().map_err(|_| "unreadable-file")?;
    if !before.is_file() || before.len() > MAX_FILE || before.nlink() != 1 {
        return Err("unsafe-file");
    }
    if metadata.is_some() && before.len() > MAX_METADATA {
        return Err("metadata-bound");
    }
    // Ordinary bytes are never retained after hashing. One fixed chunk is
    // independent of file count and aggregate payload size.
    let mut digest = Sha256::new();
    let mut chunk = [0u8; 64 * 1024];
    let mut total = 0u64;
    loop {
        let n = file.read(&mut chunk).map_err(|_| "unreadable-file")?;
        if n == 0 {
            break;
        }
        total = total.checked_add(n as u64).ok_or("changed-file")?;
        if total > MAX_FILE {
            return Err("changed-file");
        }
        digest.update(&chunk[..n]);
    }
    let elf_valid = !inspect_elf || native_elf(&mut file, before.len(), architecture);
    // At most one bounded metadata body is alive. Only its validation boolean
    // survives this function; a complete file-body map is never built.
    let metadata_valid = if let Some(package) = metadata {
        file.seek(SeekFrom::Start(0))
            .map_err(|_| "unreadable-file")?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(before.len() as usize)
            .map_err(|_| "unreadable-file")?;
        bytes.resize(before.len() as usize, 0);
        file.read_exact(&mut bytes).map_err(|_| "unreadable-file")?;
        package_metadata(&bytes, package)
    } else {
        false
    };
    let after = fs::symlink_metadata(path).map_err(|_| "changed-file")?;
    let descriptor_after = file.metadata().map_err(|_| "changed-file")?;
    let identity = |m: &fs::Metadata| {
        (
            m.dev(),
            m.ino(),
            m.len(),
            m.mode(),
            m.mtime(),
            m.mtime_nsec(),
            m.ctime(),
            m.ctime_nsec(),
        )
    };
    if total != before.len()
        || identity(&before) != identity(&after)
        || identity(&before) != identity(&descriptor_after)
    {
        return Err("changed-file");
    }
    Ok(Inspection {
        sha256: format!("{:x}", digest.finalize()),
        mode: before.mode() & 0o7777,
        native_elf: elf_valid,
        metadata_valid,
    })
}
fn walk(
    root: &Path,
    dir: &Path,
    depth: usize,
    files: &mut BTreeSet<String>,
) -> Result<(), &'static str> {
    if depth > MAX_DEPTH || files.len() > MAX_ITEMS {
        return Err("inventory-bound");
    }
    let meta = fs::symlink_metadata(dir).map_err(|_| "unreadable-file")?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err("unsafe-file");
    }
    for entry in fs::read_dir(dir).map_err(|_| "unreadable-file")? {
        let path = entry.map_err(|_| "unreadable-file")?.path();
        let meta = fs::symlink_metadata(&path).map_err(|_| "unreadable-file")?;
        if meta.file_type().is_symlink() || (!meta.is_file() && !meta.is_dir()) {
            return Err("unsafe-file");
        }
        if meta.is_dir() {
            walk(root, &path, depth + 1, files)?;
        } else {
            let name = path
                .strip_prefix(root)
                .map_err(|_| "unsafe-path")?
                .to_str()
                .ok_or("unsafe-path")?
                .to_owned();
            if !safe_path(&name) {
                return Err("unsafe-path");
            }
            files.insert(name);
            if files.len() > MAX_ITEMS {
                return Err("inventory-bound");
            }
        }
    }
    Ok(())
}
fn package_metadata(bytes: &[u8], p: &Package) -> bool {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return false;
    };
    let mut fields: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for line in text.lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once(" = ") else {
            return false;
        };
        fields.entry(key).or_default().push(value);
    }
    for (key, value) in [
        ("pkgname", p.name.as_str()),
        ("pkgver", p.version.as_str()),
        ("arch", p.architecture.as_str()),
    ] {
        if fields.get(key) != Some(&vec![value]) {
            return false;
        }
    }
    let expected: BTreeSet<_> = p
        .dependencies
        .iter()
        .map(|d| format!("{}={}", d.name, d.version))
        .collect();
    let deps = fields.get("depend").cloned().unwrap_or_default();
    deps.len() == expected.len()
        && deps
            .iter()
            .map(|s| (*s).to_owned())
            .collect::<BTreeSet<_>>()
            == expected
}
fn read_at<const N: usize>(file: &mut fs::File, offset: u64) -> Option<[u8; N]> {
    file.seek(SeekFrom::Start(offset)).ok()?;
    let mut bytes = [0u8; N];
    file.read_exact(&mut bytes).ok()?;
    Some(bytes)
}

fn native_elf(file: &mut fs::File, length: u64, arch: &str) -> bool {
    let Some(bytes) = read_at::<64>(file, 0) else {
        return false;
    };
    if &bytes[..4] != b"\x7fELF"
        || bytes[4] != 2
        || bytes[5] != 1
        || bytes[6] != 1
        || u32::from_le_bytes(bytes[20..24].try_into().unwrap()) != 1
    {
        return false;
    }
    let machine = u16::from_le_bytes(bytes[18..20].try_into().unwrap());
    let expected = match arch {
        "aarch64" => 183,
        "x86_64" => 62,
        _ => return false,
    };
    let kind = u16::from_le_bytes(bytes[16..18].try_into().unwrap());
    if machine != expected
        || ![2, 3].contains(&kind)
        || u16::from_le_bytes(bytes[52..54].try_into().unwrap()) != 64
    {
        return false;
    }
    let entry = u64::from_le_bytes(bytes[24..32].try_into().unwrap());
    let start = u64::from_le_bytes(bytes[32..40].try_into().unwrap());
    let size = u16::from_le_bytes(bytes[54..56].try_into().unwrap()) as u64;
    let count = u16::from_le_bytes(bytes[56..58].try_into().unwrap()) as u64;
    if start < 64
        || size != 56
        || count == 0
        || count > 128
        || start
            .checked_add(size * count)
            .is_none_or(|end| end > length)
    {
        return false;
    }
    let expected_interp = if arch == "aarch64" {
        b"/lib/ld-linux-aarch64.so.1\0".as_slice()
    } else {
        b"/lib64/ld-linux-x86-64.so.2\0".as_slice()
    };
    let mut interpreter_seen = false;
    let mut previous_load_address = None;
    let mut entry_backed_by_executable_bytes = false;
    for i in 0..count {
        let Some(h) = read_at::<56>(file, start + i * size) else {
            return false;
        };
        let ph_type = u32::from_le_bytes(h[..4].try_into().unwrap());
        let flags = u32::from_le_bytes(h[4..8].try_into().unwrap());
        let offset = u64::from_le_bytes(h[8..16].try_into().unwrap());
        let address = u64::from_le_bytes(h[16..24].try_into().unwrap());
        let file_size = u64::from_le_bytes(h[32..40].try_into().unwrap());
        let memory_size = u64::from_le_bytes(h[40..48].try_into().unwrap());
        let align = u64::from_le_bytes(h[48..56].try_into().unwrap());
        if ph_type == 1 {
            // This validates load geometry, not relocation/DT_NEEDED, page-size
            // qualification, loader success or runtime service ownership.
            if file_size > memory_size
                || offset.checked_add(file_size).is_none_or(|end| end > length)
                || address.checked_add(memory_size).is_none()
                || (align > 1 && (!align.is_power_of_two() || address % align != offset % align))
                || previous_load_address.is_some_and(|previous| address < previous)
            {
                return false;
            }
            previous_load_address = Some(address);
            if flags & 1 != 0 && file_size > 0 && entry >= address && entry < address + file_size {
                entry_backed_by_executable_bytes = true;
            }
        }
        if ph_type == 3 {
            if interpreter_seen
                || previous_load_address.is_some()
                || file_size != expected_interp.len() as u64
                || offset.checked_add(file_size).is_none_or(|end| end > length)
            {
                return false;
            }
            let mut interpreter = [0u8; 256];
            if file.seek(SeekFrom::Start(offset)).is_err()
                || file
                    .read_exact(&mut interpreter[..file_size as usize])
                    .is_err()
                || &interpreter[..file_size as usize] != expected_interp
            {
                return false;
            }
            interpreter_seen = true;
        }
    }
    interpreter_seen && entry_backed_by_executable_bytes
}
