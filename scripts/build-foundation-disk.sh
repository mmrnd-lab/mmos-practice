#!/usr/bin/env bash
# Copyright 2026 Sungjae Park
# SPDX-License-Identifier: Apache-2.0

set -euo pipefail

IMAGE="${1:?image path required}"
UIMAGE="${2:?uImage path required}"
EXTLINUX_CONF="${3:?extlinux config required}"
INIT_ELF="${4:?init ELF required}"
SHELL_ELF="${5:?shell ELF required}"
SHARED_OBJECT="${6:?libc shared object required}"
SIZE_MB="${7:-64}"

for input in "${UIMAGE}" "${EXTLINUX_CONF}" "${INIT_ELF}" "${SHELL_ELF}" "${SHARED_OBJECT}"; do
  if [[ ! -f "${input}" ]]; then
    echo "missing input: ${input}" >&2
    exit 1
  fi
done

mkdir -p "$(dirname "${IMAGE}")"
STAGE_DIR="$(mktemp -d)"
trap 'rm -rf "${STAGE_DIR}"' EXIT

mkdir -p "${STAGE_DIR}/boot/extlinux" "${STAGE_DIR}/extlinux" \
  "${STAGE_DIR}/sbin" "${STAGE_DIR}/bin" "${STAGE_DIR}/lib"
cp "${UIMAGE}" "${STAGE_DIR}/boot/mmos.uImage"
cp "${EXTLINUX_CONF}" "${STAGE_DIR}/boot/extlinux/extlinux.conf"
cp "${EXTLINUX_CONF}" "${STAGE_DIR}/extlinux/extlinux.conf"
cp "${INIT_ELF}" "${STAGE_DIR}/sbin/init"
cp "${SHELL_ELF}" "${STAGE_DIR}/bin/sh"
cp "${SHARED_OBJECT}" "${STAGE_DIR}/lib/libmmos_user_libc.so"

rm -f "${IMAGE}"
dd if=/dev/zero of="${IMAGE}" bs=1M count="${SIZE_MB}" status=none
printf 'label: dos\nstart=2048, type=83, bootable\n' | sfdisk "${IMAGE}" >/dev/null

OFFSET_BYTES=$((2048 * 512))
FS_SIZE_KB=$(((SIZE_MB - 1) * 1024))
mkfs.ext4 -F -b 1024 -d "${STAGE_DIR}" -O ^extent,^metadata_csum,^64bit \
  -E offset="${OFFSET_BYTES}" "${IMAGE}" "${FS_SIZE_KB}" >/dev/null

echo "created foundation image with /sbin/init, /bin/sh, and libc: ${IMAGE}"
