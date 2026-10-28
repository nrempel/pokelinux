"""Same-PID staging into guest RAM; original fixture begins only after all pins match."""
from pathlib import Path
import hashlib, json, os, time
T = time.clock_gettime_ns(time.CLOCK_BOOTTIME)
R = Path("/results")
S = json.loads((R / "selection.json").read_text())
W = T + S["guest_work_seconds"] * 1_000_000_000
I = Path("/run/hwsim-original-viability")
assert not I.exists()
I.mkdir(mode=0o755)
for relative, row in S["direct_inputs"].items():
    assert time.clock_gettime_ns(time.CLOCK_BOOTTIME) < W
    path = Path(relative)
    assert not path.is_absolute() and ".." not in path.parts
    source, target = Path("/workspace/current") / path, I / path
    target.parent.mkdir(parents=True, exist_ok=True)
    with source.open("rb") as src, target.open("xb") as dst:
        h = hashlib.sha256()
        count = 0
        while block := src.read(1048576):
            assert time.clock_gettime_ns(time.CLOCK_BOOTTIME) < W
            dst.write(block)
            h.update(block)
            count += len(block)
    assert count == row["bytes"] and h.hexdigest() == row["sha256"]
    target.chmod(row["mode"])
    os.chown(target, 0, 0)
env = dict(os.environ, POKELINUX_HWSIM_T0_NS=str(T))
os.execve("/usr/bin/python3", ["/usr/bin/python3", "-B", str(I / "source/guest-main.py")], env)
