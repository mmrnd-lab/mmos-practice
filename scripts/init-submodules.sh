#!/usr/bin/env bash
# Copyright 2026 Sungjae Park
# SPDX-License-Identifier: Apache-2.0

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

clone_or_update_repo() {
  local path="$1"
  local url="$2"

  if git -C "${ROOT_DIR}" ls-files --stage -- "${path}" | grep -q "^160000 "; then
    git -C "${ROOT_DIR}" submodule update --init --recursive "${path}"
    return
  fi

  if [ -d "${ROOT_DIR}/${path}/.git" ]; then
    git -C "${ROOT_DIR}/${path}" pull --ff-only
    return
  fi

  mkdir -p "${ROOT_DIR}/$(dirname "${path}")"
  git clone --depth 1 "${url}" "${ROOT_DIR}/${path}"
}

clone_or_update_repo external/opensbi https://github.com/riscv-software-src/opensbi.git
clone_or_update_repo external/u-boot https://source.denx.de/u-boot/u-boot.git
