#!/bin/bash
# Execute in the disposable QEMU container with /vm bound to sim/local-vm.
set -euo pipefail
test ! -e /vm/rootfs.img
mkdir /tmp/guest
bsdtar -xpf /vm/rootfs.tar -C /tmp/guest
root=/tmp/guest
# Docker's marker belongs to the export container, not the booted guest.
rm -f "$root/.dockerenv"
cp "$root/boot/Image" "$root/boot/initramfs-linux.img" /vm/
chmod 644 /vm/Image /vm/initramfs-linux.img
mkdir -p "$root/etc/systemd/system/multi-user.target.wants"
cat > "$root/etc/systemd/system/native-graphics.service" <<'UNIT'
[Unit]
Description=Disposable native graphics validation
After=systemd-udev-settle.service
Wants=systemd-udev-settle.service
[Service]
Type=oneshot
ExecStart=/usr/local/bin/native-graphics-boot
ExecStopPost=/usr/bin/systemctl poweroff
TimeoutStartSec=15min
StandardOutput=journal+console
StandardError=journal+console
[Install]
WantedBy=multi-user.target
UNIT
ln -s ../native-graphics.service "$root/etc/systemd/system/multi-user.target.wants/native-graphics.service"
mkdir -p "$root/usr/local/bin"
cat > "$root/usr/local/bin/native-graphics-boot" <<'BOOT'
#!/bin/bash
set -euo pipefail
mkdir -p /workspace /results
mount -t 9p -o trans=virtio,version=9p2000.L,ro project /workspace
mount -t 9p -o trans=virtio,version=9p2000.L results /results
exec python /workspace/sim/guest/native_graphics.py
BOOT
chmod 755 "$root/usr/local/bin/native-graphics-boot"
# No password or network access is required for this test guest.
sed -i -E 's/^(root|alarm):[^:]*:/\1:!:/' "$root/etc/shadow"
truncate -s 12G /vm/rootfs.img
mkfs.ext4 -F -L native-graphics -d "$root" /vm/rootfs.img
chmod 600 /vm/rootfs.img
