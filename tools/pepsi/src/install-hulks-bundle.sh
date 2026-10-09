#!/usr/bin/env bash
set -euo pipefail
bundle=$(realpath "$1")

nix_bin=/nix/var/nix/profiles/default/bin
if [[ ! -x $nix_bin/nix-store ]]; then
    temporary=$(mktemp -d)
    trap 'rm -rf "$temporary"' EXIT
    tar -xJf "$bundle/nix-installer.tar.xz" -C "$temporary"
    "$temporary/nix-2.34.8-aarch64-linux/install" --daemon --yes --no-channel-add
fi
# Import operates on supplied bytes, without evaluation, downloads, or builds.
export NIX_CONFIG=$'substituters =\nbuilders =\n'
"$nix_bin/nix-store" --import < "$bundle/nix.export"
root=$(cat "$bundle/root-path")

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
