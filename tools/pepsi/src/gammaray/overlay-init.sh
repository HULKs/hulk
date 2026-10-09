#!/bin/busybox sh
export PATH=/bin
umask 022
exec </dev/null >/scratch/boot.log 2>&1
fail() {
    echo "HULK overlay activation failed: $*"
    sync
    reboot -f
    while :; do sleep 60; done
}
kill -KILL -1
sleep 2
sync
mount -o remount,bind,ro /lower || fail lower
mkdir /newroot || fail newroot
mount -t overlay overlay -o lowerdir=/lower,upperdir=/scratch/upper,workdir=/scratch/work /newroot || fail overlay
mount -t tmpfs -o ro,size=4k,mode=000 tmpfs /newroot/var/lib/hulks-overlay || fail hide-backing
# Do not let normal boot mounts bypass the overlay.
printf '# Root managed by HULK overlay\n' > /newroot/etc/fstab || fail fstab
# The merged root always enters normal boot, never this handoff target.
target=$(cat /scratch/default-target) || fail default-target
rm -f /newroot/etc/systemd/system/default.target || fail default-target
if test -e "/newroot/etc/systemd/system/$target"; then
    ln -s "$target" /newroot/etc/systemd/system/default.target || fail default-target
else
    ln -s "/lib/systemd/system/$target" /newroot/etc/systemd/system/default.target || fail default-target
fi
mkdir -p /run/hulks-overlay-old-root /newroot/dev /newroot/sys /newroot/proc /newroot/run || fail directories
mount --move /dev /newroot/dev || fail dev
mount --move /sys /newroot/sys || fail sys
mount --move /proc /newroot/proc || fail proc
mount --move /run /newroot/run || fail run
cd /newroot || fail cd
pivot_root . run/hulks-overlay-old-root || fail pivot
exec chroot . /bin/sh -c 'cd /; umount -l /run/hulks-overlay-old-root || { /sbin/reboot -f; exit 1; }; exec /sbin/init'
