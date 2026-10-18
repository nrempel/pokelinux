"""Exact three signed additions using the qualified arm_tools local-DB procedure."""
from pathlib import Path
import hashlib, json, os, shutil, stat
def pin(path):
    path = Path(path)
    with path.open("rb") as stream:
        return {"bytes": path.stat().st_size, "sha256": hashlib.file_digest(stream, "sha256").hexdigest()}
def pairs(body):
    rows = [line.split(" ", 1) for line in body.splitlines()]
    value = dict(rows)
    assert len(value) == len(rows)
    return value
def prepare(run, I, S):
    payload = I / "source/payload"
    before_text = run(["/usr/bin/pacman", "-Q"], "package393-before", limit=131072)
    before = pairs(before_text)
    assert len(before) == 393 and before_text == (payload / "expected-installed393.txt").read_text()
    selected = json.loads((payload / "package-selection.json").read_text())
    assert selected["package_count"] == 3 and not selected["existing_package_replacements"]
    assert set(selected["seeds"]) == {"hostapd", "iw", "wireless-regdb"}
    paths = []
    for row in selected["packages"]:
        assert row["name"] not in before and row["architecture"] in ["aarch64", "any"]
        archive = payload / "packages" / row["filename"]
        signature = Path(str(archive) + ".sig")
        for path in [archive, signature]:
            assert stat.S_ISREG(path.lstat().st_mode)
        assert pin(archive) == {"bytes": row["compressed_bytes"], "sha256": row["sha256"]}
        assert pin(signature) == {"bytes": row["signature_bytes"], "sha256": row["signature_sha256"]}
        paths.append(archive)
    external = sorted(selected["base_external_dependencies"])
    assert run(["/usr/bin/pacman", "-T", *external], "actual393-external-dependencies", limit=131072) == ""
    plan = I / "plan-db"
    plan.mkdir()
    shutil.copytree("/var/lib/pacman/local", plan / "local")
    (plan / "sync").mkdir()
    for name, row in selected["sync_dbs"].items():
        source = payload / "db/sync" / name
        assert pin(source) == row
        shutil.copyfile(source, plan / "sync" / name)
    assert Path("/etc/pacman.d/gnupg/pubring.gpg").is_file()
    common = "[options]\nArchitecture = aarch64\nGPGDir = /etc/pacman.d/gnupg\nCacheDir = " + str(payload / "packages") + "\nSigLevel = Required DatabaseOptional TrustedOnly\nLocalFileSigLevel = Required TrustedOnly\nRemoteFileSigLevel = Required TrustedOnly\n"
    config = I / "plan.conf"
    config.write_text(common + "DBPath = " + str(plan) + "\nLogFile = " + str(I / "plan-pacman.log") + "\n" + "".join(
        "\n[" + name + "]\nServer = file://" + str(payload / "packages") + "\n" for name in ["core", "extra", "alarm"]))
    body = run(["/usr/bin/pacman", "--config", str(config), "-Sp", "--noconfirm", "--needed",
                "--print-format", "POKELINUX_PACKAGE_ROW\t%n\t%v\t%f", *selected["seeds"]],
               "actual393-additive-projection", seconds=30, limit=131072)
    projected = [tuple(line.split("\t")[1:]) for line in body.splitlines() if line.startswith("POKELINUX_PACKAGE_ROW\t")]
    expected = {(v["name"], v["version"], v["filename"]) for v in selected["packages"]}
    assert len(projected) == len(set(projected)) == 3 and set(projected) == expected
    config = I / "install.conf"
    config.write_text(common + "DBPath = /var/lib/pacman\nLogFile = " + str(I / "install-pacman.log") + "\n")
    run(["/usr/bin/pacman", "--config", str(config), "--noconfirm", "-U", *map(str, paths)],
        "strict-signed-offline3-install", seconds=120, limit=2097152)
    after = pairs(run(["/usr/bin/pacman", "-Q"], "package396-after-install", limit=131072))
    assert len(after) == 396 and all(after.get(k) == v for k, v in before.items())
    additions = {k: v for k, v in after.items() if k not in before}
    assert additions == {v["name"]: v["version"] for v in selected["packages"]}
    return {"before": before, "after": after, "additions": additions, "exact3_additions": True,
            "original393_versions_unchanged": True, "actual_TrustedOnly_offline_install": True}
def verify_after(run, I, S, receipt):
    final = pairs(run(["/usr/bin/pacman", "-Q"], "package396-final", limit=131072))
    assert final == receipt["after"] and all(final.get(k) == v for k, v in receipt["before"].items())
