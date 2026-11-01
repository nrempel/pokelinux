"""TEST ONLY per-boot private event ring; no Source or permission authority."""
import time
START = time.clock_gettime_ns(time.CLOCK_BOOTTIME)
from pathlib import Path
import hashlib, json, mmap, os, stat, struct, sys

UNTIL = START + 5_000_000_000
FILE = Path('/run/user/1000/pixel10-profile/startup-events.bin')
HEADER = struct.Struct('<8sIIIIQQ3Q')
RECORD = struct.Struct('<QQII')
SIZE, CAPACITY = 4096, 128
now = lambda: time.clock_gettime_ns(time.CLOCK_BOOTTIME)
assert len(sys.argv) == 1 and sys.byteorder == 'little'
assert os.getresuid() == os.getresgid() == (1000, 1000, 1000)
assert now() < UNTIL and HEADER.size == 64 and RECORD.size == 24
# Same longest-enclosing-mount check as the existing cold consumer.
with Path('/proc/self/mountinfo').open('rb') as stream:
    mountinfo = stream.read(131073)
assert len(mountinfo) <= 131072
mounts = []
for line in mountinfo.decode().splitlines():
    left, right = line.split(' - ', 1)
    mount = left.split()[4]
    if str(FILE) == mount or str(FILE).startswith(mount.rstrip('/') + '/'):
        mounts.append((len(mount), right.split()[0]))
assert mounts and max(mounts)[1] == 'tmpfs'
runtime = FILE.parent.parent.lstat()
assert stat.S_ISDIR(runtime.st_mode) and runtime.st_uid == runtime.st_gid == 1000
assert stat.S_IMODE(runtime.st_mode) == 0o700
FILE.parent.mkdir(mode=0o700, exist_ok=True)
parent = FILE.parent.lstat()
assert stat.S_ISDIR(parent.st_mode) and parent.st_uid == parent.st_gid == 1000
assert stat.S_IMODE(parent.st_mode) == 0o700
header = HEADER.pack(b'PLSEVT01', 1, 64, 24, CAPACITY, 0, 0, 0, 0, 0)
data = header + bytes(SIZE - len(header))
assert now() < UNTIL
fd = os.open(FILE, os.O_CREAT | os.O_EXCL | os.O_RDWR | os.O_CLOEXEC | os.O_NOFOLLOW | os.O_NONBLOCK, 0o600)
try:
    os.fchmod(fd, 0o600)
    os.ftruncate(fd, SIZE)
    assert os.write(fd, data) == SIZE
    # Same cache-line touch performed by the selected fixture Mapping before
    # listener registration. Only the fresh zero-filled file is touched here.
    with mmap.mmap(fd, SIZE, flags=mmap.MAP_SHARED, prot=mmap.PROT_READ | mmap.PROT_WRITE) as memory:
        for offset in range(0, SIZE, 64):
            memory[offset] = memory[offset]
    info = os.fstat(fd)
    assert stat.S_ISREG(info.st_mode) and info.st_nlink == 1 and info.st_size == SIZE
    assert info.st_uid == info.st_gid == 1000 and stat.S_IMODE(info.st_mode) == 0o600
finally:
    os.close(fd)
assert now() < UNTIL
print(json.dumps({'schema':1,'event':'private_startup_event_ring_prepared','PID':os.getpid(),
    'boot_id':Path('/proc/sys/kernel/random/boot_id').read_text().strip(),
    'path':str(FILE),'device':info.st_dev,'inode':info.st_ino,'bytes':SIZE,
    'UID':info.st_uid,'GID':info.st_gid,'mode':oct(stat.S_IMODE(info.st_mode)),
    'initial_sha256':hashlib.sha256(data).hexdigest(),'actual_enclosing_filesystem':'tmpfs',
    'capacity':CAPACITY,'prefaulted_cache_line_step':64,'prepared_before_native_owner':True,
    'Source_or_focus_or_permission_fact_published':False}),flush=True)
