#!/usr/bin/env bash
set -euo pipefail
if [[ $(findmnt -n -o FSTYPE --mountpoint /) != overlay ]]; then
    echo 'Refusing to install the HULK runtime outside OverlayFS.' >&2
    exit 1
fi
bundle=$(realpath "$1")
root=$(python3 - "$bundle/manifest.json" <<'PY'
import json
import pathlib
import platform
import re
import sys

manifest = json.loads(pathlib.Path(sys.argv[1]).read_text())
if platform.machine() != "aarch64" or manifest["system"] != "aarch64-linux":
    sys.exit("Runtime requires an ARM64 Linux robot")
release = pathlib.Path("/etc/nv_tegra_release").read_text()
if manifest["l4t"] != "36.4.3" or not re.search(r"# R36 .*REVISION: 4\.3,", release):
    sys.exit("Runtime requires L4T 36.4.3")
root = manifest["root"]
if not re.fullmatch(r"/nix/store/[0-9a-z]{32}-hulk-runtime", root):
    sys.exit("Invalid runtime store path")
print(root)
PY
)

(cd "$bundle" && echo 'b4e565fe4db5c352547b8146488cab81db06be89301a4c67b081c76f1e457760  nix-installer.tar.xz' | sha256sum --check)

nix_bin=/nix/var/nix/profiles/default/bin
if [[ ! -x $nix_bin/nix ]]; then
    temporary=$(mktemp -d)
    trap 'rm -rf "$temporary"' EXIT
    tar -xJf "$bundle/nix-installer.tar.xz" -C "$temporary"
    "$temporary/nix-2.34.8-aarch64-linux/install" --daemon --yes --no-channel-add
fi
# Trust this supplied local bundle, not unsigned substitutes in general.
export NIX_CONFIG=$'experimental-features = nix-command\nsubstituters =\nbuilders =\n'
"$nix_bin/nix" copy --no-check-sigs --from "file://$bundle/cache" "$root"
python3 - "$bundle/manifest.json" "$root/share/hulk-runtime/runtime.json" <<'PY'
import json
import pathlib
import sys

manifest, runtime = (json.loads(pathlib.Path(p).read_text()) for p in sys.argv[1:])
if runtime != {key: manifest[key] for key in ("runtime_version", "system", "l4t")}:
    sys.exit("Runtime metadata does not match bundle manifest")
PY
rootfs_overlay="$root/share/hulk-runtime/rootfs_overlay"
test -d "$rootfs_overlay/etc"
test -d "$rootfs_overlay/usr"

for unit in hulk.service hulk-runtime.service zenoh-bridge-dds.service zenohd.service jetson-clocks-refresh.timer; do
    if systemctl cat "$unit" >/dev/null 2>&1; then
        systemctl stop "$unit"
    fi
done

rm -f /etc/containers/systemd/hulk-runtime.container /etc/containers/containers.conf.d/90-nix-runtime.conf
for link in /etc/systemd/system-generators/podman-system-generator /usr/local/bin/podman; do
    case $(readlink "$link" || true) in
        /nix/var/nix/profiles/hulks/*) rm "$link" ;;
    esac
done
"$nix_bin/nix-env" --profile /nix/var/nix/profiles/hulks --set "$root"
getent group zenohd >/dev/null || groupadd --system zenohd
id zenohd >/dev/null 2>&1 || useradd --system --gid zenohd --home-dir /var/lib/zenoh --shell /usr/sbin/nologin zenohd
install -d -o zenohd -g zenohd /var/lib/zenoh
# Copy only bundled files; don't change existing host directories or mirror deletions.
(
    cd "$rootfs_overlay"
    find . \( -type f -o -type l \) -print0 |
        rsync -lp --ignore-times --relative --no-implied-dirs --chmod=Fu+w \
            --from0 --files-from=- ./ /
)
