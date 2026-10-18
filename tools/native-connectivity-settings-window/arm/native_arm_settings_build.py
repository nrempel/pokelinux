"""Proposed ARM normal Settings CMake target and production DESTDIR staging.
Not executed: existing Scope-owned run() supplies original preparation cutoffs.
No Rust build, component test, app, compositor or worker execution in this helper.
"""
from pathlib import Path
import hashlib, json, os, shlex, shutil, stat

def pin(path):
    path = Path(path)
    with path.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    return {"bytes": path.stat().st_size, "sha256": digest}

def build_settings(results, selection, run):
    results = Path(results)
    config = selection["normal_settings_build"]
    source = results / config["source_root_relative"]
    assert source.is_absolute()
    assert os.getuid() == 0 and os.uname().machine == "aarch64"
    before = {name: pin(source / name) for name in config["source_files"]}
    assert before == config["source_files"]
    archives = config["archives"]
    assert set(archives) == {"settings", "appearance", "network_status"}
    selected = {}
    for role, row in archives.items():
        path = results / row["relative_path"]
        assert pin(path) == {"bytes": row["bytes"], "sha256": row["sha256"]}
        selected[role] = path
    qt = run(["/usr/bin/pkg-config", "--modversion", "Qt6Core", "Qt6Gui", "Qt6Qml", "Qt6Quick"], "normal-Settings-build-Qt-version").splitlines()
    assert qt == [config["expected_Qt_version"]] * 4
    build = Path("/tmp/pokelinux-settings-current-qt-build")
    assert not build.exists()
    configure = ["/usr/bin/cmake", "-S", str(source / "native-settings-ui"), "-B", str(build),
        "-G", "Ninja", "-DCMAKE_BUILD_TYPE=Release", "-DCMAKE_CXX_COMPILER=/usr/bin/g++",
        "-DCMAKE_MAKE_PROGRAM=/usr/bin/ninja", "-DCMAKE_EXPORT_COMPILE_COMMANDS=ON",
        "-DCMAKE_INSTALL_PREFIX=/usr",
        "-DCMAKE_CXX_FLAGS=-fPIC"]
    # PIC is an explicit setup selection after the older binary's actual
    # protected-symbol COPY-relocation refusal. Normal CMake/source are unchanged.
    for role, prefix in [("settings", "SETTINGS"), ("appearance", "APPEARANCE"), ("network_status", "NETWORK_STATUS")]:
        configure += ["-DPOKELINUX_" + prefix + "_CLIENT_ARCHIVE=" + str(selected[role]),
            "-DPOKELINUX_" + prefix + "_CLIENT_ARCHIVE_SHA256=" + archives[role]["sha256"]]
    run(configure, "normal-Settings-CMake-configure", seconds=120)
    compile_argv = ["/usr/bin/cmake", "--build", str(build), "--parallel", "4", "--target", "pokelinux-settings", "--verbose"]
    output = run(compile_argv, "normal-Settings-CMake-target-build", seconds=300, limit=2097152)
    link_lines = [line for line in output.splitlines() if " -o pokelinux-settings " in line]
    assert len(link_lines) == 1
    tokens = shlex.split(link_lines[0])
    assert all(tokens.count(str(path)) == 1 for path in selected.values())
    assert "cargo" not in output.lower() and "Qt6Test" not in link_lines[0]
    binary = build / "pokelinux-settings"
    with binary.open("rb") as stream:
        header = stream.read(20)
    assert header[:4] == b"\x7fELF" and int.from_bytes(header[18:20], "little") == 183
    loader = run(["/usr/bin/ldd", "-r", str(binary)], "normal-Settings-built-target-loader")
    assert "not found" not in loader.lower() and "undefined symbol" not in loader.lower()
    assert "copy relocation" not in loader.lower() and "error due to" not in loader.lower()
    assert "libQt6Test" not in loader
    assert before == {name: pin(source / name) for name in config["source_files"]}
    for role, path in selected.items():
        assert pin(path) == {"bytes": archives[role]["bytes"], "sha256": archives[role]["sha256"]}
    stage = results / "generated/settings-production-stage"
    assert not stage.exists()
    install_argv = ["/usr/bin/env", "DESTDIR=" + str(stage), "/usr/bin/cmake", "--install", str(build)]
    run(install_argv, "normal-Settings-production-DESTDIR", seconds=30)
    expected = {
        "usr/libexec/pokelinux/pokelinux-settings": dict(pin(binary), mode=0o755),
        "usr/share/pokelinux/appearance/PhoneAppearance.qml": dict(pin(source / "native-app/PhoneAppearance.qml"), mode=0o644),
        "usr/share/applications/org.omarchy.Settings.desktop": dict(pin(source / "native-settings-ui/org.omarchy.Settings.desktop"), mode=0o644),
    }
    staged = {}
    for path in sorted(stage.rglob("*")):
        info = path.lstat()
        assert info.st_uid == info.st_gid == 0 and not stat.S_ISLNK(info.st_mode)
        if stat.S_ISDIR(info.st_mode):
            assert stat.S_IMODE(info.st_mode) == 0o755
        else:
            assert stat.S_ISREG(info.st_mode)
            staged[str(path.relative_to(stage))] = dict(pin(path), mode=stat.S_IMODE(info.st_mode))
    assert staged == expected
    # Real-filesystem installation is a separately selected guest preparation step.
    destination = results / "generated/settings/pokelinux-settings"
    assert not destination.exists()
    destination.parent.mkdir(parents=True, exist_ok=False)
    shutil.copyfile(binary, destination)
    destination.chmod(0o755)
    assert pin(destination) == pin(binary)
    proof = {"normal_CMake_only_target": "pokelinux-settings", "normal_sources_unchanged": True,
        "actual_Qt_versions": qt, "configure_argv": configure, "build_argv": compile_argv,
        "explicit_selected_CXX_flags": "-fPIC", "actual_verbose_link_line": link_lines[0],
        "all_three_frozen_Rust_archives_linked_once": True, "Rust_not_rebuilt": True,
        "components_or_app_or_Main_executed_by_build": False, "source_files": before,
        "archives": archives, "actual_compile_commands": json.loads((build / "compile_commands.json").read_text()),
        "generated_ELF": pin(destination), "ELF_machine": 183,
        "loader_sha256": hashlib.sha256(loader.encode()).hexdigest(),
        "production_DESTDIR_argv": install_argv, "production_staged_rows": expected,
        "staged_root_relative": str(stage.relative_to(results)),
        "real_filesystem_install_or_normal_window_executed": False}
    row = {"relative_path": str(destination.relative_to(results)), **pin(destination), "ELF_machine": 183,
        "lineage": "Unchanged normal Settings CMake target built in selected Qt guest with the three frozen Rust archives"}
    return row, proof
