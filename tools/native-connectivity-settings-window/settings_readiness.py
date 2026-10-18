"""Fixed normal Settings label-state observation using the unchanged GUI9 capture."""
from pathlib import Path
import hashlib, json, math, os, shlex
import display
from owned import now

RESOURCE = "01445eaf0fcf25f75acfc1ea622df97c00a9efb261cdb0ed7d778f314010d324"
COLORS = {"foreground": (202, 204, 204), "muted": (112, 120, 128)}
LABELS = {"Enable": (80, 309, 140, 329), "Disable": (267.25, 309, 335.25, 329)}
STATE = (18, 237, 393, 251)

def sha(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()

def build(results, run, binaries):
    results, binaries = Path(results), Path(binaries)
    source = results / "source"
    names = ["raw-output-capture.cpp", "wlr-screencopy-unstable-v1.xml"]
    before = {name: sha(source / name) for name in names}
    assert before == {"raw-output-capture.cpp": "e9aebd5eee61ebfeb119cb093c421c7a1af095658b4f46d4faaed0b842f7ef7d",
        "wlr-screencopy-unstable-v1.xml": "0f50f6187965b02782a12ba346ad950d6031454f6af6e1ac5b341a279ef16d56"}
    output = results / "native-readiness-build"
    output.mkdir()
    flags = shlex.split(run(["/usr/bin/pkg-config", "--cflags", "--libs", "Qt6Gui", "wayland-client", "libdrm"], "native-capture-SDK-flags", seconds=5))
    protocol = source / names[1]
    run(["/usr/bin/wayland-scanner", "client-header", str(protocol), str(output / "wlr-screencopy-client-protocol.h")], "native-capture-header", seconds=30)
    run(["/usr/bin/wayland-scanner", "private-code", str(protocol), str(output / "wlr-screencopy-protocol.c")], "native-capture-protocol", seconds=30)
    run(["/usr/bin/gcc", "-std=c11", "-fPIC", "-c", str(output / "wlr-screencopy-protocol.c"), "-o", str(output / "wlr-screencopy-protocol.o")], "native-capture-protocol-build", seconds=60)
    binary = binaries / "pixel10-raw-output-capture"
    argv = ["/usr/bin/g++", "-std=c++17", "-O0", "-Wall", "-Wextra", "-Werror", "-fPIC", "-I" + str(output), str(source / names[0]), str(output / "wlr-screencopy-protocol.o"), "-o", str(binary), *flags]
    run(argv, "native-capture-build", seconds=180)
    binary.chmod(0o755)
    os.chown(binary, 0, 0)
    header = binary.read_bytes()[:20]
    assert header[:4] == b"\x7fELF" and int.from_bytes(header[18:20], "little") == 62
    loader = run(["/usr/bin/ldd", "-r", str(binary)], "native-capture-loader")
    assert "not found" not in loader.lower() and "undefined symbol" not in loader.lower()
    assert before == {name: sha(source / name) for name in names}
    return {"source_files": before, "actual_build_argv": argv, "ELF_machine": 62,
        "generated_ELF": {"bytes": binary.stat().st_size, "sha256": sha(binary)},
        "actual_loader_sha256": hashlib.sha256(loader.encode()).hexdigest(), "GUI9_capture_source_and_XML_unchanged": True}

def observe(results, binaries, run, env, pid, configured, original_until, qualify, focus, record, app_sha):
    from PIL import Image
    results = Path(results)
    assert sha(results / "source/qt-source/native-settings-ui/SettingsView.qml") == RESOURCE
    start = now()
    until = min(original_until, start + 1_000_000_000)
    record.update(qualified=False, pid=pid, expected_configured=configured,
        started_at_ns=start, original_parent_until_ns=original_until, original_readiness_until_ns=until,
        normal_QML_sha256=RESOURCE, normal_app_ELF_sha256=app_sha, max_frames=8, frames=[],
        normal_app_getters_or_OCR_selected=False, Root_original_PNG_semantic_review_required=True)
    focus(pid, until)
    def native_rect(rect, client, scale):
        x, y = client["at"]
        l, t, r, b = rect
        box = [math.ceil((x + l) * scale), math.ceil((y + t) * scale), math.floor((x + r) * scale), math.floor((y + b) * scale)]
        assert 0 <= box[0] < box[2] <= 1080 and 0 <= box[1] < box[3] <= 2424
        return box
    for attempt in range(1, 9):
        assert now() < until, "original visual-readiness cutoff"
        before = qualify(pid, until)
        assert before is not None
        frame = display.capture_native(("second-disable" if configured else "first-enable") + "-ready-" + str(attempt), results, results / "source", binaries,
            lambda argv, label, **kwargs: run(argv, label, until=until, **kwargs), env,
            {"qualified": True, "scope": "same PID/exe/UID/GID/class/title/output/profile normal window", "same_normal_window": before})
        after = qualify(pid, until)
        assert after is not None and before["actual_compositor_client"]["address"] == after["actual_compositor_client"]["address"]
        client, monitor = after["actual_compositor_client"], after["actual_output"]
        assert monitor["scale"] == 2.625 and frame["actual_native_capture"]["output"] == monitor["name"]
        row = {"attempt": attempt, "capture": frame, "before": before, "after": after, "labels": {},
            "state_text_native_ROI": native_rect(STATE, client, monitor["scale"]),
            "expected_state_text_Root_review_candidate": "Local connection: Enabled" if configured else "Local connection: Disabled"}
        record["frames"].append(row)
        with Image.open(results / frame["file"]) as image:
            assert image.size == (1080, 2424)
            rgb = image.convert("RGB")
            for label, rect in LABELS.items():
                box = native_rect(rect, client, monitor["scale"])
                expected = "foreground" if (label == "Disable") == configured else "muted"
                counts = {name: sum(rgb.getpixel((x, y)) == color for y in range(box[1], box[3]) for x in range(box[0], box[2])) for name, color in COLORS.items()}
                opposite = "muted" if expected == "foreground" else "foreground"
                row["labels"][label] = {"logical_ROI": rect, "native_ROI": box, "expected_color": expected,
                    "actual_palette_counts": counts, "matched": counts[expected] >= 10 and counts[opposite] == 0}
        row["observed_at_ns"] = now()
        row["matched"] = all(value["matched"] for value in row["labels"].values())
        assert now() < until, "capture/qualification completed after original readiness cutoff"
        if row["matched"]:
            record.update(qualified=True, matched_frame=attempt, completed_at_ns=now())
            return
    raise AssertionError("normal control label-state not ready in max8 frames/original1s")
