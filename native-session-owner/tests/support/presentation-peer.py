"""Isolated UID1000 socket fixture; no compositor or permission authority claim."""
import json, os, socket, sys
from pathlib import Path
root = Path("/run/user/1000")
sockets = []
def bind(path):
    path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    sock.bind(str(path)); sock.listen(32); sockets.append(sock)
for kind in sys.argv[1:]:
    bind(root / ("wayland-1" if kind == "wayland" else "hypr/fixture_compositor_1/.socket.sock"))
print("READY", flush=True)
for line in sys.stdin:
    request = json.loads(line)
    op = request["op"]
    target = root / "pokelinux-presentation.env"
    if op == "write":
        target.write_text(request["body"]); target.chmod(request.get("mode", 0o600))
    elif op == "remove":
        target.unlink(missing_ok=True)
    elif op == "read":
        print(json.dumps(target.read_text() if target.exists() else None), flush=True); continue
    elif op == "symlink":
        (root / "sentinel").write_text("keep"); target.symlink_to(root / "sentinel")
    elif op == "hardlink":
        os.link(target, root / "sentinel")
    elif op == "sentinel":
        print(json.dumps((root / "sentinel").read_text()), flush=True); continue
    elif op == "quit":
        break
    else:
        raise RuntimeError(op)
    print("OK", flush=True)
