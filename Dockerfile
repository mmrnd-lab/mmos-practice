# Copyright 2026 Sungjae Park
# SPDX-License-Identifier: Apache-2.0

FROM ubuntu:24.04

RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y \
    build-essential \
    curl \
    git \
    qemu-system-misc \
    u-boot-tools \
    device-tree-compiler \
    pkg-config \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

ENV RUSTUP_HOME=/usr/local/rustup
ENV CARGO_HOME=/usr/local/cargo
ENV PATH="${CARGO_HOME}/bin:${PATH}"

RUN mkdir -p "${RUSTUP_HOME}" "${CARGO_HOME}" \
    && chmod 755 /usr/local "${RUSTUP_HOME}" "${CARGO_HOME}"

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal
RUN rustup target add riscv64gc-unknown-none-elf

WORKDIR /workspace/mmos-practice
