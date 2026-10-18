"""Selected guest-only installation of the three actual production DESTDIR rows.
Not a host installer. Caller supplies the actual generated manifest from the
bounded CMake helper and records this operation in its existing guest receipt.
"""
from pathlib import Path
import hashlib, os, shutil, stat


def pin(path):
    path = Path(path)
    with path.open("rb") as stream:
        return {"bytes": path.stat().st_size, "sha256": hashlib.file_digest(stream, "sha256").hexdigest()}


def install_settings(stage, rows):
    assert os.uname().machine == "aarch64" and os.getresuid() == (0, 0, 0)
    assert Path("/proc/1/comm").read_text().strip() == "systemd"
    assert set(rows) == {"usr/libexec/pokelinux/pokelinux-settings",
        "usr/share/pokelinux/appearance/PhoneAppearance.qml",
        "usr/share/applications/org.omarchy.Settings.desktop"}
    stage = Path(stage)
    assert stage.is_absolute() and stage != Path("/")
    # Authenticate every source and all existing destination ancestry before writes.
    for relative, row in rows.items():
        source, target = stage / relative, Path("/") / relative
        info = source.lstat()
        assert stat.S_ISREG(info.st_mode) and info.st_uid == info.st_gid == 0
        assert dict(pin(source), mode=stat.S_IMODE(info.st_mode)) == row
        for parent in reversed(target.parents):
            info = parent.lstat()
            assert stat.S_ISDIR(info.st_mode) and info.st_uid == info.st_gid == 0
            assert not stat.S_IMODE(info.st_mode) & 0o022
        if target.exists():
            info = target.lstat()
            assert stat.S_ISREG(info.st_mode) and info.st_uid == info.st_gid == 0
    for relative, row in rows.items():
        source, target = stage / relative, Path("/") / relative
        pending = target.with_name("." + target.name + ".arm-settings-new")
        assert not pending.exists()
        with source.open("rb") as src, pending.open("xb") as dst:
            shutil.copyfileobj(src, dst)
            dst.flush()
            os.fsync(dst.fileno())
        pending.chmod(row["mode"])
        os.chown(pending, 0, 0)
        os.replace(pending, target)
        parent_fd = os.open(target.parent, os.O_DIRECTORY | os.O_RDONLY | os.O_CLOEXEC)
        try:
            os.fsync(parent_fd)
        finally:
            os.close(parent_fd)
        assert dict(pin(target), mode=stat.S_IMODE(target.stat().st_mode)) == row
    return {"production_DESTDIR_rows_installed": rows,
        "service_units_added_or_activated": False, "normal_Main_or_window_executed": False}
