#!/usr/bin/env bash
set -Eeuo pipefail
trap 'printf "Lower-root command failed (exit %s, line %s): %s\n" "$?" "$LINENO" "$BASH_COMMAND" >&2' ERR
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
