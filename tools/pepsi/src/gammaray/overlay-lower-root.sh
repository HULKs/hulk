#!/usr/bin/env bash
set -euo pipefail
umask 022
root=/
# Run in a private mount namespace so the temporary lower mount stays private.
mount --make-rprivate /
if [[ $(findmnt -n -o FSTYPE --mountpoint /) == overlay ]]; then
    device=$(cat /run/hulks-overlay/root-device)
    root=$(mktemp -d /run/hulks-overlay-lower.XXXXXX)
    trap 'umount "$root"; rmdir "$root"' EXIT
    mount -t ext4 "$device" "$root"
fi
