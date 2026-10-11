#!/usr/bin/env bash
# Copyright 2026 Sungjae Park
# SPDX-License-Identifier: Apache-2.0

set -euo pipefail

git submodule update --init --recursive external/opensbi external/u-boot
