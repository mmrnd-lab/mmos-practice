#!/usr/bin/env bash
# Copyright 2026 Sungjae Park
# SPDX-License-Identifier: Apache-2.0

set -euo pipefail

IMAGE="${1:-build/drive0.img}"
UIMAGE="${2:-build/mmos.uImage}"
EXTLINUX_CONF="${3:-boot/extlinux.conf}"
ARG4="${4:-}"
ARG5="${5:-}"
ARG6="${6:-}"

USER_ELF=""
SHARED_OBJECT=""
SIZE_MB="64"
if [[ -n "${ARG4}" ]]; then
  if [[ "${ARG4}" =~ ^[0-9]+$ ]]; then
    SIZE_MB="${ARG4}"
  else
    USER_ELF="${ARG4}"
    if [[ -n "${ARG5}" ]]; then
      if [[ "${ARG5}" =~ ^[0-9]+$ ]]; then
        SIZE_MB="${ARG5}"
      else
        SHARED_OBJECT="${ARG5}"
        if [[ -n "${ARG6}" ]]; then
          SIZE_MB="${ARG6}"
        fi
      fi
    fi
  fi
fi

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
if [ -n "${USER_ELF}" ]; then
  if [ ! -f "${USER_ELF}" ]; then
    echo "missing user ELF: ${USER_ELF}" >&2
    exit 1
  fi
  mkdir -p "${TMPDIR}/sbin"
  cp "${USER_ELF}" "${TMPDIR}/sbin/init"
fi

if [ -n "${SHARED_OBJECT}" ]; then
  if [ ! -f "${SHARED_OBJECT}" ]; then
    echo "missing shared object: ${SHARED_OBJECT}" >&2
    exit 1
  fi
  mkdir -p "${TMPDIR}/lib"
  cp "${SHARED_OBJECT}" "${TMPDIR}/lib/libmmos_user_crt.so"
fi

rm -f "${IMAGE}"
dd if=/dev/zero of="${IMAGE}" bs=1M count="${SIZE_MB}" status=none

# Create a simple MBR with one Linux partition starting at sector 2048 (1 MiB).
printf 'label: dos\nstart=2048, type=83, bootable\n' | sfdisk "${IMAGE}" >/dev/null

OFFSET_BYTES=$((2048 * 512))
FS_SIZE_KB=$(((SIZE_MB - 1) * 1024))
mkfs.ext4 -F -b 1024 -d "${TMPDIR}" -O ^extent,^metadata_csum,^64bit -E offset="${OFFSET_BYTES}" "${IMAGE}" "${FS_SIZE_KB}" >/dev/null

if [ -n "${USER_ELF}" ] && [ -n "${SHARED_OBJECT}" ]; then
  echo "created bootable ext4 image with /sbin/init and /lib/libmmos_user_crt.so: ${IMAGE}"
elif [ -n "${USER_ELF}" ]; then
  echo "created bootable ext4 image with /sbin/init: ${IMAGE}"
else
  echo "created bootable ext4 image: ${IMAGE}"
fi
