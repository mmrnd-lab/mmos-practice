#!/usr/bin/env bash
# Copyright 2026 Sungjae Park
# SPDX-License-Identifier: Apache-2.0

set -euo pipefail

IMAGE="${1:-drive0.img}"
SIZE_MB="${2:-64}"

if [ -f "${IMAGE}" ]; then
  echo "disk image already exists: ${IMAGE}"
  exit 0
fi

dd if=/dev/zero of="${IMAGE}" bs=1M count="${SIZE_MB}" status=none
mkfs.ext4 -F "${IMAGE}" >/dev/null

echo "created ${IMAGE} (${SIZE_MB} MiB)"
