#!/bin/sh
set -eu
root=$(findmnt --task 1 --noheadings --output FSTYPE --mountpoint /)
boot=$(cat /proc/sys/kernel/random/boot_id)
case "$root" in
    ext4) printf 'normal %s\n' "$boot" ;;
    overlay)
        test "$(cat /run/hulks-overlay/boot-id)" = "$boot"
        options=$(findmnt --task 1 --noheadings --output OPTIONS --mountpoint /)
        for option in lowerdir=/lower upperdir=/scratch/upper workdir=/scratch/work; do
            case ",$options," in
                *",$option,"*) ;;
                *) echo 'Unrecognized root overlay' >&2; exit 1 ;;
            esac
        done
        printf 'overlay %s\n' "$boot"
        ;;
    *) echo "Unsupported root filesystem: $root" >&2; exit 1 ;;
esac
