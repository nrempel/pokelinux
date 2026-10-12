use pokelinux_power::Application;
use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
pub struct TemporaryNode(PathBuf);
impl TemporaryNode {
    pub fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "pokelinux-power-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
    pub fn path(&self) -> PathBuf {
        self.0.join("node")
    }
    pub fn write(&self, value: &str) {
        fs::write(self.path(), value).unwrap();
    }
    pub fn read(&self) -> String {
        fs::read_to_string(self.path()).unwrap()
    }
    pub fn execute(&self, p: &Application) -> io::Result<()> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(self.path())?;
        file.write_all(p.value().as_bytes())?;
        file.sync_all()
    }
}
impl Drop for TemporaryNode {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
