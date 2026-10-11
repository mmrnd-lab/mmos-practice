#!/usr/bin/env bash
# Copyright 2026 Sungjae Park
# SPDX-License-Identifier: Apache-2.0

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SKIP_INSTALL=0

if [[ "${1:-}" == "--skip-install" ]]; then
  SKIP_INSTALL=1
elif [[ $# -gt 0 ]]; then
  echo "usage: $0 [--skip-install]" >&2
  exit 2
fi

if ! grep -qi microsoft /proc/sys/kernel/osrelease; then
  echo "error: this helper is intended for WSL2" >&2
  exit 1
fi

if [[ "$(stat -fc %T "${ROOT_DIR}")" == "9p" ]]; then
  echo "warning: the repository is on a Windows-mounted filesystem"
  echo "warning: move it under the WSL Linux home directory for faster builds and reliable permissions"
fi

if [[ ${SKIP_INSTALL} -eq 0 ]]; then
  sudo apt-get update
  sudo env DEBIAN_FRONTEND=noninteractive apt-get install -y \
    podman uidmap slirp4netns fuse-overlayfs
fi

for command_name in podman newuidmap newgidmap; do
  if ! command -v "${command_name}" >/dev/null 2>&1; then
    echo "error: missing ${command_name}" >&2
    exit 1
  fi
done

if [[ ! -s /etc/subuid || ! -s /etc/subgid ]]; then
  echo "warning: subordinate UID/GID mappings are empty; rootless Podman may use a reduced mapping"
fi

# Artifacts created earlier by a rootful Docker container can remain owned by
# host root and block the keep-id user. Limit repair to generated directories.
GENERATED_DIRS=(
  "${ROOT_DIR}/target"
  "${ROOT_DIR}/build"
  "${ROOT_DIR}/user/crt/target"
  "${ROOT_DIR}/user/libc/target"
  "${ROOT_DIR}/user/init/target"
  "${ROOT_DIR}/user/shell/target"
  "${ROOT_DIR}/user/stdin-probe/target"
)
for generated_dir in "${GENERATED_DIRS[@]}"; do
  if [[ -d "${generated_dir}" ]] && find "${generated_dir}" ! -user "$(id -u)" -print -quit | grep -q .; then
    echo "repairing generated artifact ownership: ${generated_dir}"
    sudo chown -R "$(id -u):$(id -g)" "${generated_dir}"
  fi
done

podman info >/dev/null
# Podman 5 uses pasta by default for rootless networking. WSL2 can expose a
# large mirrored route table (VPNs and Windows host routes in particular),
# which makes pasta fail while duplicating routes. slirp4netns avoids copying
# that host route table and is sufficient for image pulls/builds and MMOS QEMU.
PODMAN_NETWORK="${PODMAN_NETWORK:-slirp4netns}"
podman run --rm --network "${PODMAN_NETWORK}" docker.io/library/alpine:latest true
podman build --network "${PODMAN_NETWORK}" -t mmos-practice:latest "${ROOT_DIR}"

echo "WSL2 Podman environment is ready."
echo "Next: cd ${ROOT_DIR} && ./docker.run"
