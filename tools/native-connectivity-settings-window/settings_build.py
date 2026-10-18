"""Build the unchanged normal Settings target against the selected guest Qt.
Generated executable only; no Rust rebuild, component test, app or window launch.
"""
from pathlib import Path
import hashlib, json, shlex, shutil

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
        "-DCMAKE_CXX_FLAGS=-fPIC"]
    # PIC is an explicit setup selection after the older binary's actual
    # protected-symbol COPY-relocation refusal. Normal CMake/source are unchanged.
    for role, prefix in [("settings", "SETTINGS"), ("appearance", "APPEARANCE"), ("network_status", "NETWORK_STATUS")]:
        configure += ["-DPOKELINUX_" + prefix + "_CLIENT_ARCHIVE=" + str(selected[role]),
            "-DPOKELINUX_" + prefix + "_CLIENT_ARCHIVE_SHA256=" + archives[role]["sha256"]]
    run(configure, "normal-Settings-CMake-configure", seconds=120)
    compile_argv = ["/usr/bin/cmake", "--build", str(build), "--parallel", "6", "--target", "pokelinux-settings", "--verbose"]
    output = run(compile_argv, "normal-Settings-CMake-target-build", seconds=180, limit=2097152)
    link_lines = [line for line in output.splitlines() if " -o pokelinux-settings " in line]
    assert len(link_lines) == 1
    tokens = shlex.split(link_lines[0])
    assert all(tokens.count(str(path)) == 1 for path in selected.values())
    assert "cargo" not in output.lower() and "Qt6Test" not in link_lines[0]
    binary = build / "pokelinux-settings"
    with binary.open("rb") as stream:
        header = stream.read(20)
    assert header[:4] == b"\x7fELF" and int.from_bytes(header[18:20], "little") == 62
    loader = run(["/usr/bin/ldd", "-r", str(binary)], "normal-Settings-built-target-loader")
    assert "not found" not in loader.lower() and "undefined symbol" not in loader.lower()
    assert "copy relocation" not in loader.lower() and "error due to" not in loader.lower()
    assert "libQt6Test" not in loader
    assert before == {name: pin(source / name) for name in config["source_files"]}
    for role, path in selected.items():
        assert pin(path) == {"bytes": archives[role]["bytes"], "sha256": archives[role]["sha256"]}
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
        "generated_ELF": pin(destination), "ELF_machine": 62,
        "loader_sha256": hashlib.sha256(loader.encode()).hexdigest()}
    row = {"relative_path": str(destination.relative_to(results)), **pin(destination), "ELF_machine": 62,
        "lineage": "Unchanged normal Settings CMake target built in selected Qt guest with the three frozen Rust archives"}
    return row, proof
