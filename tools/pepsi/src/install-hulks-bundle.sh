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

for service in hulk hulk-runtime zenoh-bridge-dds zenohd; do
    if systemctl cat "$service.service" >/dev/null 2>&1; then
        systemctl stop "$service.service"
    fi
done
"$nix_bin/nix-env" --profile /nix/var/nix/profiles/hulks --set "$root"
getent group zenohd >/dev/null || groupadd --system zenohd
id zenohd >/dev/null 2>&1 || useradd --system --gid zenohd --home-dir /var/lib/zenoh --shell /usr/sbin/nologin zenohd
install -d -o zenohd -g zenohd /var/lib/zenoh
mkdir -p /etc/zenohd /etc/zenoh-bridge-dds /etc/containers/systemd /etc/containers/containers.conf.d /etc/systemd/system-generators
ln -sfn /nix/var/nix/profiles/hulks/lib/systemd/system-generators/podman-system-generator \
    /etc/systemd/system-generators/podman-system-generator
cat > /etc/containers/containers.conf.d/90-nix-runtime.conf <<EOF
[containers]
init_path = "$root/bin/catatonit"
[engine]
runtime = "crun"
lock_type = "file"
conmon_path = ["$root/libexec/podman/conmon"]
[engine.runtimes]
crun = ["$root/libexec/podman/crun"]
EOF
if [[ ! -e /etc/containers/policy.json ]]; then
    echo '{"default":[{"type":"insecureAcceptAnything"}]}' > /etc/containers/policy.json
fi
# Existing containers can reference the old static runtime. Migrate explicitly;
# a failure must not be hidden or treated as successful provisioning.
# File locks avoid incompatible shared-memory mutex layouts between Podman builds.
"$root/bin/podman" system renumber
"$root/bin/podman" system migrate --new-runtime crun --migrate-db
# Other Pepsi commands invoke `sudo podman`; keep their PATH entry managed too.
mkdir -p /usr/local/bin
ln -sfn /nix/var/nix/profiles/hulks/bin/podman /usr/local/bin/podman
