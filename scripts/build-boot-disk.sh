#!/usr/bin/env bash
# Copyright 2026 Sungjae Park
# SPDX-License-Identifier: Apache-2.0

set -euo pipefail

IMAGE="${1:-build/drive0.img}"
UIMAGE="${2:-build/mmos.uImage}"
EXTLINUX_CONF="${3:-boot/extlinux.conf}"
SIZE_MB="${4:-64}"

if [ ! -f "${UIMAGE}" ]; then
  echo "missing uImage: ${UIMAGE}" >&2
  exit 1
fi

if [ ! -f "${EXTLINUX_CONF}" ]; then
  echo "missing extlinux config: ${EXTLINUX_CONF}" >&2
  exit 1
fi

mkdir -p "$(dirname "${IMAGE}")"

TMPDIR="$(mktemp -d)"
trap 'rm -rf "${TMPDIR}"' EXIT

mkdir -p "${TMPDIR}/boot/extlinux" "${TMPDIR}/extlinux"
cp "${UIMAGE}" "${TMPDIR}/boot/mmos.uImage"
cp "${EXTLINUX_CONF}" "${TMPDIR}/boot/extlinux/extlinux.conf"
cp "${EXTLINUX_CONF}" "${TMPDIR}/extlinux/extlinux.conf"

rm -f "${IMAGE}"
dd if=/dev/zero of="${IMAGE}" bs=1M count="${SIZE_MB}" status=none

# Create a simple MBR with one Linux partition starting at sector 2048 (1 MiB).
printf 'label: dos\nstart=2048, type=83, bootable\n' | sfdisk "${IMAGE}" >/dev/null

OFFSET_BYTES=$((2048 * 512))
FS_SIZE_KB=$(((SIZE_MB - 1) * 1024))
mkfs.ext4 -F -d "${TMPDIR}" -E offset="${OFFSET_BYTES}" "${IMAGE}" "${FS_SIZE_KB}" >/dev/null

echo "created bootable ext4 image: ${IMAGE}"
