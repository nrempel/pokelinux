#!/bin/sh
# Run in the simulation container. Creates only sim/rootfs and sim/rootfs.img.
set -eu
cd "$(dirname "$0")/.."
test ! -e sim/rootfs.img || { echo 'Existing VM image: use it or move it aside explicitly.' >&2; exit 1; }
archive=sim/cache/ArchLinuxARM-aarch64-latest.tar.gz
echo '42a4eeaa038994ffd31fa173256ef2f0ef511358eeb41b9ea1f8626391b9b319  sim/cache/ArchLinuxARM-aarch64-latest.tar.gz' | sha256sum -c -
mkdir -p sim/gnupg sim/rootfs sim/boot sim/results
chmod 700 sim/gnupg
gpg --homedir sim/gnupg --import sim/cache/archlinuxarm.asc
gpg --homedir sim/gnupg --status-fd 1 --verify "$archive.sig" "$archive" > sim/cache/signature.status
grep -q '^\[GNUPG:\] VALIDSIG 68B3537F39A313B3E574D06777193F152BDBE6A6 ' sim/cache/signature.status
if [ ! -f sim/rootfs/etc/arch-release ]; then bsdtar -xpf "$archive" -C sim/rootfs; fi
cp sim/rootfs/boot/Image sim/rootfs/boot/initramfs-linux.img sim/boot/
chmod 644 sim/boot/*
mkdir -p sim/rootfs/usr/local/bin sim/rootfs/etc/systemd/system/multi-user.target.wants
cat > sim/rootfs/usr/local/bin/pixel-sim-boot <<'BOOT'
#!/bin/bash
set -euo pipefail
mkdir -p /workspace /results
mount -t 9p -o trans=virtio,version=9p2000.L,ro project /workspace
mount -t 9p -o trans=virtio,version=9p2000.L results /results
exec bash /workspace/sim/guest/boot.sh
BOOT
chmod 755 sim/rootfs/usr/local/bin/pixel-sim-boot
cat > sim/rootfs/etc/systemd/system/pixel-sim.service <<'UNIT'
[Unit]
Description=Pixel graphics offline simulation
After=systemd-udev-settle.service network-online.target
Wants=systemd-udev-settle.service network-online.target
[Service]
Type=oneshot
ExecStart=/usr/local/bin/pixel-sim-boot
StandardOutput=journal+console
StandardError=journal+console
TimeoutStartSec=30min
ExecStopPost=/usr/bin/systemctl poweroff
[Install]
WantedBy=multi-user.target
UNIT
ln -sf ../pixel-sim.service sim/rootfs/etc/systemd/system/multi-user.target.wants/pixel-sim.service
cat > sim/rootfs/etc/systemd/network/20-wired.network <<'NETWORK'
[Match]
Name=en* eth*
[Network]
DHCP=yes
NETWORK
printf 'Server = https://de3.mirror.archlinuxarm.org/$arch/$repo\n' > sim/rootfs/etc/pacman.d/mirrorlist
python3 - <<'PY'
from pathlib import Path
p = Path('sim/rootfs/etc/shadow')
lines = []
for line in p.read_text().splitlines():
    fields = line.split(':')
    if fields[0] in ('root', 'alarm'):
        fields[1] = '!'
    lines.append(':'.join(fields))
p.write_text('\n'.join(lines) + '\n')
PY
truncate -s 16G sim/rootfs.img
mkfs.ext4 -F -L pixel-sim -d sim/rootfs sim/rootfs.img
chown "${SIM_UID:-1000}:${SIM_GID:-1000}" sim/rootfs.img sim/results
chmod 600 sim/rootfs.img
