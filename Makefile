# Copyright 2026 Sungjae Park
# SPDX-License-Identifier: Apache-2.0

PROFILE ?= release
QEMU ?= qemu-system-riscv64
RUN_SECONDS ?= 5
TEST_TARGET ?= x86_64-unknown-linux-gnu
KERNEL_ELF := target/riscv64gc-unknown-none-elf/$(PROFILE)/mmos-practice

.PHONY: require-container image shell test mmos.bin run-nographic run-monitor

require-container:
	@if [ "$${ALLOW_HOST_BUILD:-0}" = "1" ]; then \
		exit 0; \
	fi
	@if [ ! -f /.dockerenv ] && [ -z "$${container:-}" ]; then \
		echo "error: run this target inside container (set ALLOW_HOST_BUILD=1 to bypass)"; \
		exit 1; \
	fi

image:
	podman build -t mmos-practice:latest .

shell:
	./docker.run

test: require-container
	cargo test --lib --target $(TEST_TARGET)

mmos.bin: require-container
	cargo build --profile $(PROFILE)

run-nographic: require-container
	@echo "running QEMU (nographic) with $(KERNEL_ELF)"
	@echo "tip: set RUN_SECONDS=<sec> to adjust runtime window"
	@timeout --foreground $(RUN_SECONDS)s $(QEMU) \
		-machine virt \
		-nographic \
		-bios default \
		-kernel $(KERNEL_ELF) \
		-no-reboot \
		-smp 1 \
		-m 512M || test $$? -eq 124

run-monitor: require-container
	@echo "running QEMU with serial+monitor on stdio"
	@echo "tip: press Ctrl-t c to switch between serial console and QEMU monitor"
	@echo "tip: press Ctrl-t h to show QEMU hotkeys"
	@echo "tip: set RUN_SECONDS=<sec> to adjust runtime window"
	@timeout --foreground $(RUN_SECONDS)s $(QEMU) \
		-machine virt \
		-nographic \
		-echr 0x14 \
		-serial mon:stdio \
		-bios default \
		-kernel $(KERNEL_ELF) \
		-no-reboot \
		-smp 1 \
		-m 512M || test $$? -eq 124
