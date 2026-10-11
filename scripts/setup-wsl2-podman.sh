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

podman info >/dev/null
podman run --rm docker.io/library/alpine:latest true
podman build -t mmos-practice:latest "${ROOT_DIR}"

echo "WSL2 Podman environment is ready."
echo "Next: cd ${ROOT_DIR} && ./docker.run"
