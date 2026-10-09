#!/usr/bin/env bash
set -euo pipefail
umask 022
stage=/var/lib/hulks-overlay
handoff=/run/hulks-overlay-handoff

test "$(findmnt -n -o FSTYPE --mountpoint /)" = ext4
modprobe overlay
if [[ -f $stage/clean ]]; then
    for directory in upper work; do
        test ! -L "$stage/$directory"
        rm -rf --one-file-system "${stage:?}/${directory:?}"
    done
    rm "$stage/clean"
fi
mkdir -p "$stage/upper" "$stage/work" "$handoff"
mount -t tmpfs -o size=32m,mode=755 tmpfs "$handoff"
mkdir -p "$handoff"/{bin,dev,sys,proc,run,lower,mnt,scratch}
cp /bin/busybox "$handoff/bin/busybox"
for applet in sh mount umount mkdir sync sleep reboot chroot pivot_root cat rm ln chmod; do
    ln -s busybox "$handoff/bin/$applet"
done
cp "$stage/init" "$handoff/init"
mount --bind / "$handoff/lower"
mount -o remount,bind,ro "$handoff/lower"
mount --bind "$stage" "$handoff/scratch"
mount -t tmpfs -o mode=755 tmpfs "$handoff/run"
mkdir "$handoff/run/hulks-overlay"
cat /proc/sys/kernel/random/boot_id > "$handoff/run/hulks-overlay/boot-id"
printf '/dev/disk/by-uuid/%s\n' "$(findmnt -n -o UUID --mountpoint /)" > "$handoff/run/hulks-overlay/root-device"
sync
exec systemctl --no-block switch-root "$handoff" /init
