#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail

target_version="5.8.1"
architecture="arm64"
base_url="https://github.com/mgoltzsche/podman-static/releases/download/v${target_version}"
archive_name="podman-linux-${architecture}.tar.gz"
install_prefix="/usr/local"

current_version=$(podman --version | awk '{print $3}')

if [[ "$current_version" == "$target_version" ]]; then
    exit 0
fi

if [ "$(id --user)" -ne 0 ]; then
  echo "This script must be run as root."
  exit 1
fi

apt update
apt remove --yes podman crun || true
apt install --yes iptables uidmap util-linux


temporary_directory="$(mktemp --directory)"
trap 'rm --recursive --force "${temporary_directory}"' EXIT

curl --fail --silent --show-error --location --output "${temporary_directory}/${archive_name}" "${base_url}/${archive_name}"

tar --extract --gzip --file "${temporary_directory}/${archive_name}" --directory "${temporary_directory}"
extracted_directory="${temporary_directory}/podman-linux-${architecture}"

cp --recursive "${extracted_directory}/usr/local/"* "${install_prefix}/"

if [ -d "${extracted_directory}/etc" ]; then
  # Preserve user modifications to existing configuration files
  cp --recursive --no-clobber "${extracted_directory}/etc/"* /etc/
fi

# Clear memory locks that cause permission errors after upgrades
rm --force /dev/shm/libpod_lock || true
rm --force /dev/shm/libpod_rootless_lock_* || true

# Prevent execution blocks due to custom installation path
apparmor_file="/etc/apparmor.d/podman"
if [ -f "${apparmor_file}" ]; then
  if grep --quiet "^profile podman /usr/bin/podman" "${apparmor_file}"; then
    sed --regexp-extended --in-place 's!^profile podman /usr/bin/podman !profile podman /usr/{bin,local/bin}/podman !' "${apparmor_file}"
    systemctl reload apparmor || true
  fi
fi

# Prevent duplicate namespace entries for rootless mode
primary_user="${SUDO_USER:-$(logname)}"
if [ -n "${primary_user}" ] && [ "${primary_user}" != "root" ]; then
  if ! grep --quiet "^${primary_user}:" /etc/subuid; then
    echo "${primary_user}:100000:200000" >> /etc/subuid
  fi
  if ! grep --quiet "^${primary_user}:" /etc/subgid; then
    echo "${primary_user}:100000:200000" >> /etc/subgid
  fi
fi

# Migrate existing database
if command -v podman >/dev/null 2>&1; then
  podman system migrate --migrate-db || true
fi

podman --version
