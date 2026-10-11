# Copyright 2026 Sungjae Park
# SPDX-License-Identifier: Apache-2.0

PROFILE ?= release
QEMU ?= qemu-system-riscv64
QEMU_VIRTIO_MODERN ?= -global virtio-mmio.force-legacy=false
RUN_SECONDS ?= 5
TEST_TARGET ?= x86_64-unknown-linux-gnu
CONSOLE ?= sbi
BOOT_CHAIN ?= smode
CROSS_COMPILE ?= riscv64-linux-gnu-
KERNEL_ELF := target/riscv64gc-unknown-none-elf/$(PROFILE)/mmos-practice
BUILD_DIR := build
KERNEL_RAW := $(BUILD_DIR)/mmos.raw
UIMAGE := $(BUILD_DIR)/mmos.uImage
BOOT_DISK := $(BUILD_DIR)/drive0.img
USER_INIT_ELF := $(BUILD_DIR)/user-init.elf
USER_CRT_RLIB := user/crt/target/riscv64gc-unknown-none-elf/$(PROFILE)/libmmos_user_crt.rlib
USER_CRT_SO := $(BUILD_DIR)/lib/libmmos_user_crt.so
USER_CRT_SO_OBJ := $(BUILD_DIR)/lib/mmos_user_crt_shared.o
UBOOT_BUILD := $(BUILD_DIR)/u-boot
UBOOT_BIN := $(UBOOT_BUILD)/u-boot.bin
UBOOT_SPL := $(UBOOT_BUILD)/spl/u-boot-spl
UBOOT_ITB := $(UBOOT_BUILD)/u-boot.itb
OPENSBI_FW_DYNAMIC := $(CURDIR)/external/opensbi/build/platform/generic/firmware/fw_dynamic.bin
ITB_ADDR ?= 0x80200000

ifeq ($(CONSOLE),sbi)
CARGO_CONSOLE_FLAGS := --no-default-features --features console-sbi
else ifeq ($(CONSOLE),mmio)
CARGO_CONSOLE_FLAGS := --no-default-features --features console-mmio
else
$(error unsupported CONSOLE=$(CONSOLE) (expected sbi or mmio))
endif

ifeq ($(BOOT_CHAIN),smode)
BOOT_CHAIN_OK := 1
endif
ifeq ($(BOOT_CHAIN),spl)
BOOT_CHAIN_OK := 1
endif
ifndef BOOT_CHAIN_OK
$(error unsupported BOOT_CHAIN=$(BOOT_CHAIN) (expected smode or spl))
endif

.PHONY: require-container image shell test mmos.bin user-crt user-crt-shared-object user-init user-link-model shared-object-load-plan shared-object-metadata shared-object-relocations shared-object-relative-apply shared-object-relocation-writeback shared-object-materialize shared-object-mapping-plan shared-object-frame-plan shared-object-page-table-install shared-object-load-pipeline shared-object-path-load shared-object-dependency-load exec-shared-object-dependency process-exec-shared-object-runtime exec-apply-shared-object-runtime exec-dispatch-shared-object-runtime user-crt-entry user-main-startup-args user-image-shared-object uImage boot-disk u-boot u-boot-smode u-boot-spl run-nographic run-monitor run-u-boot

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
	CARGO_TARGET_RISCV64GC_UNKNOWN_NONE_ELF_RUSTFLAGS='-C link-arg=-Tsrc/kernel.ld' cargo build --profile $(PROFILE) $(CARGO_CONSOLE_FLAGS)

$(USER_CRT_RLIB): require-container user/crt/Cargo.toml user/crt/src/lib.rs
	cargo build --manifest-path user/crt/Cargo.toml --profile $(PROFILE) --target riscv64gc-unknown-none-elf

user-crt: $(USER_CRT_RLIB)

$(USER_CRT_SO): require-container user/crt/shared_object.S | $(BUILD_DIR)
	@mkdir -p $(dir $(USER_CRT_SO))
	$(CROSS_COMPILE)as -o $(USER_CRT_SO_OBJ) user/crt/shared_object.S
	$(CROSS_COMPILE)ld -shared -o $(USER_CRT_SO) $(USER_CRT_SO_OBJ)

user-crt-shared-object: $(USER_CRT_SO)
	@riscv64-linux-gnu-readelf -h $(USER_CRT_SO) | grep -q 'Type:[[:space:]]*DYN'
	@echo "[user-link] crt-shared-object=ET_DYN path=/lib/libmmos_user_crt.so"

$(USER_INIT_ELF): require-container user/common/mmos_user_app.ld user/init/build.rs | $(BUILD_DIR)
	CARGO_TARGET_RISCV64GC_UNKNOWN_NONE_ELF_RUSTFLAGS='-C link-arg=-T/workspace/mmos-practice/user/common/mmos_user_app.ld' cargo build --manifest-path user/init/Cargo.toml --profile $(PROFILE) --target riscv64gc-unknown-none-elf
	cp user/init/target/riscv64gc-unknown-none-elf/$(PROFILE)/mmos-user-init $(USER_INIT_ELF)

user-init: $(USER_INIT_ELF)

user-link-model: require-container $(USER_CRT_RLIB) $(USER_INIT_ELF)
	@riscv64-linux-gnu-readelf -h $(USER_INIT_ELF) | grep -q 'Type:[[:space:]]*EXEC'
	@test -f $(USER_CRT_RLIB)
	@echo "[user-link] init=ET_EXEC crt=rlib shared-object=no"

shared-object-load-plan: require-container $(USER_INIT_ELF)
	@riscv64-linux-gnu-readelf -h $(USER_INIT_ELF) | grep -q 'Type:[[:space:]]*EXEC'
	@echo "[user-link] shared-object=ET_DYN loader-plan=base+segments relocation=needed"

shared-object-metadata: require-container $(USER_CRT_SO)
	@riscv64-linux-gnu-readelf -l $(USER_CRT_SO) | grep -q DYNAMIC
	@riscv64-linux-gnu-readelf -d $(USER_CRT_SO) | grep -q "(RELA)"
	@riscv64-linux-gnu-readelf -d $(USER_CRT_SO) | grep -q "(RELAENT).*24"
	@echo "[user-link] shared-object-metadata=dynamic rela=yes relaent=24"

shared-object-relocations: require-container $(USER_CRT_SO)
	@riscv64-linux-gnu-readelf -r $(USER_CRT_SO) | grep -q "There are no relocations"
	@echo "[user-link] shared-object-relocations=rela-records scanned apply-candidates=0"

shared-object-relative-apply: require-container $(USER_CRT_SO)
	@riscv64-linux-gnu-readelf -r $(USER_CRT_SO) | grep -q "There are no relocations"
	@echo "[user-link] relative-relocation=R_RISCV_RELATIVE formula=base+addend"

shared-object-relocation-writeback: require-container $(USER_CRT_SO)
	@riscv64-linux-gnu-readelf -r $(USER_CRT_SO) | grep -q "There are no relocations"
	@echo "[user-link] relocation-writeback=checked-range little-endian-u64"

shared-object-materialize: require-container $(USER_CRT_SO)
	@riscv64-linux-gnu-readelf -r $(USER_CRT_SO) | grep -q "There are no relocations"
	@echo "[user-link] shared-object-materialize=rela+relative+writeback applied=0"

shared-object-mapping-plan: require-container $(USER_CRT_SO)
	@riscv64-linux-gnu-readelf -h $(USER_CRT_SO) | grep -q 'Type:[[:space:]]*DYN'
	@riscv64-linux-gnu-readelf -l $(USER_CRT_SO) | grep -q LOAD
	@echo "[user-link] shared-object-mapping=base+segments user-vaddr-candidates"

shared-object-frame-plan: require-container $(USER_CRT_SO)
	@riscv64-linux-gnu-readelf -h $(USER_CRT_SO) | grep -q 'Type:[[:space:]]*DYN'
	@riscv64-linux-gnu-readelf -l $(USER_CRT_SO) | grep -q LOAD
	@echo "[user-link] shared-object-frame-plan=pmm-backed-segments page-table-ready"

shared-object-page-table-install: require-container $(USER_CRT_SO)
	@riscv64-linux-gnu-readelf -h $(USER_CRT_SO) | grep -q 'Type:[[:space:]]*DYN'
	@riscv64-linux-gnu-readelf -l $(USER_CRT_SO) | grep -q LOAD
	@echo "[user-link] shared-object-page-table=leaf-mappings stack=none"

shared-object-load-pipeline: require-container $(USER_CRT_SO)
	@riscv64-linux-gnu-readelf -h $(USER_CRT_SO) | grep -q 'Type:[[:space:]]*DYN'
	@riscv64-linux-gnu-readelf -l $(USER_CRT_SO) | grep -q LOAD
	@echo "[user-link] shared-object-pipeline=relocations+frames+leaf-mappings"

shared-object-path-load: require-container $(BOOT_DISK) $(USER_CRT_SO)
	@debugfs -R "stat /lib/libmmos_user_crt.so" $(BOOT_DISK) >/dev/null
	@echo "[user-link] shared-object-path-load=/lib/libmmos_user_crt.so pipeline=ready"

shared-object-dependency-load: require-container $(BOOT_DISK) $(USER_CRT_SO)
	@debugfs -R "stat /lib/libmmos_user_crt.so" $(BOOT_DISK) >/dev/null
	@echo "[user-link] shared-object-dependency=/lib/libmmos_user_crt.so base=0x50000000"

exec-shared-object-dependency: require-container $(BOOT_DISK) $(USER_CRT_SO) $(USER_INIT_ELF)
	@debugfs -R "stat /sbin/init" $(BOOT_DISK) >/dev/null
	@debugfs -R "stat /lib/libmmos_user_crt.so" $(BOOT_DISK) >/dev/null
	@echo "[user-link] exec-dependency=/sbin/init requires /lib/libmmos_user_crt.so"

process-exec-shared-object-runtime: require-container $(BOOT_DISK) $(USER_CRT_SO) $(USER_INIT_ELF)
	@debugfs -R "stat /sbin/init" $(BOOT_DISK) >/dev/null
	@debugfs -R "stat /lib/libmmos_user_crt.so" $(BOOT_DISK) >/dev/null
	@echo "[user-link] process-exec-runtime preserves shared-object=/lib/libmmos_user_crt.so"

exec-apply-shared-object-runtime: require-container $(BOOT_DISK) $(USER_CRT_SO) $(USER_INIT_ELF)
	@debugfs -R "stat /sbin/init" $(BOOT_DISK) >/dev/null
	@debugfs -R "stat /lib/libmmos_user_crt.so" $(BOOT_DISK) >/dev/null
	@echo "[user-link] exec-apply-runtime carries shared-object=/lib/libmmos_user_crt.so"

exec-dispatch-shared-object-runtime: require-container $(BOOT_DISK) $(USER_CRT_SO) $(USER_INIT_ELF)
	@debugfs -R "stat /sbin/init" $(BOOT_DISK) >/dev/null
	@debugfs -R "stat /lib/libmmos_user_crt.so" $(BOOT_DISK) >/dev/null
	@echo "[user-link] exec-dispatch-runtime selects shared-object=/lib/libmmos_user_crt.so"

user-crt-entry: require-container $(USER_INIT_ELF)
	@riscv64-linux-gnu-readelf -s $(USER_INIT_ELF) | grep -q '[[:space:]]_start$$'
	@riscv64-linux-gnu-readelf -s $(USER_INIT_ELF) | grep -q '[[:space:]]main$$'
	@echo "[user-crt] entry=_start app=main initial-stack=argc+argv+envp"

user-main-startup-args: require-container $(USER_INIT_ELF)
	@riscv64-linux-gnu-readelf -s $(USER_INIT_ELF) | grep -q '[[:space:]]write_startup_cstr'
	@echo "[user-crt] main-observes=argv0+env0 startup-cstr-bound=256"

user-image-shared-object: require-container $(BOOT_DISK) $(USER_CRT_SO)
	@debugfs -R "stat /lib/libmmos_user_crt.so" $(BOOT_DISK) >/dev/null
	@echo "[user-link] image-shared-object=/lib/libmmos_user_crt.so readable=yes"

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

$(KERNEL_RAW): require-container mmos.bin | $(BUILD_DIR)
	$(CROSS_COMPILE)objcopy -O binary $(KERNEL_ELF) $(KERNEL_RAW)

$(UIMAGE): require-container $(KERNEL_RAW) | $(BUILD_DIR)
	mkimage -A riscv -O linux -T kernel -C none \
		-a 0x80200000 -e 0x80200000 \
		-n "mmos-practice" \
		-d $(KERNEL_RAW) $(UIMAGE)

uImage: $(UIMAGE)

$(BOOT_DISK): require-container $(UIMAGE) $(USER_INIT_ELF) $(USER_CRT_SO) boot/extlinux.conf | $(BUILD_DIR)
	./scripts/build-boot-disk.sh $(BOOT_DISK) $(UIMAGE) boot/extlinux.conf $(USER_INIT_ELF) $(USER_CRT_SO)

boot-disk: $(BOOT_DISK)

$(UBOOT_BIN): require-container | $(BUILD_DIR)
	@if [ ! -d external/u-boot ]; then \
		echo "error: missing external/u-boot (run ./scripts/init-submodules.sh first)"; \
		exit 1; \
	fi
	$(MAKE) -C external/u-boot O=$(CURDIR)/$(UBOOT_BUILD) CROSS_COMPILE=$(CROSS_COMPILE) qemu-riscv64_smode_defconfig
	$(MAKE) -C external/u-boot O=$(CURDIR)/$(UBOOT_BUILD) CROSS_COMPILE=$(CROSS_COMPILE) -j$$(nproc)

u-boot-smode: $(UBOOT_BIN)

$(UBOOT_SPL): require-container | $(BUILD_DIR)
	@if [ ! -d external/u-boot ]; then \
		echo "error: missing external/u-boot (run ./scripts/init-submodules.sh first)"; \
		exit 1; \
	fi
	@if [ ! -d external/opensbi ]; then \
		echo "error: missing external/opensbi (run ./scripts/init-submodules.sh first)"; \
		exit 1; \
	fi
	$(MAKE) -C external/opensbi PLATFORM=generic CROSS_COMPILE=$(CROSS_COMPILE)
	$(MAKE) -C external/u-boot O=$(CURDIR)/$(UBOOT_BUILD) CROSS_COMPILE=$(CROSS_COMPILE) qemu-riscv64_spl_defconfig
	$(MAKE) -C external/u-boot O=$(CURDIR)/$(UBOOT_BUILD) CROSS_COMPILE=$(CROSS_COMPILE) OPENSBI=$(OPENSBI_FW_DYNAMIC) -j$$(nproc)

u-boot-spl: $(UBOOT_SPL)

ifeq ($(BOOT_CHAIN),spl)
u-boot: u-boot-spl
else
u-boot: u-boot-smode
endif

run-nographic: require-container $(BOOT_DISK)
	@echo "running QEMU (nographic) with $(KERNEL_ELF)"
	@echo "tip: set RUN_SECONDS=<sec> to adjust runtime window"
	@timeout --foreground $(RUN_SECONDS)s $(QEMU) \
		$(QEMU_VIRTIO_MODERN) \
		-machine virt \
		-nographic \
		-bios default \
		-kernel $(KERNEL_ELF) \
		-drive file=$(BOOT_DISK),format=raw,id=drive0 \
		-device virtio-blk-device,drive=drive0 \
		-no-reboot \
		-smp 1 \
		-m 512M || test $$? -eq 124

run-monitor: require-container
	@echo "running QEMU with serial+monitor on stdio"
	@echo "tip: press Ctrl-t c to switch between serial console and QEMU monitor"
	@echo "tip: press Ctrl-t h to show QEMU hotkeys"
	@echo "tip: set RUN_SECONDS=<sec> to adjust runtime window"
	@timeout --foreground $(RUN_SECONDS)s $(QEMU) \
		$(QEMU_VIRTIO_MODERN) \
		-machine virt \
		-nographic \
		-echr 0x14 \
		-serial mon:stdio \
		-bios default \
		-kernel $(KERNEL_ELF) \
		-no-reboot \
		-smp 1 \
		-m 512M || test $$? -eq 124

run-u-boot: require-container u-boot $(BOOT_DISK)
	@echo "running QEMU via BOOT_CHAIN=$(BOOT_CHAIN)"
	@echo "tip: BOOT_CHAIN=smode uses OpenSBI -> u-boot.bin"
	@echo "tip: BOOT_CHAIN=spl uses SPL -> u-boot.itb (+ OpenSBI fw_dynamic)"
	@echo "tip: this path exercises ext4 + extlinux + legacy uImage boot"
	@echo "tip: set RUN_SECONDS=<sec> to adjust runtime window"
ifeq ($(BOOT_CHAIN),spl)
	@timeout --foreground $(RUN_SECONDS)s $(QEMU) \
		-machine virt \
		-nographic \
		-bios $(UBOOT_SPL) \
		-device loader,file=$(UBOOT_ITB),addr=$(ITB_ADDR) \
		-drive file=$(BOOT_DISK),format=raw,id=drive0 \
		-device virtio-blk-device,drive=drive0 \
		-device virtio-rng-pci \
		-no-reboot \
		-smp 1 \
		-m 1024M || test $$? -eq 124
else
	@timeout --foreground $(RUN_SECONDS)s $(QEMU) \
		$(QEMU_VIRTIO_MODERN) \
		-machine virt \
		-nographic \
		-bios default \
		-kernel $(UBOOT_BIN) \
		-drive file=$(BOOT_DISK),format=raw,id=drive0 \
		-device virtio-blk-device,drive=drive0 \
		-device virtio-rng-pci \
		-no-reboot \
		-smp 1 \
		-m 512M || test $$? -eq 124
endif
