#!/usr/bin/env python3
# Copyright 2026 Sungjae Park
# SPDX-License-Identifier: Apache-2.0

"""Interactive TUI verifier for mmos-practice chapter checkpoints.

Features:
- Host target selection: localhost, , 
- Independent chapter-set selection
- Live command/log streaming in fixed panes (no full-screen scroll flood)
- Per-step and per-chapter pass/fail summary
"""

from __future__ import annotations

import argparse
import curses
import os
import re
import shlex
import subprocess
import textwrap
import time
from collections import deque
from dataclasses import dataclass, field
from pathlib import Path
from typing import Deque, List, Optional, Sequence, Tuple


LOCAL_REPO_DEFAULT = str(Path(__file__).resolve().parents[1])
REMOTE_REPO_DEFAULT = "/home/nicesj/workspace/mmrnd-lab/mmos-practice"

HOSTS = ["localhost", "", ""]
CONSOLES = ["sbi", "mmio"]
BOOT_CHAINS = ["smode", "spl"]
ANSI_CSI_RE = re.compile(r"\x1B\[[0-?]*[ -/]*[@-~]")
ANSI_OSC_RE = re.compile(r"\x1B\][^\x07\x1B]*(?:\x07|\x1B\\)")
ANSI_ESC_RE = re.compile(r"\x1B[@-Z\\-_]")
HUNK_RE = re.compile(r"^@@ -\d+(?:,\d+)? \+(\d+)(?:,(\d+))? @@")
STRING_RE = re.compile(r'"([^"\\]|\\.)*"|\'([^\'\\]|\\.)*\'')
NUMBER_RE = re.compile(r"\b(?:0x[0-9a-fA-F]+|\d+)\b")


@dataclass(frozen=True)
class UiTheme:
    header: int = 0
    panel_title: int = 0
    selected_item: int = 0
    running_state: int = 0
    idle_state: int = 0
    pass_text: int = 0
    fail_text: int = 0
    warn_text: int = 0
    skip_text: int = 0
    info_text: int = 0
    search_hit: int = 0
    file_added: int = 0
    file_modified: int = 0
    file_deleted: int = 0
    line_no: int = 0
    line_added: int = 0
    line_modified: int = 0
    line_added_bg: int = 0
    line_modified_bg: int = 0
    syn_keyword: int = 0
    syn_string: int = 0
    syn_comment: int = 0
    syn_number: int = 0
    syn_keyword_add_bg: int = 0
    syn_string_add_bg: int = 0
    syn_comment_add_bg: int = 0
    syn_number_add_bg: int = 0
    syn_keyword_mod_bg: int = 0
    syn_string_mod_bg: int = 0
    syn_comment_mod_bg: int = 0
    syn_number_mod_bg: int = 0


@dataclass
class Step:
    name: str
    cmd: str
    allow_failure: bool = False
    expected_pattern: Optional[str] = None
    must_contain: Sequence[str] = ()


@dataclass
class ChapterSet:
    key: str
    label: str
    commit_candidates: Sequence[str]
    steps: Sequence[Step]
    note: str = ""


@dataclass
class StepResult:
    chapter_key: str
    chapter_label: str
    step_name: str
    status: str
    elapsed_sec: float
    commit: str
    cmd: str
    detail: str = ""


@dataclass
class SourceFileEntry:
    path: str
    status: str
    old_path: str = ""


@dataclass
class SourceViewerState:
    active: bool = False
    chapter_label: str = ""
    commit: str = ""
    prev_commit: str = ""
    files: List[SourceFileEntry] = field(default_factory=list)
    file_idx: int = 0
    scroll: int = 0
    content: List[str] = field(default_factory=list)
    line_kind: dict[int, str] = field(default_factory=dict)
    language: str = "plain"
    note: str = ""
    cache: dict[str, Tuple[List[str], dict[int, str], str, str]] = field(default_factory=dict)


@dataclass
class AppState:
    host_idx: int = 0
    selected_idx: int = 0
    checked: List[bool] = field(default_factory=list)
    running: bool = False
    stop_requested: bool = False
    current_action: str = "idle"
    current_cmd: str = ""
    log_lines: Deque[str] = field(default_factory=lambda: deque(maxlen=800))
    log_follow: bool = True
    log_anchor: int = 0
    log_search: str = ""
    search_hits: List[int] = field(default_factory=list)
    search_pos: int = -1
    results: List[StepResult] = field(default_factory=list)
    status_line: str = "Ready"
    resolved_commits: dict[str, str] = field(default_factory=dict)
    theme: UiTheme = field(default_factory=UiTheme)
    source_view: SourceViewerState = field(default_factory=SourceViewerState)
    sync_before_run: bool = True
    rebuild_image_before_run: bool = True
    console_idx: int = 0
    boot_chain_idx: int = 0
    focus_idx: int = 0  # 0=chapters, 1=menu, 2=logs
    menu_idx: int = 0


CHAPTERS: List[ChapterSet] = [
    ChapterSet(
        key="ch04_kernel",
        label="Ch04 Kernel Bringup Foundation",
        commit_candidates=["d19e9e7"],
        steps=[
            Step("build", "make mmos.bin PROFILE=release"),
            Step(
                "run",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=("[mmos-practice] console backend=", "[chapter] ch04 kernel bringup"),
            ),
        ],
        note="old stage: make test target not available; runtime signature=console backend line",
    ),
    ChapterSet(
        key="ch05_test_core",
        label="Ch05 UART Bringup (Practice Anchor: Test Core)",
        commit_candidates=["ede1e09"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=("[chapter] ch05 test core",),
            ),
        ],
    ),
    ChapterSet(
        key="ch06_memory",
        label="Ch06 Memory Management",
        commit_candidates=["11981bb"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=("[chapter] ch06 memory",),
            ),
        ],
    ),
    ChapterSet(
        key="ch07_uboot",
        label="Ch07 U-Boot and Storage Boot Flow",
        commit_candidates=["a2ef2c8"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-u-boot",
                "make run-u-boot PROFILE=release RUN_SECONDS=4",
                must_contain=("U-Boot", "[mmos-practice] console backend=", "[chapter] ch07 u-boot flow"),
            ),
        ],
        note="runtime signature=OpenSBI -> U-Boot -> MMOS",
    ),
    ChapterSet(
        key="ch08_trap_scaffold",
        label="Ch08 Testing and Debugging (Practice Anchor: Trap)",
        commit_candidates=["ae85946"],
        steps=[
            Step(
                "test+build",
                "make test && make mmos.bin PROFILE=release",
                allow_failure=True,
                expected_pattern="csrsi sie, 0x222",
            ),
            Step(
                "run-u-boot",
                "make run-u-boot PROFILE=release RUN_SECONDS=4",
                must_contain=("U-Boot", "trap/irq", "[chapter] ch08 trap scaffold"),
            ),
        ],
        note="known historical failure on old chain due csr immediate",
    ),
    ChapterSet(
        key="ch10_trap_syscall",
        label="Ch10 Trap Dispatch and Syscall",
        commit_candidates=["840b365"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-u-boot",
                "make run-u-boot PROFILE=release RUN_SECONDS=4",
                must_contain=("trap/irq", "[chapter] ch10 trap syscall"),
            ),
        ],
        note="runtime signature=trap/irq path prints on boot",
    ),
    ChapterSet(
        key="ch11_sv39",
        label="Ch11 Sv39 Kernel Paging",
        commit_candidates=["ca0f226"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-u-boot",
                "make run-u-boot PROFILE=release RUN_SECONDS=4",
                must_contain=("trap/irq", "[chapter] ch11 sv39"),
            ),
        ],
        note="runtime signature=sv39 stage still reaches trap/irq boot line",
    ),
    ChapterSet(
        key="ch12_user_init",
        label="Ch12 User Init Bringup",
        commit_candidates=["89ca87f"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-u-boot",
                "make run-u-boot PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[mmos-practice] user init context prepared (SPP=U)",
                    "[mmos-practice] user init demo exit=0",
                    "[chapter] ch12 user init",
                ),
            ),
        ],
        note="runtime signature=user init context + demo exit",
    ),
    ChapterSet(
        key="ch14_device",
        label="Ch14 Block Device Foundation",
        commit_candidates=["b63f713"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-u-boot",
                "make run-u-boot PROFILE=release RUN_SECONDS=4",
                must_contain=("running QEMU via OpenSBI -> U-Boot -> MMOS", "[chapter] ch14 device layer"),
            ),
        ],
        note="runtime signature=storage boot chain via U-Boot",
    ),
    ChapterSet(
        key="ch15_fs",
        label="Ch15 Kernel FS Foundation",
        commit_candidates=["5a9772d"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-u-boot",
                "make run-u-boot PROFILE=release RUN_SECONDS=4",
                must_contain=("running QEMU via OpenSBI -> U-Boot -> MMOS", "[chapter] ch15 fs layer"),
            ),
        ],
        note="runtime signature=ext4/extlinux boot path active",
    ),
    ChapterSet(
        key="ch17_elf",
        label="Ch17 ELF Loader and Exec Prep",
        commit_candidates=["17491b1"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-u-boot",
                "make run-u-boot PROFILE=release RUN_SECONDS=4",
                must_contain=("[mmos-practice] user init demo exit=0", "[chapter] ch17 elf loader"),
            ),
        ],
        note="runtime signature=loaded user init demo exits cleanly",
    ),
    ChapterSet(
        key="ch18_exec_prep",
        label="Ch18 User ELF Init Exec Prep",
        commit_candidates=["84e0986"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=("[mmos-practice] user init context prepared (SPP=U)", "[chapter] ch18 exec prep"),
            ),
        ],
        note="runtime signature=exec-ready init context appears",
    ),
    ChapterSet(
        key="ch19_user_vm_fault",
        label="Ch19 User VM + Page Fault",
        commit_candidates=["3dc7ec5", "6ad1b57"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=("[mmos-practice] trap/irq scaffold ready", "[chapter] ch19 trap outcome bridge"),
            ),
        ],
        note="runtime signature=trap path alive after vm/fault integration",
    ),
    ChapterSet(
        key="ch20_scheduler_switch",
        label="Ch20 Context Switch and Scheduling",
        commit_candidates=["2e04fd6"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=("[mmos-practice] trap/irq scaffold ready", "[chapter] ch20 scheduler switch"),
            ),
        ],
        note="runtime signature=kernel boots with scheduler-enabled codebase",
    ),
    ChapterSet(
        key="ch21_process_model",
        label="Ch21 Process Model Foundation (PCB/TCB)",
        commit_candidates=["697fbd7"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=("[mmos-practice] trap/irq scaffold ready", "[chapter] ch21 process model"),
            ),
        ],
        note="runtime signature=process model stage boots stably",
    ),
    ChapterSet(
        key="ch22_fork_exec_wait",
        label="Ch22 Fork/Exec/Wait (Runtime Demo)",
        commit_candidates=["30b90b2"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch22 fork exec wait demo",
                    "[sched] tick current=1",
                    "[sched] switch prev=1 next=2",
                    "[proc] fork parent=1 child=2",
                    "[proc] exec pid=2 path=/bin/sh",
                    "[proc] wait reap pid=2 code=0",
                ),
            ),
        ],
        note="runtime signature=scheduler/process model demo logs are visible in QEMU",
    ),
    ChapterSet(
        key="ch24_user_app_build",
        label="Ch24 User App Build Foundation",
        commit_candidates=["03ae40b"],
        steps=[
            Step(
                "user-init-build",
                "make user-init PROFILE=release && riscv64-linux-gnu-readelf -h build/user-init.elf | grep 'Machine:'",
                must_contain=(
                    "Machine:                           RISC-V",
                ),
            ),
            Step(
                "boot-disk-inject",
                "make boot-disk PROFILE=release && riscv64-linux-gnu-readelf -s build/user-init.elf | grep '_start'",
                must_contain=(
                    "_start",
                ),
            ),
        ],
        note="runtime signature=real user-init ELF is built/injected and can be inspected with readelf (-S/-l) for section+segment sanity",
    ),
    ChapterSet(
        key="ch25_user_stack_abi",
        label="Ch25 User ABI Stack Layout (argc/argv/envp)",
        commit_candidates=["7656007"],
        steps=[
            Step(
                "user-stack-tests",
                "make test && cargo test --lib --target x86_64-unknown-linux-gnu user_stack_builder_writes_argc_argv_envp_layout",
                must_contain=(
                    "user_stack_builder_writes_argc_argv_envp_layout ... ok",
                ),
            ),
        ],
        note="runtime signature=initial user stack ABI is validated by focused tests",
    ),
    ChapterSet(
        key="ch26_user_init_launch_handoff",
        label="Ch26 User Init Launch Handoff",
        commit_candidates=["f1d6e68"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch26 user init launch handoff",
                    "[launch] vm+stack+trapframe ready for first sret",
                ),
            ),
        ],
        note="runtime signature=exec/vm/stack handoff checkpoints are visible before first sret",
    ),
    ChapterSet(
        key="ch27_runtime_user_init_load",
        label="Ch27 Runtime User Init Load + Handoff",
        commit_candidates=["bcff4c9"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch18 exec prep",
                    "[runtime] loaded /sbin/init from virtio ext4 into exec image",
                    "[chapter] ch26 user init launch handoff",
                ),
            ),
        ],
        note="runtime signature=kernel path mounts the virtio ext4 image and prepares first user handoff",
    ),
    ChapterSet(
        key="ch28_first_user_sret",
        label="Ch28 First User SRET Execution",
        commit_candidates=["877a62e"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch28 first user sret",
                    "[user-init] hello from user app",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=kernel enters U-mode and user-init emits syscall write/exit logs",
    ),
    ChapterSet(
        key="ch31_character_device",
        label="Ch31 Character Device Foundation",
        commit_candidates=["ba8bb0b"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
        ],
        note="unit signature=CharacterDevice trait, CharError, and mock character device behavior",
    ),
    ChapterSet(
        key="ch32_serial_character_device",
        label="Ch32 Serial Character Device",
        commit_candidates=["34ef23b"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch32 serial character device",
                    "[chapter] ch28 first user sret",
                    "[user-init] hello from user app",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=console output is routed through the serial CharacterDevice path",
    ),
    ChapterSet(
        key="ch33_fd_table_stdio",
        label="Ch33 File Descriptor Table and Stdio",
        commit_candidates=["712bc4c"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch33 fd table stdio",
                    "[chapter] ch28 first user sret",
                    "[user-init] hello from user app",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=write(1, ...) resolves through the preopened fd table stdout slot",
    ),
    ChapterSet(
        key="ch34_devfs_device_nodes",
        label="Ch34 DevFS Namespace and Device Nodes",
        commit_candidates=["efe9ae5"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch34 devfs device nodes",
                    "[chapter] ch33 fd table stdio",
                    "[user-init] hello from user app",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=/dev/console resolves through the devfs namespace model",
    ),
    ChapterSet(
        key="ch35_open_path_fd_allocation",
        label="Ch35 Open Path and FD Allocation",
        commit_candidates=["62891af"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch35 open syscall path",
                    "[chapter] ch34 devfs device nodes",
                    "[user-init] hello from user app",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=open path helper allocates a new fd for /dev/console",
    ),
    ChapterSet(
        key="ch36_process_fd_context",
        label="Ch36 Process FD Context",
        commit_candidates=["480edc8"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch36 process fd context",
                    "[chapter] ch35 open syscall path",
                    "[user-init] hello from user app",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=open result remains in a process IO fd table and is reused by write",
    ),
    ChapterSet(
        key="ch37_user_copyin",
        label="Ch37 User Pointer Copyin",
        commit_candidates=["6efaccc"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch37 user copyin",
                    "[chapter] ch36 process fd context",
                    "[user-init] hello from user app",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=open path and write buffer pass through explicit user copyin helpers",
    ),
    ChapterSet(
        key="ch38_vm_checked_copyin",
        label="Ch38 VM Checked User Copyin",
        commit_candidates=["edc3a09"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch38 vm checked copyin",
                    "[chapter] ch37 user copyin",
                    "[user-init] hello from user app",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=copyin checks the current user VM area permissions before copying",
    ),
    ChapterSet(
        key="ch39_fd_write_syscall",
        label="Ch39 FD Write Syscall",
        commit_candidates=["f46b61c"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch39 fd write syscall",
                    "[chapter] ch38 vm checked copyin",
                    "[user-init] hello from user app",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=write(fd, user_buf, len) uses process fd table after checked copyin",
    ),
    ChapterSet(
        key="ch40_current_process_io_context",
        label="Ch40 Current Process IO Context",
        commit_candidates=["715ac89"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch40 current process io context",
                    "[chapter] ch39 fd write syscall",
                    "[user-init] hello from user app",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=write syscall uses the current process IO context wrapper",
    ),
    ChapterSet(
        key="ch41_task_owner_io_lookup",
        label="Ch41 Task Owner IO Lookup",
        commit_candidates=["e2d853e"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch41 task owner io lookup",
                    "[chapter] ch40 current process io context",
                    "[user-init] hello from user app",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=current task owner_pid resolves the process IO context used by write",
    ),
    ChapterSet(
        key="ch42_process_owned_io_runtime",
        label="Ch42 Process Owned IO Runtime",
        commit_candidates=["e05b920"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch42 process owned io runtime",
                    "[chapter] ch41 task owner io lookup",
                    "[user-init] hello from user app",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=process creation registers stdio IO context used through task owner lookup",
    ),
    ChapterSet(
        key="ch43_close_fd_lifecycle",
        label="Ch43 Close FD Lifecycle",
        commit_candidates=["c54dc2e"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch43 close fd lifecycle",
                    "[chapter] ch42 process owned io runtime",
                    "[user-init] hello from user app",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=close(fd) releases the slot and later write returns EBADF",
    ),
    ChapterSet(
        key="ch44_read_fd_copyout",
        label="Ch44 Read FD Copyout",
        commit_candidates=["4864788"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch44 read fd copyout",
                    "[chapter] ch43 close fd lifecycle",
                    "[user-init] hello from user app",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=read(fd, user_buf, len) checks VM write permission and copyout path",
    ),
    ChapterSet(
        key="ch45_console_read_bytes",
        label="Ch45 Console Read Bytes",
        commit_candidates=["fcdd437"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch45 console read bytes",
                    "[chapter] ch44 read fd copyout",
                    "[user-init] hello from user app",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=console read path returns real bytes that are copied out to user memory",
    ),
    ChapterSet(
        key="ch46_nonblocking_read_wouldblock",
        label="Ch46 Nonblocking Read WouldBlock",
        commit_candidates=["0e6ae7f"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch46 nonblocking read wouldblock",
                    "[chapter] ch45 console read bytes",
                    "[user-init] hello from user app",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=empty buffered console read maps WouldBlock to -EAGAIN",
    ),
    ChapterSet(
        key="ch48_blocking_read_wait_queue",
        label="Ch48 Blocking Read Wait Queue",
        commit_candidates=["28fba0c"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch48 blocking read wait queue",
                    "[chapter] ch46 nonblocking read wouldblock",
                    "[user-init] hello from user app",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=WouldBlock can park the current task on a read wait queue",
    ),
    ChapterSet(
        key="ch49_serial_rx_wake_reader",
        label="Ch49 Serial RX Wake Reader",
        commit_candidates=["d8ab2ed"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch49 serial rx wake reader",
                    "[chapter] ch48 blocking read wait queue",
                    "[user-init] hello from user app",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=serial RX event wakes a task parked on the read wait queue",
    ),
    ChapterSet(
        key="ch50_blocked_syscall_scheduler_handoff",
        label="Ch50 Blocked Syscall Scheduler Handoff",
        commit_candidates=["9e969cb"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch50 blocked syscall scheduler handoff",
                    "[chapter] ch49 serial rx wake reader",
                    "[user-init] hello from user app",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=blocked read syscall asks trap handling to schedule another task",
    ),
    ChapterSet(
        key="ch51_schedule_next_runnable_task",
        label="Ch51 Schedule Next Runnable Task",
        commit_candidates=["366a719"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch51 schedule next runnable task",
                    "[chapter] ch50 blocked syscall scheduler handoff",
                    "[user-init] hello from user app",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=TrapOutcome::Schedule selects the next runnable task after a blocked syscall",
    ),
    ChapterSet(
        key="ch52_idle_when_no_runnable_task",
        label="Ch52 Idle When No Runnable Task",
        commit_candidates=["18156fc"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch52 idle when no runnable task",
                    "[chapter] ch51 schedule next runnable task",
                    "[user-init] hello from user app",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=scheduler reports an idle decision when every task is blocked",
    ),
    ChapterSet(
        key="ch53_idle_wait_for_interrupt",
        label="Ch53 Idle Wait For Interrupt",
        commit_candidates=["20ecd9a"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch53 idle wait for interrupt",
                    "[chapter] ch52 idle when no runnable task",
                    "[user-init] hello from user app",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=idle decision maps to a wait-for-interrupt action without entering an endless idle loop",
    ),
    ChapterSet(
        key="ch54_restore_next_task_frame",
        label="Ch54 Restore Next Task Frame",
        commit_candidates=["4b02833"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch54 restore next task frame",
                    "[chapter] ch53 idle wait for interrupt",
                    "[user-init] hello from user app",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=switched schedule decision restores the selected task trap frame into the dispatch frame",
    ),
    ChapterSet(
        key="ch55_trap_schedule_dispatch",
        label="Ch55 Trap Schedule Dispatch",
        commit_candidates=["f2ea044"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch55 trap schedule dispatch",
                    "[chapter] ch54 restore next task frame",
                    "[user-init] hello from user app",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=TrapOutcome::Schedule is dispatched into scheduler handoff and next task frame restore",
    ),
    ChapterSet(
        key="ch56_trap_runtime_scheduler_state",
        label="Ch56 Trap Runtime Scheduler State",
        commit_candidates=["d298fd0"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch56 trap runtime scheduler state",
                    "[chapter] ch55 trap schedule dispatch",
                    "[user-init] hello from user app",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=trap runtime owns scheduler state used by schedule dispatch",
    ),
    ChapterSet(
        key="ch57_global_trap_runtime_state",
        label="Ch57 Global Trap Runtime State",
        commit_candidates=["bf3eec2"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch57 global trap runtime state",
                    "[chapter] ch56 trap runtime scheduler state",
                    "[user-init] hello from user app",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=global trap runtime state can dispatch schedule handoff",
    ),
    ChapterSet(
        key="ch58_trap_dispatch_global_runtime",
        label="Ch58 Trap Dispatch Global Runtime",
        commit_candidates=["7ffd712"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch58 trap dispatch global runtime",
                    "[chapter] ch57 global trap runtime state",
                    "[user-init] hello from user app",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=trap dispatch schedule outcome uses global trap runtime",
    ),
    ChapterSet(
        key="ch59_launch_task_global_runtime",
        label="Ch59 Launch Task Global Runtime",
        commit_candidates=["0b29714"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch59 launch task global runtime",
                    "[chapter] ch58 trap dispatch global runtime",
                    "[user-init] hello from user app",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=user init launch trap frame is installed into global trap runtime",
    ),
    ChapterSet(
        key="ch61_user_process_creation_path",
        label="Ch61 User Process Creation Path",
        commit_candidates=["bd084fe"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch61 user process creation path",
                    "[chapter] ch59 launch task global runtime",
                    "[user-init] hello from user app",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=launch plan is registered through the common user process creation path",
    ),
    ChapterSet(
        key="ch62_second_user_task_switch",
        label="Ch62 Second User Task Switch",
        commit_candidates=["17bd102"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch62 second user task switch",
                    "[chapter] ch61 user process creation path",
                    "[chapter] ch59 launch task global runtime",
                    "[user-init] hello from user app",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=two created user task contexts can switch after the first blocks",
    ),
    ChapterSet(
        key="ch63_task_address_space_token",
        label="Ch63 Task Address Space Token",
        commit_candidates=["b6b4016"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch63 task address space token",
                    "[chapter] ch62 second user task switch",
                    "[chapter] ch61 user process creation path",
                    "[user-init] hello from user app",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=schedule decision can resolve the next process address-space token",
    ),
    ChapterSet(
        key="ch64_address_space_switch_apply",
        label="Ch64 Address Space Switch Apply",
        commit_candidates=["5d7eb11"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch64 address space switch apply",
                    "[chapter] ch63 task address space token",
                    "[chapter] ch62 second user task switch",
                    "[user-init] hello from user app",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=address-space switch token is applied to the active satp slot",
    ),
    ChapterSet(
        key="ch65_satp_commit_boundary",
        label="Ch65 SATP Commit Boundary",
        commit_candidates=["f5008a8"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch65 satp commit boundary",
                    "[chapter] ch64 address space switch apply",
                    "[chapter] ch63 task address space token",
                    "[user-init] hello from user app",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=satp commit plan is ready before the hardware write boundary",
    ),
    ChapterSet(
        key="ch66_user_address_relocation",
        label="Ch66 User Address Relocation",
        commit_candidates=["44729b3"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch66 user address relocation",
                    "[chapter] ch65 satp commit boundary",
                    "[chapter] ch64 address space switch apply",
                    "[runtime] user virtual entry waits for satp commit",
                ),
            ),
        ],
        note="runtime signature=user space is moved away from the kernel identity mapping slot",
    ),
    ChapterSet(
        key="ch67_satp_commit_preflight",
        label="Ch67 SATP Commit Preflight",
        commit_candidates=["e74e921"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch67 satp commit preflight",
                    "[chapter] ch66 user address relocation",
                    "[chapter] ch65 satp commit boundary",
                    "[runtime] user virtual entry waits for satp commit",
                ),
            ),
        ],
        note="runtime signature=user/kernel mappings pass the satp commit preflight",
    ),
    ChapterSet(
        key="ch68_kernel_trap_stack_switch",
        label="Ch68 Kernel Trap Stack Switch",
        commit_candidates=["6885ba8"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch68 kernel trap stack switch",
                    "[chapter] ch67 satp commit preflight",
                    "[chapter] ch66 user address relocation",
                    "[runtime] user virtual entry waits for satp commit",
                ),
            ),
        ],
        note="runtime signature=kernel trap stack frame reservation is ready before satp commit",
    ),
    ChapterSet(
        key="ch69_trap_entry_scratch_swap",
        label="Ch69 Trap Entry Scratch Swap",
        commit_candidates=["bbaeca1"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch69 trap entry scratch swap",
                    "[chapter] ch68 kernel trap stack switch",
                    "[chapter] ch67 satp commit preflight",
                    "[runtime] user virtual entry waits for satp commit",
                ),
            ),
        ],
        note="runtime signature=sscratch is prepared to swap from user sp to kernel trap frame sp",
    ),
    ChapterSet(
        key="ch70_trap_entry_asm_stack_switch",
        label="Ch70 Trap Entry Assembly Stack Switch",
        commit_candidates=["0426b15"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch70 trap entry asm stack switch",
                    "[chapter] ch69 trap entry scratch swap",
                    "[chapter] ch68 kernel trap stack switch",
                    "[runtime] user virtual entry waits for satp commit",
                ),
            ),
        ],
        note="runtime signature=trap entry assembly now starts from the prepared kernel trap frame stack",
    ),
    ChapterSet(
        key="ch71_enter_user_sscratch_prepare",
        label="Ch71 Enter User SScratch Prepare",
        commit_candidates=["2dba431"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch71 enter user sscratch prepare",
                    "[chapter] ch70 trap entry asm stack switch",
                    "[chapter] ch69 trap entry scratch swap",
                    "[runtime] user virtual entry waits for satp commit",
                ),
            ),
        ],
        note="runtime signature=enter_user prepares sscratch with the kernel trap frame stack before sret",
    ),
    ChapterSet(
        key="ch72_satp_hardware_commit_gate",
        label="Ch72 SATP Hardware Commit Gate",
        commit_candidates=["d346ae5"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch72 satp hardware commit gate",
                    "[chapter] ch71 enter user sscratch prepare",
                    "[chapter] ch70 trap entry asm stack switch",
                    "[runtime] user virtual entry waits for satp commit",
                ),
            ),
        ],
        note="runtime signature=satp commit plan is accepted by the hardware CSR write gate",
    ),
    ChapterSet(
        key="ch73_user_launch_csr_sequence",
        label="Ch73 User Launch CSR Sequence",
        commit_candidates=["580124f"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch73 user launch csr sequence",
                    "[chapter] ch72 satp hardware commit gate",
                    "[chapter] ch71 enter user sscratch prepare",
                    "[runtime] user virtual entry waits for satp commit",
                ),
            ),
        ],
        note="runtime signature=user launch CSR sequence orders sscratch, satp, sfence.vma, and sret state",
    ),
    ChapterSet(
        key="ch74_root_page_table_storage",
        label="Ch74 PMM-backed Root Frame",
        commit_candidates=["1a6395e"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch74 root page table storage",
                    "[chapter] ch73 user launch csr sequence",
                    "[chapter] ch72 satp hardware commit gate",
                    "[runtime] user virtual entry waits for satp commit",
                ),
            ),
        ],
        note="runtime signature=satp PPN is derived from PMM-backed root frame storage",
    ),
    ChapterSet(
        key="ch75_rooted_user_launch_satp",
        label="Ch75 Rooted User Launch SATP",
        commit_candidates=["fcab164"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch75 rooted user launch satp",
                    "[chapter] ch74 root page table storage",
                    "[chapter] ch73 user launch csr sequence",
                    "[runtime] user virtual entry waits for satp commit",
                ),
            ),
        ],
        note="runtime signature=user launch sequence is bound to the satp derived from the root frame",
    ),
    ChapterSet(
        key="ch76_process_root_satp_token",
        label="Ch76 Process Root SATP Token",
        commit_candidates=["f98b6ee"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch76 process root satp token",
                    "[chapter] ch75 rooted user launch satp",
                    "[chapter] ch74 root page table storage",
                    "[runtime] user virtual entry waits for satp commit",
                ),
            ),
        ],
        note="runtime signature=process address-space token is rebound to the root-frame satp",
    ),
    ChapterSet(
        key="ch77_rooted_satp_commit_sequence",
        label="Ch77 Rooted SATP Commit Sequence",
        commit_candidates=["24fcd33"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch77 rooted satp commit sequence",
                    "[chapter] ch76 process root satp token",
                    "[chapter] ch75 rooted user launch satp",
                    "[runtime] user virtual entry waits for satp commit",
                ),
            ),
        ],
        note="runtime signature=PCB token flows through commit plan and rooted launch sequence",
    ),
    ChapterSet(
        key="ch78_kernel_satp_runtime_guard",
        label="Ch78 Kernel SATP Runtime Guard",
        commit_candidates=["75bff0e"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch78 kernel satp runtime guard",
                    "[chapter] ch77 rooted satp commit sequence",
                    "[chapter] ch76 process root satp token",
                    "[runtime] user virtual entry waits for satp commit",
                ),
            ),
        ],
        note="runtime signature=kernel code, stack, and trap entry remain mapped under the satp root",
    ),
    ChapterSet(
        key="ch79_process_address_space_root",
        label="Ch79 Process Address Space Root Frame",
        commit_candidates=["8ad5df6"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch79 process address space root",
                    "[chapter] ch78 kernel satp runtime guard",
                    "[chapter] ch77 rooted satp commit sequence",
                    "[runtime] user virtual entry waits for satp commit",
                ),
            ),
        ],
        note="runtime signature=process address space owns root table storage and satp token",
    ),
    ChapterSet(
        key="ch80_pcb_address_space_handle",
        label="Ch80 PCB Address Space Handle",
        commit_candidates=["3110a1f"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch80 pcb address space handle",
                    "[chapter] ch79 process address space root",
                    "[chapter] ch78 kernel satp runtime guard",
                    "[runtime] user virtual entry waits for satp commit",
                ),
            ),
        ],
        note="runtime signature=PCB address_space_id resolves satp through address-space table",
    ),
    ChapterSet(
        key="ch81_satp_hardware_commit_probe",
        label="Ch81 SATP Hardware Commit Probe",
        commit_candidates=["5784d1e"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch81 satp hardware commit probe",
                    "[chapter] ch80 pcb address space handle",
                    "[chapter] ch79 process address space root",
                    "[runtime] user virtual entry waits for satp commit",
                ),
            ),
        ],
        note="runtime signature=temporary satp CSR write, sfence.vma, and restore-to-bare probe",
    ),
    ChapterSet(
        key="ch82_user_frame_backed_image",
        label="Ch82 User Frame Backed Image",
        commit_candidates=["74b9ec8"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch82 user frame backed image",
                    "[chapter] ch81 satp hardware commit probe",
                    "[chapter] ch80 pcb address space handle",
                    "[runtime] user virtual entry waits for satp commit",
                ),
            ),
        ],
        note="runtime signature=user entry and stack are planned on PMM-backed frames before persistent satp",
    ),
    ChapterSet(
        key="ch83_user_frame_backed_page_mapping",
        label="Ch83 User Frame Backed Page Mapping",
        commit_candidates=["e7ce69e"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch83 user frame backed page mapping",
                    "[chapter] ch82 user frame backed image",
                    "[chapter] ch81 satp hardware commit probe",
                    "[runtime] user virtual entry waits for satp commit",
                ),
            ),
        ],
        note="runtime signature=user VA is mapped through lower-level Sv39 tables to PMM frames",
    ),
    ChapterSet(
        key="ch84_user_frame_materialize",
        label="Ch84 User Frame Materialize",
        commit_candidates=["64fcc6a"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch84 user frame materialize",
                    "[chapter] ch83 user frame backed page mapping",
                    "[chapter] ch82 user frame backed image",
                    "[runtime] user virtual entry waits for satp commit",
                ),
            ),
        ],
        note="runtime signature=ELF segment and initial stack bytes are copied into PMM-backed user frames",
    ),
    ChapterSet(
        key="ch85_persistent_satp_commit",
        label="Ch85 Persistent SATP Commit",
        commit_candidates=["3ad6605"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch85 persistent satp commit",
                    "[chapter] ch84 user frame materialize",
                    "[chapter] ch83 user frame backed page mapping",
                    "[runtime] user virtual entry waits for satp commit",
                ),
            ),
        ],
        note="runtime signature=prepared user satp is written persistently and kernel output continues under the new root",
    ),
    ChapterSet(
        key="ch86_user_memory_access_gate",
        label="Ch86 User Memory Access Gate",
        commit_candidates=["bc125fe"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch86 user memory access gate",
                    "[chapter] ch85 persistent satp commit",
                    "[chapter] ch84 user frame materialize",
                    "[runtime] user virtual entry waits for satp commit",
                ),
            ),
        ],
        note="runtime signature=supervisor opens a scoped SUM window after persistent satp commit",
    ),
    ChapterSet(
        key="ch87_first_user_sret_syscall",
        label="Ch87 First User SRET Syscall",
        commit_candidates=["88a60ea"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=6",
                must_contain=(
                    "[chapter] ch87 first user sret syscall",
                    "[chapter] ch87 first user syscall trap",
                    "[user-init] hello from user app",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=sret enters user code, write traps back, and init exits cleanly",
    ),
    ChapterSet(
        key="ch90_process_exit_runtime",
        label="Ch90 Process Exit Runtime",
        commit_candidates=["49e7539"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=6",
                must_contain=(
                    "[chapter] ch90 process exit runtime",
                    "[proc] exit pid=1 code=0 tasks=2",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=process-wide exit marks owner PCB and all owned TCBs before user init still exits cleanly",
    ),
    ChapterSet(
        key="ch91_exit_schedule_handoff",
        label="Ch91 Exit Schedule Handoff",
        commit_candidates=["1fd8a1b"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=6",
                must_contain=(
                    "[chapter] ch91 exit schedule handoff",
                    "[proc] exit handoff prev=1 next=3 owner_tasks=2 scheduler_tasks=2",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=process-wide exit removes sibling tasks from scheduler and hands off to another process",
    ),
    ChapterSet(
        key="ch92_trap_exit_process_dispatch",
        label="Ch92 Trap Exit Process Dispatch",
        commit_candidates=["75633dc"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=6",
                must_contain=(
                    "[chapter] ch92 trap exit process dispatch",
                    "[proc] trap exit dispatch prev=1 next=3 restored=0x40003000",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=TrapOutcome::Exit-style dispatch applies process exit and restores the next process frame",
    ),
    ChapterSet(
        key="ch93_process_trap_runtime_exit",
        label="Ch93 Process Trap Runtime Exit",
        commit_candidates=["e6f1eca"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=6",
                must_contain=(
                    "[chapter] ch93 process trap runtime exit",
                    "[proc] runtime exit owner=1 next=3 restored=0x40003000",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=process-aware trap runtime owns process/task tables and dispatches exit to the next process frame",
    ),
    ChapterSet(
        key="ch94_process_trap_runtime_reap",
        label="Ch94 Process Trap Runtime Reap",
        commit_candidates=["44d1aa1"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=6",
                must_contain=(
                    "[chapter] ch94 process trap runtime reap",
                    "[proc] runtime reap pid=1 code=0 tasks=2 sched=2 next=3",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=process-aware trap runtime reaps the exited owner and removes its task slots",
    ),
    ChapterSet(
        key="ch95_process_wait_child_reap",
        label="Ch95 Process Wait Child Reap",
        commit_candidates=["cbef4a7"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=6",
                must_contain=(
                    "[chapter] ch95 process wait child reap",
                    "[proc] wait parent=1 child=2 code=7 tasks=2 sched=2 next=3",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=parent wait path reaps a zombie child through the process-aware trap runtime",
    ),
    ChapterSet(
        key="ch96_wait_syscall_surface",
        label="Ch96 Wait Syscall Surface",
        commit_candidates=["605f148"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=6",
                must_contain=(
                    "[chapter] ch96 wait syscall surface",
                    "[syscall] wait parent=1 child=2 status=9",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=current process wait syscall surface reaps a zombie child and copies out status",
    ),
    ChapterSet(
        key="ch97_blocking_wait_wakeup",
        label="Ch97 Blocking Wait Wakeup",
        commit_candidates=["d31c79c"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=6",
                must_contain=(
                    "[chapter] ch97 blocking wait wakeup",
                    "[syscall] wait blocked parent=1 child=2 wake=1 status=13",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="runtime signature=parent waits before child exit, blocks, and wakes when child becomes zombie",
    ),
    ChapterSet(
        key="live_ch26_launch_handoff",
        label="Live Ch26 Runtime (HEAD)",
        commit_candidates=["HEAD"],
        steps=[
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch26 user init launch handoff",
                    "[launch] vm+stack+trapframe ready for first sret",
                ),
            ),
        ],
        note="live signature=launch handoff is built from vm+stack+trapframe probe",
    ),
    ChapterSet(
        key="live_ch27_runtime_integration",
        label="Live Ch27 Runtime (HEAD)",
        commit_candidates=["HEAD"],
        steps=[
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[runtime] loaded /sbin/init from virtio ext4 into exec image",
                    "[launch] vm+stack+trapframe ready for first sret",
                ),
            ),
        ],
        note="live signature=virtio ext4->elf->launch integration path executes in QEMU runtime",
    ),
    ChapterSet(
        key="live_ch28_first_sret",
        label="Live Ch28 Runtime (HEAD)",
        commit_candidates=["HEAD"],
        steps=[
            Step(
                "run-nographic",
                "make run-nographic PROFILE=release RUN_SECONDS=4",
                must_contain=(
                    "[chapter] ch28 first user sret",
                    "[user-init] hello from user app",
                    "[user-init] exit=0",
                ),
            ),
        ],
        note="live signature=first U-mode entry and user syscall round-trip are visible on QEMU console",
    ),
]


def check_patterns(output: str, patterns: Sequence[str]) -> Tuple[List[str], List[str]]:
    if not patterns:
        return [], []
    out_norm = output.lower()
    found: List[str] = []
    missing: List[str] = []
    for p in patterns:
        if p.lower() in out_norm:
            found.append(p)
        else:
            missing.append(p)
    return found, missing


def repo_path_for_host(host: str) -> str:
    if host == "localhost":
        return os.environ.get("MMOS_PRACTICE_LOCAL", LOCAL_REPO_DEFAULT)
    return os.environ.get("MMOS_PRACTICE_REMOTE", REMOTE_REPO_DEFAULT)


def host_exec_argv(host: str, shell_cmd: str) -> List[str]:
    if host == "localhost":
        return ["bash", "-lc", shell_cmd]
    return ["ssh", host, f"bash -lc {shlex.quote(shell_cmd)}"]


def selected_console(state: AppState) -> str:
    return CONSOLES[state.console_idx]


def selected_boot_chain(state: AppState) -> str:
    return BOOT_CHAINS[state.boot_chain_idx]


def focus_name(state: AppState) -> str:
    return ("chapters", "menu", "logs")[state.focus_idx]


def menu_items(state: AppState) -> List[Tuple[str, str]]:
    return [
        ("Host", HOSTS[state.host_idx]),
        ("Console", selected_console(state)),
        ("Boot Chain", selected_boot_chain(state)),
        ("Sync Before Run", "ON" if state.sync_before_run else "OFF"),
        ("Rebuild Image", "ON" if state.rebuild_image_before_run else "OFF"),
        ("Run Selected", "execute"),
        ("Show History", "inspect"),
        ("Show Changes", "inspect"),
        ("Source View", "open"),
        ("Stop", "request"),
        ("Clear Logs", "reset"),
        ("Search Logs", "input"),
    ]


def apply_build_options(inner_cmd: str, state: AppState) -> str:
    # Inject selected build options for every make invocation.
    # Example: "make test && make run-u-boot" ->
    #          "make CONSOLE=sbi BOOT_CHAIN=smode test && make CONSOLE=sbi BOOT_CHAIN=smode run-u-boot"
    console = selected_console(state)
    boot_chain = selected_boot_chain(state)
    opt = f"make CONSOLE={console} BOOT_CHAIN={boot_chain}"
    return re.sub(r"\bmake(?=\s)", opt, inner_cmd)


def sanitize_log_line(raw: str) -> str:
    # Normalize line first so CR-based progress output cannot overwrite borders.
    s = raw.replace("\r", "")
    # Strip common terminal control sequences (CSI / OSC / single ESC).
    s = ANSI_OSC_RE.sub("", s)
    s = ANSI_CSI_RE.sub("", s)
    s = ANSI_ESC_RE.sub("", s)

    cleaned = []
    for ch in s:
        o = ord(ch)
        # Drop all remaining C0/C1 controls except tab.
        if ch == "\t":
            cleaned.append("    ")
            continue
        if o < 32 or 127 <= o <= 159:
            continue
        cleaned.append(ch)

    # Keep logs single-line and trimmed to avoid stale drawing artifacts.
    return "".join(cleaned).strip("\n")


def fit_line(text: str, width: int) -> str:
    if width <= 0:
        return ""
    if len(text) <= width:
        return text
    if width <= 3:
        return "." * width
    return text[: width - 3] + "..."


def run_selected_action(state: AppState, stdscr: curses.window) -> None:
    state.log_lines.clear()
    state.log_follow = True
    state.log_anchor = 0
    rebuild_search_hits(state)
    state.results.clear()
    state.resolved_commits.clear()
    run_selected(stdscr, state, CHAPTERS)


def run_inspect_action(state: AppState, stdscr: curses.window, mode: str) -> None:
    state.log_follow = True
    chapter = CHAPTERS[state.selected_idx]
    run_inspect_command(
        HOSTS[state.host_idx],
        repo_path_for_host(HOSTS[state.host_idx]),
        chapter,
        state.selected_idx,
        mode,
        state,
        stdscr,
    )
    rebuild_search_hits(state)


def open_source_action(state: AppState, stdscr: curses.window) -> None:
    open_source_view_for_selected(state, stdscr)


def search_logs_action(state: AppState, stdscr: curses.window) -> None:
    query = prompt_input(stdscr, "Search (/): ", state.log_search)
    if query is not None:
        state.log_search = query.strip()
        rebuild_search_hits(state)
        if state.log_search and state.search_hits:
            jump_search(state, +1, max(1, stdscr.getmaxyx()[0] - 8))
        elif state.log_search:
            state.status_line = f"No match: {state.log_search}"
        else:
            state.status_line = "Search cleared"
            state.log_follow = True


def apply_menu_action(state: AppState, stdscr: curses.window, delta: int = 0) -> None:
    idx = state.menu_idx
    if idx == 0:
        state.host_idx = (state.host_idx + (1 if delta >= 0 else -1)) % len(HOSTS)
        state.status_line = f"Host switched to {HOSTS[state.host_idx]}"
    elif idx == 1:
        state.console_idx = (state.console_idx + (1 if delta >= 0 else -1)) % len(CONSOLES)
        state.status_line = f"Console switched to {selected_console(state)}"
    elif idx == 2:
        state.boot_chain_idx = (state.boot_chain_idx + (1 if delta >= 0 else -1)) % len(BOOT_CHAINS)
        state.status_line = f"Boot chain switched to {selected_boot_chain(state)}"
    elif idx == 3:
        state.sync_before_run = not state.sync_before_run
        state.status_line = f"sync-before-run={'ON' if state.sync_before_run else 'OFF'}"
    elif idx == 4:
        state.rebuild_image_before_run = not state.rebuild_image_before_run
        state.status_line = f"rebuild-image-before-run={'ON' if state.rebuild_image_before_run else 'OFF'}"
    elif idx == 5:
        run_selected_action(state, stdscr)
    elif idx == 6:
        run_inspect_action(state, stdscr, "history")
    elif idx == 7:
        run_inspect_action(state, stdscr, "changes")
    elif idx == 8:
        open_source_action(state, stdscr)
    elif idx == 9:
        state.stop_requested = True
        state.status_line = "Nothing running"
    elif idx == 10:
        state.log_lines.clear()
        state.log_follow = True
        state.log_anchor = 0
        rebuild_search_hits(state)
        state.status_line = "Logs cleared"
    elif idx == 11:
        search_logs_action(state, stdscr)


def append_log(state: AppState, line: str) -> None:
    state.log_lines.append(line)
    if state.log_search:
        idx = len(state.log_lines) - 1
        if state.log_search.lower() in line.lower():
            state.search_hits.append(idx)
            if state.search_pos < 0:
                state.search_pos = 0
    if state.log_follow:
        state.log_anchor = max(0, len(state.log_lines) - 1)


def rebuild_search_hits(state: AppState) -> None:
    if not state.log_search:
        state.search_hits = []
        state.search_pos = -1
        return
    needle = state.log_search.lower()
    lines = list(state.log_lines)
    state.search_hits = [i for i, ln in enumerate(lines) if needle in ln.lower()]
    state.search_pos = 0 if state.search_hits else -1


def run_capture(host: str, shell_cmd: str) -> Tuple[int, str]:
    argv = host_exec_argv(host, shell_cmd)
    proc = subprocess.run(argv, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    lines: List[str] = []
    for raw in (proc.stdout or "").splitlines():
        s = raw.replace("\r", "")
        s = ANSI_OSC_RE.sub("", s)
        s = ANSI_CSI_RE.sub("", s)
        s = ANSI_ESC_RE.sub("", s)
        cleaned = []
        for ch in s:
            o = ord(ch)
            if ch == "\t":
                cleaned.append("\t")
                continue
            if o < 32 or 127 <= o <= 159:
                continue
            cleaned.append(ch)
        lines.append("".join(cleaned).strip("\n"))
    return proc.returncode, "\n".join(lines)


def normalize_status(raw: str) -> str:
    if not raw:
        return "?"
    s = raw[0].upper()
    if s in ("A", "M", "D", "R", "C", "T"):
        return s
    return s


def detect_language(path: str) -> str:
    name = Path(path).name.lower()
    if name in ("makefile",):
        return "make"
    if name.endswith(".rs"):
        return "rust"
    if name.endswith(".py"):
        return "python"
    if name.endswith(".sh"):
        return "shell"
    if name.endswith(".toml"):
        return "toml"
    if name.endswith(".c") or name.endswith(".h"):
        return "c"
    if name.endswith(".ld"):
        return "ld"
    return "plain"


def lang_keywords(lang: str) -> set[str]:
    if lang == "rust":
        return {
            "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn",
            "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
            "return", "self", "Self", "static", "struct", "trait", "true", "type", "unsafe", "use",
            "where", "while", "async", "await", "dyn",
        }
    if lang == "python":
        return {
            "def", "class", "if", "elif", "else", "for", "while", "try", "except", "finally", "return",
            "import", "from", "as", "with", "pass", "break", "continue", "True", "False", "None",
        }
    if lang == "shell":
        return {"if", "then", "fi", "for", "in", "do", "done", "case", "esac", "function", "return"}
    if lang == "toml":
        return {"true", "false"}
    if lang == "c":
        return {
            "int", "long", "short", "char", "void", "struct", "enum", "typedef", "return", "if", "else",
            "for", "while", "switch", "case", "break", "continue", "const", "static", "extern",
        }
    return set()


def line_comment_token(lang: str) -> Optional[str]:
    if lang in ("rust", "c", "ld"):
        return "//"
    if lang in ("python", "shell", "make", "toml"):
        return "#"
    return None


def parse_name_status(output: str) -> List[SourceFileEntry]:
    out: List[SourceFileEntry] = []
    for ln in output.splitlines():
        if not ln.strip():
            continue
        parts = ln.split("\t")
        if not parts:
            continue
        status = normalize_status(parts[0])
        if status in ("R", "C") and len(parts) >= 3:
            out.append(SourceFileEntry(path=parts[2], status=status, old_path=parts[1]))
            continue
        if len(parts) >= 2:
            out.append(SourceFileEntry(path=parts[1], status=status))
    return out


def parse_added_hunk_lines(diff_text: str) -> List[int]:
    lines: List[int] = []
    for ln in diff_text.splitlines():
        m = HUNK_RE.match(ln)
        if not m:
            continue
        start = int(m.group(1))
        count = int(m.group(2)) if m.group(2) else 1
        if count <= 0:
            continue
        lines.extend(range(start, start + count))
    return lines


def changed_line_kind(
    host: str,
    repo: str,
    commit: str,
    prev: str,
    file_entry: SourceFileEntry,
) -> dict[int, str]:
    if file_entry.status == "D":
        return {}

    if file_entry.status == "A":
        rc, out = run_capture(
            host,
            f"cd {shlex.quote(repo)} && git show --format= --numstat {shlex.quote(commit)} -- {shlex.quote(file_entry.path)}",
        )
        if rc != 0:
            return {}
        line_count = 0
        for ln in out.splitlines():
            cols = ln.split("\t")
            if len(cols) >= 2 and cols[0].isdigit():
                line_count = int(cols[0])
                break
        if line_count <= 0:
            return {}
        return {i: "add" for i in range(1, line_count + 1)}

    if prev:
        cmd = (
            f"cd {shlex.quote(repo)} && "
            f"git diff --unified=0 --find-renames {shlex.quote(prev)} {shlex.quote(commit)} -- {shlex.quote(file_entry.path)}"
        )
    else:
        cmd = f"cd {shlex.quote(repo)} && git show --unified=0 --format= {shlex.quote(commit)} -- {shlex.quote(file_entry.path)}"
    rc, out = run_capture(host, cmd)
    if rc != 0:
        return {}

    tag = "add" if file_entry.status == "A" else "mod"
    return {i: tag for i in parse_added_hunk_lines(out)}


def jump_search(state: AppState, direction: int, visible: int) -> None:
    if not state.search_hits:
        state.status_line = "No search match"
        return

    if state.search_pos < 0:
        state.search_pos = 0 if direction >= 0 else len(state.search_hits) - 1
    else:
        n = len(state.search_hits)
        state.search_pos = (state.search_pos + direction) % n

    hit = state.search_hits[state.search_pos]
    total = len(state.log_lines)
    max_start = max(0, total - visible)
    state.log_follow = False
    state.log_anchor = max(0, min(max_start, hit - max(0, visible // 2)))
    state.status_line = f"Search {state.search_pos + 1}/{len(state.search_hits)}: {state.log_search}"


def prompt_input(stdscr: curses.window, prompt: str, initial: str = "") -> Optional[str]:
    h, w = stdscr.getmaxyx()
    y = max(0, h - 1)
    x = 0
    max_len = max(1, w - len(prompt) - 2)

    old_nodelay = stdscr.nodelay(False)
    try:
        curses.curs_set(1)
    except curses.error:
        pass

    curses.echo()
    try:
        stdscr.move(y, x)
        stdscr.clrtoeol()
        stdscr.addnstr(y, x, prompt, w - 1)
        if initial:
            stdscr.addnstr(y, len(prompt), initial[:max_len], max_len)
            stdscr.move(y, min(w - 1, len(prompt) + len(initial[:max_len])))
        stdscr.refresh()
        raw = stdscr.getstr(y, len(prompt), max_len)
        return raw.decode(errors="ignore")
    except KeyboardInterrupt:
        return None
    finally:
        curses.noecho()
        try:
            curses.curs_set(0)
        except curses.error:
            pass
        stdscr.nodelay(old_nodelay)


def adjust_log_scroll(state: AppState, delta: int, visible: int) -> None:
    total = len(state.log_lines)
    if total <= 0:
        state.log_follow = True
        state.log_anchor = 0
        return

    max_start = max(0, total - visible)
    if state.log_follow:
        state.log_anchor = max_start
        state.log_follow = False

    state.log_anchor = max(0, min(max_start, state.log_anchor + delta))
    if state.log_anchor >= max_start:
        state.log_follow = True


def scroll_log_top(state: AppState) -> None:
    state.log_follow = False
    state.log_anchor = 0


def scroll_log_bottom(state: AppState) -> None:
    state.log_follow = True
    state.log_anchor = max(0, len(state.log_lines) - 1)


def run_streaming(
    host: str,
    shell_cmd: str,
    state: AppState,
    stdscr: curses.window,
) -> Tuple[int, str]:
    argv = host_exec_argv(host, shell_cmd)
    raw_cmd = " ".join(shlex.quote(a) for a in argv)
    # Multiline ssh/bash payloads must be flattened; embedded newlines break box rendering.
    one_line_cmd = sanitize_log_line(raw_cmd.replace("\n", " | "))
    state.current_cmd = one_line_cmd
    append_log(state, f"$ {one_line_cmd}")
    draw_ui(stdscr, state)

    proc = subprocess.Popen(
        argv,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        bufsize=1,
    )

    output_accum: List[str] = []

    while True:
        ch = stdscr.getch()
        if ch in (ord("s"), ord("S")):
            state.stop_requested = True
            if proc.poll() is None:
                proc.terminate()
                append_log(state, "[stop requested] terminating current command...")
        elif ch in (curses.KEY_PPAGE,):
            adjust_log_scroll(state, -8, max(1, stdscr.getmaxyx()[0] - 8))
        elif ch in (curses.KEY_NPAGE,):
            adjust_log_scroll(state, 8, max(1, stdscr.getmaxyx()[0] - 8))
        elif ch in (curses.KEY_HOME, ord("g")):
            scroll_log_top(state)
        elif ch in (curses.KEY_END, ord("G")):
            scroll_log_bottom(state)
        elif ch == ord("n"):
            jump_search(state, +1, max(1, stdscr.getmaxyx()[0] - 8))
        elif ch == ord("N"):
            jump_search(state, -1, max(1, stdscr.getmaxyx()[0] - 8))

        line = proc.stdout.readline() if proc.stdout else ""
        if line:
            line = sanitize_log_line(line.rstrip("\n"))
            output_accum.append(line)
            append_log(state, line)

        if proc.poll() is not None:
            if proc.stdout:
                tail = proc.stdout.read()
                if tail:
                    for raw in tail.splitlines():
                        line = sanitize_log_line(raw)
                        output_accum.append(line)
                        append_log(state, line)
            break

        draw_ui(stdscr, state)
        time.sleep(0.03)

    rc = proc.returncode or 0
    combined = "\n".join(output_accum)
    return rc, combined


def commit_exists(host: str, repo: str, commit: str, state: AppState, stdscr: Optional[curses.window]) -> bool:
    cmd = f"cd {shlex.quote(repo)} && git cat-file -e {shlex.quote(commit)}^{{commit}}"
    if stdscr is None:
        rc, _ = run_capture(host, cmd)
    else:
        rc, _ = run_streaming(host, cmd, state, stdscr)
    return rc == 0


def resolve_commit(host: str, repo: str, chapter: ChapterSet, state: AppState, stdscr: Optional[curses.window]) -> Optional[str]:
    cached = state.resolved_commits.get(chapter.key)
    if cached:
        return cached

    for cand in chapter.commit_candidates:
        state.current_action = f"resolving commit for {chapter.label}: {cand}"
        if stdscr is not None:
            draw_ui(stdscr, state)
        if commit_exists(host, repo, cand, state, stdscr):
            state.resolved_commits[chapter.key] = cand
            return cand
    return None


def prev_chapter_commit(
    host: str,
    repo: str,
    chapter_idx: int,
    state: AppState,
    stdscr: curses.window,
) -> Optional[str]:
    if chapter_idx <= 0:
        return None
    prev = CHAPTERS[chapter_idx - 1]
    return resolve_commit(host, repo, prev, state, stdscr)


def run_inspect_command(
    host: str,
    repo: str,
    chapter: ChapterSet,
    chapter_idx: int,
    mode: str,
    state: AppState,
    stdscr: curses.window,
) -> None:
    commit = resolve_commit(host, repo, chapter, state, stdscr)
    if not commit:
        state.status_line = f"Cannot resolve commit for {chapter.label}"
        return

    prev = prev_chapter_commit(host, repo, chapter_idx, state, stdscr)
    range_note = f"{prev}..{commit}" if prev else commit
    append_log(state, "")
    append_log(state, f"===== Inspect {chapter.label} ({mode}) =====")
    append_log(state, f"commit: {commit}")
    if prev:
        append_log(state, f"range : {prev}..{commit}")
    else:
        append_log(state, "range : (first chapter baseline)")

    if mode == "history":
        if prev:
            inspect_cmd = textwrap.dedent(
                f"""
                set -e
                cd {shlex.quote(repo)}
                echo '[chapter commit]'
                git show --no-patch --decorate --pretty=fuller {shlex.quote(commit)}
                echo
                echo '[commits between previous chapter and current]'
                git log --oneline --decorate --reverse {shlex.quote(prev)}..{shlex.quote(commit)}
                """
            ).strip()
        else:
            inspect_cmd = textwrap.dedent(
                f"""
                set -e
                cd {shlex.quote(repo)}
                echo '[chapter commit]'
                git show --no-patch --decorate --pretty=fuller {shlex.quote(commit)}
                echo
                echo '[nearby history around chapter commit]'
                git log --oneline --decorate -n 20 {shlex.quote(commit)}
                """
            ).strip()
    else:
        if prev:
            inspect_cmd = textwrap.dedent(
                f"""
                set -e
                cd {shlex.quote(repo)}
                echo '[range changed files]'
                git diff --name-status {shlex.quote(prev)}..{shlex.quote(commit)}
                echo
                echo '[range diff stat]'
                git diff --stat {shlex.quote(prev)}..{shlex.quote(commit)}
                echo
                echo '[chapter commit patch]'
                git show --patch --stat --find-renames --color=never {shlex.quote(commit)}
                """
            ).strip()
        else:
            inspect_cmd = textwrap.dedent(
                f"""
                set -e
                cd {shlex.quote(repo)}
                echo '[chapter commit changed files]'
                git show --name-status --format= {shlex.quote(commit)}
                echo
                echo '[chapter commit patch]'
                git show --patch --stat --find-renames --color=never {shlex.quote(commit)}
                """
            ).strip()

    state.current_action = f"inspect {mode}: {chapter.label} [{range_note}]"
    rc, _ = run_streaming(host, inspect_cmd, state, stdscr)
    if rc == 0:
        state.status_line = f"Inspect {mode} complete: {chapter.label}"
    else:
        state.status_line = f"Inspect {mode} failed (rc={rc}): {chapter.label}"


def list_changed_files(
    host: str,
    repo: str,
    commit: str,
    prev: str,
) -> List[SourceFileEntry]:
    if prev:
        cmd = f"cd {shlex.quote(repo)} && git diff --name-status --find-renames {shlex.quote(prev)}..{shlex.quote(commit)}"
    else:
        cmd = f"cd {shlex.quote(repo)} && git show --name-status --format= {shlex.quote(commit)}"
    rc, out = run_capture(host, cmd)
    if rc != 0:
        return []
    return parse_name_status(out)


def load_source_file(
    host: str,
    repo: str,
    commit: str,
    prev: str,
    file_entry: SourceFileEntry,
) -> Tuple[List[str], dict[int, str], str, str]:
    if file_entry.status == "D":
        msg = [f"[deleted in {commit}] {file_entry.path}"]
        if file_entry.old_path:
            msg.append(f"(from: {file_entry.old_path})")
        return msg, {}, "plain", "deleted file"

    cmd = f"cd {shlex.quote(repo)} && git show {shlex.quote(commit)}:{shlex.quote(file_entry.path)}"
    rc, out = run_capture(host, cmd)
    if rc != 0:
        return [f"[failed to open file at {commit}] {file_entry.path}", out], {}, "plain", "open failed"

    lines = out.splitlines()
    line_kind = changed_line_kind(host, repo, commit, prev, file_entry)
    lang = detect_language(file_entry.path)
    note = ""
    return lines, line_kind, lang, note


def open_source_view_for_selected(state: AppState, stdscr: curses.window) -> None:
    host = HOSTS[state.host_idx]
    repo = repo_path_for_host(host)
    idx = state.selected_idx
    chapter = CHAPTERS[idx]
    commit = resolve_commit(host, repo, chapter, state, stdscr)
    if not commit:
        state.status_line = f"Cannot resolve commit for {chapter.label}"
        return

    prev = prev_chapter_commit(host, repo, idx, state, stdscr) or ""
    files = list_changed_files(host, repo, commit, prev)
    if not files:
        state.status_line = f"No changed files found for {chapter.label}"
        return

    sv = SourceViewerState(
        active=True,
        chapter_label=chapter.label,
        commit=commit,
        prev_commit=prev,
        files=files,
        file_idx=0,
        scroll=0,
        note="",
    )
    state.source_view = sv
    load_current_source_file(state)
    state.status_line = f"Source view: {chapter.label} ({len(files)} files)"


def load_current_source_file(state: AppState) -> None:
    sv = state.source_view
    if not sv.files:
        sv.content = ["[no files]"]
        sv.line_kind = {}
        sv.language = "plain"
        sv.note = ""
        return

    entry = sv.files[sv.file_idx]
    cache_key = f"{sv.commit}:{entry.path}:{sv.prev_commit}"
    cached = sv.cache.get(cache_key)
    if cached:
        sv.content, sv.line_kind, sv.language, sv.note = cached
        return

    host = HOSTS[state.host_idx]
    repo = repo_path_for_host(host)
    content, line_kind, lang, note = load_source_file(host, repo, sv.commit, sv.prev_commit, entry)
    sv.content = content if content else [""]
    sv.line_kind = line_kind
    sv.language = lang
    sv.note = note
    sv.cache[cache_key] = (sv.content, sv.line_kind, sv.language, sv.note)


def source_scroll(state: AppState, delta: int, visible: int) -> None:
    sv = state.source_view
    total = len(sv.content)
    if total <= 0:
        sv.scroll = 0
        return
    max_start = max(0, total - visible)
    sv.scroll = max(0, min(max_start, sv.scroll + delta))


def source_go_file(state: AppState, delta: int) -> None:
    sv = state.source_view
    if not sv.files:
        return
    sv.file_idx = (sv.file_idx + delta) % len(sv.files)
    sv.scroll = 0
    load_current_source_file(state)

def wrap_checkout_command(repo: str, commit: str, inner_cmd: str) -> str:
    container_cmd = (
        "podman run --rm "
        "-v \"$PWD\":/workspace/mmos-practice "
        "-w /workspace/mmos-practice "
        "mmos-practice:latest "
        "/bin/bash -lc "
        f"{shlex.quote(inner_cmd)}"
    )
    return textwrap.dedent(
        f"""
        set -e
        cd {shlex.quote(repo)}
        if [ "${{FORCE_REBUILD_IMAGE:-0}}" = "1" ]; then
          podman build -t mmos-practice:latest .
        elif ! podman image exists mmos-practice:latest >/dev/null 2>&1; then
          podman build -t mmos-practice:latest .
        fi
        if [ ! -d external/u-boot ] || [ ! -d external/opensbi ]; then
          ./scripts/init-submodules.sh
        fi
        orig_ref=$(git rev-parse --abbrev-ref HEAD)
        if [ "$orig_ref" = "HEAD" ]; then
          orig_ref=$(git rev-parse HEAD)
        fi
        cleanup() {{
          git checkout -q "$orig_ref" >/dev/null 2>&1 || true
        }}
        trap cleanup EXIT
        git checkout -q {shlex.quote(commit)}
        {container_cmd}
        """
    ).strip()


def preflight_prepare(host: str, repo: str, state: AppState, stdscr: curses.window) -> bool:
    cmds = [
        "set -e",
        f"cd {shlex.quote(repo)}",
        "test -d .git",
    ]

    if state.sync_before_run:
        cmds.extend(
            [
                "git fetch origin --prune",
                "if git show-ref --verify --quiet refs/heads/main; then git checkout -q main; fi",
                "git pull --ff-only origin main || true",
            ]
        )

    if state.rebuild_image_before_run:
        cmds.append("podman build -t mmos-practice:latest .")
    else:
        cmds.append(
            "if ! podman image exists mmos-practice:latest >/dev/null 2>&1; then podman build -t mmos-practice:latest .; fi"
        )

    state.current_action = "preflight: sync/build image"
    rc, _ = run_streaming(host, "\n".join(cmds), state, stdscr)
    if rc != 0:
        state.status_line = f"Preflight failed (rc={rc})"
        return False
    return True


def run_selected(stdscr: curses.window, state: AppState, chapters: Sequence[ChapterSet]) -> None:
    selected = [ch for i, ch in enumerate(chapters) if state.checked[i]]
    if not selected:
        state.status_line = "No chapter selected"
        return

    host = HOSTS[state.host_idx]
    repo = repo_path_for_host(host)
    state.running = True
    state.stop_requested = False
    state.status_line = (
        f"Running {len(selected)} chapter set(s) on {host}: {repo} "
        f"[CONSOLE={selected_console(state)} BOOT_CHAIN={selected_boot_chain(state)}]"
    )
    state.resolved_commits.clear()

    if not preflight_prepare(host, repo, state, stdscr):
        state.running = False
        state.current_action = "idle"
        state.current_cmd = ""
        return

    for chapter in selected:
        if state.stop_requested:
            break

        state.current_action = f"chapter: {chapter.label}"
        draw_ui(stdscr, state)

        commit = resolve_commit(host, repo, chapter, state, stdscr)
        if not commit:
            state.results.append(
                StepResult(
                    chapter_key=chapter.key,
                    chapter_label=chapter.label,
                    step_name="resolve-commit",
                    status="FAIL",
                    elapsed_sec=0.0,
                    commit="N/A",
                    cmd="resolve commit",
                    detail="No candidate commit exists on target repository",
                )
            )
            continue

        chapter_failed = False
        for step in chapter.steps:
            if chapter_failed or state.stop_requested:
                state.results.append(
                    StepResult(
                        chapter_key=chapter.key,
                        chapter_label=chapter.label,
                        step_name=step.name,
                        status="SKIP",
                        elapsed_sec=0.0,
                        commit=commit,
                        cmd=step.cmd,
                        detail="Skipped due to previous failure or stop request",
                    )
                )
                continue

            state.current_action = f"{chapter.label} [{commit}] -> {step.name}"
            step_cmd = apply_build_options(step.cmd, state)
            step_shell = wrap_checkout_command(repo, commit, step_cmd)
            start = time.time()
            rc, out = run_streaming(host, step_shell, state, stdscr)
            elapsed = time.time() - start

            status = "PASS"
            detail = ""
            if rc != 0:
                if step.allow_failure and step.expected_pattern and step.expected_pattern in out:
                    status = "WARN"
                    detail = f"Expected historical failure matched: {step.expected_pattern}"
                    chapter_failed = True
                else:
                    status = "FAIL"
                    detail = f"Exit code: {rc}"
                    chapter_failed = True
            elif step.must_contain:
                found, missing = check_patterns(out, step.must_contain)
                if found:
                    append_log(state, f"[signature:found] {', '.join(found)}")
                if missing:
                    append_log(state, f"[signature:missing] {', '.join(missing)}")
                    status = "WARN"
                    detail = f"Missing expected runtime signature: {', '.join(missing)}"

            state.results.append(
                StepResult(
                    chapter_key=chapter.key,
                    chapter_label=chapter.label,
                    step_name=step.name,
                    status=status,
                    elapsed_sec=elapsed,
                    commit=commit,
                    cmd=step_cmd,
                    detail=detail,
                )
            )

    state.running = False
    state.current_action = "idle"
    state.current_cmd = ""
    if state.stop_requested:
        state.status_line = "Stopped by user"
    else:
        state.status_line = "Run complete"


def draw_box(win: curses.window, title: str) -> None:
    win.erase()
    h, w = win.getmaxyx()
    win.box()
    title_txt = f" {title} "
    if w > len(title_txt) + 2:
        try:
            win.addstr(0, 2, title_txt)
        except curses.error:
            pass


def init_theme() -> UiTheme:
    if not curses.has_colors():
        return UiTheme()

    curses.start_color()
    if hasattr(curses, "use_default_colors"):
        curses.use_default_colors()

    # Keep pair ids explicit and stable.
    curses.init_pair(1, curses.COLOR_BLACK, curses.COLOR_CYAN)     # header
    curses.init_pair(2, curses.COLOR_CYAN, -1)                     # panel title
    curses.init_pair(3, curses.COLOR_BLACK, curses.COLOR_YELLOW)   # selected list item
    curses.init_pair(4, curses.COLOR_GREEN, -1)                    # running state / pass
    curses.init_pair(5, curses.COLOR_WHITE, -1)                    # idle / info
    curses.init_pair(6, curses.COLOR_RED, -1)                      # fail
    curses.init_pair(7, curses.COLOR_YELLOW, -1)                   # warn
    curses.init_pair(8, curses.COLOR_MAGENTA, -1)                  # skip
    curses.init_pair(9, curses.COLOR_BLUE, curses.COLOR_WHITE)     # search hit
    curses.init_pair(10, curses.COLOR_GREEN, -1)                   # file added / line added
    curses.init_pair(11, curses.COLOR_YELLOW, -1)                  # file modified / line modified
    curses.init_pair(12, curses.COLOR_RED, -1)                     # file deleted
    curses.init_pair(13, curses.COLOR_BLUE, -1)                    # line no
    curses.init_pair(14, curses.COLOR_CYAN, -1)                    # keyword
    curses.init_pair(15, curses.COLOR_MAGENTA, -1)                 # string
    curses.init_pair(16, curses.COLOR_GREEN, -1)                   # comment
    curses.init_pair(17, curses.COLOR_YELLOW, -1)                  # number
    if curses.COLORS >= 256:
        # High-contrast palettes for readability.
        add_bg = 24   # deep blue
        mod_bg = 178  # warm yellow
        curses.init_pair(18, 231, add_bg)                          # added line base (white on blue)
        curses.init_pair(19, 16, mod_bg)                           # modified line base (black on yellow)
        curses.init_pair(20, 226, add_bg)                          # keyword on added line
        curses.init_pair(21, 255, add_bg)                          # string on added line
        curses.init_pair(22, 159, add_bg)                          # comment on added line
        curses.init_pair(23, 213, add_bg)                          # number on added line
        curses.init_pair(24, 19, mod_bg)                           # keyword on modified line
        curses.init_pair(25, 88, mod_bg)                           # string on modified line
        curses.init_pair(26, 22, mod_bg)                           # comment on modified line
        curses.init_pair(27, 52, mod_bg)                           # number on modified line
    else:
        # Fallback for 8/16-color terminals: prioritize contrast.
        curses.init_pair(18, curses.COLOR_WHITE, curses.COLOR_BLUE)    # added line background
        curses.init_pair(19, curses.COLOR_BLACK, curses.COLOR_YELLOW)  # modified line background
        curses.init_pair(20, curses.COLOR_YELLOW, curses.COLOR_BLUE)    # keyword on added line
        curses.init_pair(21, curses.COLOR_WHITE, curses.COLOR_BLUE)     # string on added line
        curses.init_pair(22, curses.COLOR_CYAN, curses.COLOR_BLUE)      # comment on added line
        curses.init_pair(23, curses.COLOR_MAGENTA, curses.COLOR_BLUE)   # number on added line
        curses.init_pair(24, curses.COLOR_BLUE, curses.COLOR_YELLOW)    # keyword on modified line
        curses.init_pair(25, curses.COLOR_RED, curses.COLOR_YELLOW)     # string on modified line
        curses.init_pair(26, curses.COLOR_GREEN, curses.COLOR_YELLOW)   # comment on modified line
        curses.init_pair(27, curses.COLOR_MAGENTA, curses.COLOR_YELLOW) # number on modified line

    return UiTheme(
        header=curses.color_pair(1) | curses.A_BOLD,
        panel_title=curses.color_pair(2) | curses.A_BOLD,
        selected_item=curses.color_pair(3) | curses.A_BOLD,
        running_state=curses.color_pair(4) | curses.A_BOLD,
        idle_state=curses.color_pair(5),
        pass_text=curses.color_pair(4) | curses.A_BOLD,
        fail_text=curses.color_pair(6) | curses.A_BOLD,
        warn_text=curses.color_pair(7) | curses.A_BOLD,
        skip_text=curses.color_pair(8) | curses.A_BOLD,
        info_text=curses.color_pair(5),
        search_hit=curses.color_pair(9) | curses.A_BOLD,
        file_added=curses.color_pair(10) | curses.A_BOLD,
        file_modified=curses.color_pair(11) | curses.A_BOLD,
        file_deleted=curses.color_pair(12) | curses.A_BOLD,
        line_no=curses.color_pair(13),
        line_added=curses.color_pair(10) | curses.A_BOLD,
        line_modified=curses.color_pair(11) | curses.A_BOLD,
        line_added_bg=curses.color_pair(18),
        line_modified_bg=curses.color_pair(19),
        syn_keyword=curses.color_pair(14) | curses.A_BOLD,
        syn_string=curses.color_pair(15),
        syn_comment=curses.color_pair(16),
        syn_number=curses.color_pair(17),
        syn_keyword_add_bg=curses.color_pair(20) | curses.A_BOLD,
        syn_string_add_bg=curses.color_pair(21),
        syn_comment_add_bg=curses.color_pair(22),
        syn_number_add_bg=curses.color_pair(23),
        syn_keyword_mod_bg=curses.color_pair(24) | curses.A_BOLD,
        syn_string_mod_bg=curses.color_pair(25),
        syn_comment_mod_bg=curses.color_pair(26),
        syn_number_mod_bg=curses.color_pair(27),
    )


def render_wrapped(win: curses.window, y: int, x: int, text: str, width: int, max_lines: int) -> int:
    lines = textwrap.wrap(text, width=max(8, width)) or [""]
    used = 0
    for ln in lines[:max_lines]:
        try:
            win.addnstr(y + used, x, ln, width)
        except curses.error:
            pass
        used += 1
    return used


def summarize_results(results: Sequence[StepResult]) -> Tuple[int, int, int, int]:
    p = sum(1 for r in results if r.status == "PASS")
    f = sum(1 for r in results if r.status == "FAIL")
    w = sum(1 for r in results if r.status == "WARN")
    s = sum(1 for r in results if r.status == "SKIP")
    return p, f, w, s


def comment_index(line: str, lang: str) -> int:
    token = line_comment_token(lang)
    if not token:
        return -1
    return line.find(token)


def render_code_line(
    win: curses.window,
    y: int,
    x: int,
    width: int,
    line: str,
    lang: str,
    theme: UiTheme,
    base_attr: int = curses.A_NORMAL,
    line_kind: str = "",
) -> None:
    if width <= 0:
        return

    # Paint full line width first so changed lines keep a uniform background.
    try:
        win.addnstr(y, x, " " * width, width, base_attr)
    except curses.error:
        return

    txt = line[:width]
    try:
        win.addnstr(y, x, txt, width, base_attr)
    except curses.error:
        return

    if lang == "plain":
        return

    cidx = comment_index(txt, lang)
    comment_cut = len(txt) if cidx < 0 else cidx

    kw_attr = theme.syn_keyword
    str_attr = theme.syn_string
    cmt_attr = theme.syn_comment
    num_attr = theme.syn_number
    if line_kind == "add":
        kw_attr = theme.syn_keyword_add_bg
        str_attr = theme.syn_string_add_bg
        cmt_attr = theme.syn_comment_add_bg
        num_attr = theme.syn_number_add_bg
    elif line_kind == "mod":
        kw_attr = theme.syn_keyword_mod_bg
        str_attr = theme.syn_string_mod_bg
        cmt_attr = theme.syn_comment_mod_bg
        num_attr = theme.syn_number_mod_bg

    kw = lang_keywords(lang)
    if kw:
        for m in re.finditer(r"\b[A-Za-z_][A-Za-z0-9_]*\b", txt[:comment_cut]):
            token = m.group(0)
            if token in kw:
                try:
                    win.addnstr(
                        y,
                        x + m.start(),
                        token,
                        min(len(token), width - m.start()),
                        kw_attr,
                    )
                except curses.error:
                    pass

    for m in NUMBER_RE.finditer(txt[:comment_cut]):
        num = m.group(0)
        try:
            win.addnstr(y, x + m.start(), num, min(len(num), width - m.start()), num_attr)
        except curses.error:
            pass

    for m in STRING_RE.finditer(txt[:comment_cut]):
        sval = m.group(0)
        try:
            win.addnstr(y, x + m.start(), sval, min(len(sval), width - m.start()), str_attr)
        except curses.error:
            pass

    if cidx >= 0 and cidx < width:
        cmt = txt[cidx:]
        try:
            win.addnstr(y, x + cidx, cmt, min(len(cmt), width - cidx), cmt_attr)
        except curses.error:
            pass


def file_status_attr(theme: UiTheme, st: str) -> int:
    if st == "A":
        return theme.file_added
    if st in ("M", "R", "C", "T"):
        return theme.file_modified
    if st == "D":
        return theme.file_deleted
    return curses.A_NORMAL


def draw_source_ui(stdscr: curses.window, state: AppState) -> None:
    stdscr.erase()
    h, w = stdscr.getmaxyx()
    if h < 24 or w < 100:
        msg = "Terminal too small. Need at least 100x24. Resize and retry."
        stdscr.addnstr(0, 0, msg, max(1, w - 1))
        stdscr.refresh()
        return

    sv = state.source_view
    header_h = 3
    body_h = h - header_h
    left_w = max(17, int(w * 0.17))
    right_w = w - left_w

    header = stdscr.derwin(header_h, w, 0, 0)
    files = stdscr.derwin(body_h, left_w, header_h, 0)
    src = stdscr.derwin(body_h, right_w, header_h, left_w)

    draw_box(header, "MMOS Practice Source Viewer")
    try:
        header.bkgd(" ", state.theme.header)
    except curses.error:
        pass
    hk = "[Left/Right] file  [Up/Down] line  [PgUp/PgDn] page  [Home/End] top/bottom  [B/Esc] back"
    header.addnstr(1, 2, hk, w - 4)

    draw_box(files, "Changed Files")
    files.addnstr(0, 2, " Changed Files ", max(1, left_w - 4), state.theme.panel_title)
    file_h = body_h - 2
    fstart = 0
    if sv.file_idx >= file_h:
        fstart = sv.file_idx - file_h + 1
    for row, idx in enumerate(range(fstart, min(len(sv.files), fstart + file_h))):
        ent = sv.files[idx]
        prefix = ">" if idx == sv.file_idx else " "
        label = f"{prefix} [{ent.status}] {ent.path}"
        attr = file_status_attr(state.theme, ent.status)
        if idx == sv.file_idx:
            attr |= curses.A_REVERSE
        files.addnstr(1 + row, 1, fit_line(label, left_w - 2), left_w - 2, attr)

    ent = sv.files[sv.file_idx] if sv.files else SourceFileEntry(path="(none)", status="?")
    subtitle = f"{sv.chapter_label} | {sv.prev_commit or 'BASE'}..{sv.commit}"
    draw_box(src, "Source")
    src.addnstr(0, 2, fit_line(f" {ent.path} ", right_w - 4), max(1, right_w - 4), state.theme.panel_title)
    src.addnstr(1, 2, fit_line(subtitle, right_w - 4), right_w - 4, state.theme.info_text)
    if sv.note:
        src.addnstr(2, 2, fit_line(sv.note, right_w - 4), right_w - 4, state.theme.warn_text)

    content_y = 3
    visible = max(1, body_h - content_y - 1)
    total = len(sv.content)
    max_start = max(0, total - visible)
    sv.scroll = max(0, min(max_start, sv.scroll))
    line_no_w = max(4, len(str(max(1, total))))
    marker_w = 2
    code_w = max(1, right_w - (2 + line_no_w + marker_w + 1))

    blank = " " * max(1, right_w - 2)
    for i in range(visible):
        src.addnstr(content_y + i, 1, blank, max(1, right_w - 2))

    for i in range(visible):
        idx = sv.scroll + i
        if idx >= total:
            break
        ln = sv.content[idx]
        lno = idx + 1
        kind = sv.line_kind.get(lno, "")
        marker = "+"
        mark_attr = state.theme.line_added
        code_attr = state.theme.line_added_bg
        if kind != "add":
            marker = "~"
            mark_attr = state.theme.line_modified
            code_attr = state.theme.line_modified_bg
        if not kind:
            marker = " "
            mark_attr = curses.A_NORMAL
            code_attr = curses.A_NORMAL

        y = content_y + i
        src.addnstr(y, 1, str(lno).rjust(line_no_w), line_no_w, state.theme.line_no)
        src.addnstr(y, 1 + line_no_w, " ", 1)
        src.addnstr(y, 2 + line_no_w, marker, 1, mark_attr)
        src.addnstr(y, 3 + line_no_w, " ", 1)
        render_code_line(src, y, 4 + line_no_w, code_w, ln, sv.language, state.theme, code_attr, kind)

    stdscr.refresh()


def draw_ui(stdscr: curses.window, state: AppState) -> None:
    stdscr.erase()
    h, w = stdscr.getmaxyx()

    if h < 24 or w < 100:
        msg = "Terminal too small. Need at least 100x24. Resize and retry."
        stdscr.addnstr(0, 0, msg, max(1, w - 1))
        stdscr.refresh()
        return

    if state.source_view.active:
        draw_source_ui(stdscr, state)
        return

    host = HOSTS[state.host_idx]
    repo = repo_path_for_host(host)

    header_h = 3
    body_h = h - header_h
    top_h = max(10, int(body_h * 0.52))
    bottom_h = body_h - top_h
    left_w = max(21, int(w * 0.24))
    right_w = w - left_w

    header = stdscr.derwin(header_h, w, 0, 0)
    left = stdscr.derwin(top_h, left_w, header_h, 0)
    right = stdscr.derwin(top_h, right_w, header_h, left_w)
    logs = stdscr.derwin(bottom_h, w, header_h + top_h, 0)

    draw_box(header, "MMOS Practice Verifier")
    try:
        header.bkgd(" ", state.theme.header)
    except curses.error:
        pass
    hotkeys = "[Tab/Shift-Tab] focus  [Enter/Space] select/toggle  [Arrows/PgUp/PgDn] navigate  [/] search  [n/N] next/prev  [Q] quit"
    header.addnstr(1, 2, hotkeys, w - 4)

    draw_box(left, "Chapter Sets")
    left_title_attr = state.theme.panel_title | (curses.A_REVERSE if state.focus_idx == 0 else 0)
    left.addnstr(0, 2, " Chapter Sets ", left_w - 4, left_title_attr)
    max_items = top_h - 2
    start = 0
    if state.selected_idx >= max_items:
        start = state.selected_idx - max_items + 1

    for row, idx in enumerate(range(start, min(len(CHAPTERS), start + max_items))):
        ch = CHAPTERS[idx]
        mark = "x" if state.checked[idx] else " "
        cursor = ">" if idx == state.selected_idx else " "
        line = f"{cursor} [{mark}] {ch.label}"
        attr = state.theme.selected_item if idx == state.selected_idx else curses.A_NORMAL
        if idx == state.selected_idx and state.focus_idx == 0:
            attr |= curses.A_REVERSE
        left.addnstr(1 + row, 1, line, left_w - 2, attr)

    draw_box(right, "Menu / Status")
    right_title_attr = state.theme.panel_title | (curses.A_REVERSE if state.focus_idx == 1 else 0)
    right.addnstr(0, 2, " Menu / Status ", right_w - 4, right_title_attr)
    p, f, wcnt, s = summarize_results(state.results)
    selected = CHAPTERS[state.selected_idx]
    cached = state.resolved_commits.get(selected.key, "(not resolved)")
    menu = menu_items(state)
    menu_h = min(len(menu), max(4, top_h // 2))
    menu_start = 0
    if state.menu_idx >= menu_h:
        menu_start = state.menu_idx - menu_h + 1
    for row, idx in enumerate(range(menu_start, min(len(menu), menu_start + menu_h))):
        name, val = menu[idx]
        prefix = ">" if idx == state.menu_idx else " "
        line = fit_line(f"{prefix} {name:<14} : {val}", right_w - 2)
        attr = state.theme.selected_item if idx == state.menu_idx else curses.A_NORMAL
        if idx == state.menu_idx and state.focus_idx == 1:
            attr |= curses.A_REVERSE
        right.addnstr(1 + row, 1, line, right_w - 2, attr)

    def right_put(y: int, x: int, txt: str, width: int, attr: int = curses.A_NORMAL) -> None:
        if 0 <= y < top_h - 1:
            right.addnstr(y, x, txt, width, attr)

    y = 1 + menu_h + 1
    right_put(y, 2, f"Repo: {repo}", right_w - 4)
    y += 1
    right_put(
        y,
        2,
        f"State: {'RUNNING' if state.running else 'IDLE'}",
        right_w - 4,
        state.theme.running_state if state.running else state.theme.idle_state,
    )
    y += 1
    right_put(y, 2, "Summary:", right_w - 4)
    right_put(y, 11, f"PASS={p}", right_w - 13, state.theme.pass_text)
    right_put(y, 20, f"FAIL={f}", right_w - 22, state.theme.fail_text)
    right_put(y, 29, f"WARN={wcnt}", right_w - 31, state.theme.warn_text)
    right_put(y, 40, f"SKIP={s}", right_w - 42, state.theme.skip_text)
    y += 1
    right_put(y, 2, f"Selected: {selected.label}", right_w - 4)
    y += 1
    right_put(y, 2, f"Commit: {cached}", right_w - 4)
    y += 2
    y += render_wrapped(right, y, 2, f"Action: {state.current_action}", right_w - 4, 2)
    y += render_wrapped(right, y, 2, f"Cmd: {state.current_cmd}", right_w - 4, 4)
    if y < top_h - 2 and selected.note:
        y += render_wrapped(right, y, 2, f"Chapter Hint: {selected.note}", right_w - 4, 2)

    if y < top_h - 2:
        right.addnstr(y, 2, f"Info: {state.status_line}", right_w - 4, state.theme.info_text)
    if y + 1 < top_h - 2:
        if state.log_search:
            hit_info = f"{state.search_pos + 1}/{len(state.search_hits)}" if state.search_hits and state.search_pos >= 0 else "0/0"
            right.addnstr(y + 1, 2, f"Search: /{state.log_search} ({hit_info})", right_w - 4)
        else:
            right.addnstr(y + 1, 2, "Search: (none)", right_w - 4)

    draw_box(logs, f"Live Logs ({'tail' if state.log_follow else 'scroll'})")
    log_h, log_w = logs.getmaxyx()
    log_title_attr = state.theme.panel_title | (curses.A_REVERSE if state.focus_idx == 2 else 0)
    logs.addnstr(
        0,
        2,
        f" Live Logs ({'tail' if state.log_follow else 'scroll'}) ",
        max(1, log_w - 4),
        log_title_attr,
    )
    visible = bottom_h - 2
    lines = list(state.log_lines)
    total = len(lines)
    max_start = max(0, total - visible)
    if state.log_follow:
        start = max_start
        state.log_anchor = start
    else:
        state.log_anchor = max(0, min(max_start, state.log_anchor))
        start = state.log_anchor

    view = lines[start : start + visible]
    inner_w = max(1, log_w - 2)

    # Clear only the content area; never touch the right border column.
    blank = " " * inner_w
    for i in range(visible):
        try:
            logs.addnstr(1 + i, 1, blank, inner_w)
        except curses.error:
            pass

    current_hit = -1
    if state.search_hits and state.search_pos >= 0:
        current_hit = state.search_hits[state.search_pos]

    for i, ln in enumerate(view):
        abs_idx = start + i
        attr = state.theme.search_hit if abs_idx == current_hit else curses.A_NORMAL
        try:
            logs.addnstr(1 + i, 1, fit_line(ln, inner_w), inner_w, attr)
        except curses.error:
            pass

    stdscr.refresh()


def main(stdscr: curses.window) -> int:
    curses.curs_set(0)
    stdscr.nodelay(True)
    stdscr.keypad(True)

    state = AppState(checked=[False] * len(CHAPTERS))
    state.theme = init_theme()
    draw_ui(stdscr, state)

    while True:
        ch = stdscr.getch()

        if ch == -1:
            time.sleep(0.03)
            continue

        if state.running:
            draw_ui(stdscr, state)
            time.sleep(0.03)
            continue

        if state.source_view.active:
            if ch in (27, ord("b"), ord("B"), ord("q"), ord("Q")):
                state.source_view = SourceViewerState()
                state.status_line = "Source view closed"
            elif ch in (curses.KEY_LEFT, ord("h"), ord("H")):
                source_go_file(state, -1)
            elif ch in (curses.KEY_RIGHT, ord("l"), ord("L")):
                source_go_file(state, +1)
            elif ch in (curses.KEY_UP, ord("k"), ord("K")):
                source_scroll(state, -1, max(1, stdscr.getmaxyx()[0] - 6))
            elif ch in (curses.KEY_DOWN, ord("j"), ord("J")):
                source_scroll(state, +1, max(1, stdscr.getmaxyx()[0] - 6))
            elif ch == curses.KEY_PPAGE:
                source_scroll(state, -12, max(1, stdscr.getmaxyx()[0] - 6))
            elif ch == curses.KEY_NPAGE:
                source_scroll(state, +12, max(1, stdscr.getmaxyx()[0] - 6))
            elif ch in (curses.KEY_HOME, ord("g")):
                state.source_view.scroll = 0
            elif ch in (curses.KEY_END, ord("G")):
                state.source_view.scroll = max(0, len(state.source_view.content) - 1)
            draw_ui(stdscr, state)
            continue

        if ch in (ord("q"), ord("Q")) and not state.running:
            break

        if ch == 9:  # Tab
            state.focus_idx = (state.focus_idx + 1) % 3
            state.status_line = f"Focus moved to {focus_name(state)}"
        elif ch == curses.KEY_BTAB:
            state.focus_idx = (state.focus_idx - 1) % 3
            state.status_line = f"Focus moved to {focus_name(state)}"
        elif ch in (ord("a"), ord("A")):
            state.checked = [True] * len(CHAPTERS)
        elif ch in (ord("c"), ord("C")):
            state.checked = [False] * len(CHAPTERS)
        elif state.focus_idx == 0:
            if ch in (curses.KEY_UP, ord("k"), ord("K")):
                state.selected_idx = (state.selected_idx - 1) % len(CHAPTERS)
            elif ch in (curses.KEY_DOWN, ord("j"), ord("J")):
                state.selected_idx = (state.selected_idx + 1) % len(CHAPTERS)
            elif ch in (ord(" "), curses.KEY_ENTER, 10, 13):
                state.checked[state.selected_idx] = not state.checked[state.selected_idx]
            elif ch in (curses.KEY_NPAGE,):
                state.selected_idx = min(len(CHAPTERS) - 1, state.selected_idx + 8)
            elif ch in (curses.KEY_PPAGE,):
                state.selected_idx = max(0, state.selected_idx - 8)
            elif ch in (curses.KEY_HOME, ord("g")):
                state.selected_idx = 0
            elif ch in (curses.KEY_END, ord("G")):
                state.selected_idx = len(CHAPTERS) - 1
        elif state.focus_idx == 1:
            items = menu_items(state)
            if ch in (curses.KEY_UP, ord("k"), ord("K")):
                state.menu_idx = (state.menu_idx - 1) % len(items)
            elif ch in (curses.KEY_DOWN, ord("j"), ord("J")):
                state.menu_idx = (state.menu_idx + 1) % len(items)
            elif ch in (curses.KEY_RIGHT, ord("l"), ord("L")):
                apply_menu_action(state, stdscr, +1)
            elif ch in (curses.KEY_LEFT, ord("h"), ord("H")):
                apply_menu_action(state, stdscr, -1)
            elif ch in (ord(" "), curses.KEY_ENTER, 10, 13):
                apply_menu_action(state, stdscr, +1)
            elif ch in (curses.KEY_HOME, ord("g")):
                state.menu_idx = 0
            elif ch in (curses.KEY_END, ord("G")):
                state.menu_idx = len(items) - 1
        elif state.focus_idx == 2:
            if ch in (curses.KEY_UP, ord("k"), ord("K")):
                adjust_log_scroll(state, -1, max(1, stdscr.getmaxyx()[0] - 8))
            elif ch in (curses.KEY_DOWN, ord("j"), ord("J")):
                adjust_log_scroll(state, +1, max(1, stdscr.getmaxyx()[0] - 8))
            elif ch in (curses.KEY_PPAGE,):
                adjust_log_scroll(state, -8, max(1, stdscr.getmaxyx()[0] - 8))
            elif ch in (curses.KEY_NPAGE,):
                adjust_log_scroll(state, 8, max(1, stdscr.getmaxyx()[0] - 8))
            elif ch in (curses.KEY_HOME, ord("g")):
                scroll_log_top(state)
            elif ch in (curses.KEY_END, ord("G")):
                scroll_log_bottom(state)
            elif ch == ord("/"):
                search_logs_action(state, stdscr)
            elif ch == ord("n"):
                jump_search(state, +1, max(1, stdscr.getmaxyx()[0] - 8))
            elif ch == ord("N"):
                jump_search(state, -1, max(1, stdscr.getmaxyx()[0] - 8))

        draw_ui(stdscr, state)

    return 0


def run_streaming_cli(host: str, shell_cmd: str, verbose: bool = True) -> Tuple[int, str]:
    argv = host_exec_argv(host, shell_cmd)
    raw_cmd = " ".join(shlex.quote(a) for a in argv)
    one_line_cmd = sanitize_log_line(raw_cmd.replace("\n", " | "))
    if verbose:
        print(f"$ {one_line_cmd}", flush=True)

    proc = subprocess.Popen(
        argv,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        bufsize=1,
    )
    output_accum: List[str] = []
    assert proc.stdout is not None
    for raw in proc.stdout:
        line = sanitize_log_line(raw.rstrip("\n"))
        output_accum.append(line)
        if verbose:
            print(line, flush=True)
    rc = proc.wait()
    return rc, "\n".join(output_accum)


def chapter_matches(chapter: ChapterSet, target: str) -> bool:
    t = target.lower()
    return (
        chapter.key.lower() == t
        or chapter.key.lower().startswith(t)
        or chapter.label.lower() == t
        or t in chapter.label.lower()
    )


def select_cli_chapters(targets: Sequence[str], all_chapters: bool) -> List[ChapterSet]:
    if all_chapters:
        return [ch for ch in CHAPTERS if not ch.key.startswith("live_")]
    selected: List[ChapterSet] = []
    for target in targets:
        matches = [ch for ch in CHAPTERS if chapter_matches(ch, target)]
        if not matches:
            raise SystemExit(f"unknown chapter target: {target}")
        if len(matches) > 1:
            labels = ", ".join(ch.key for ch in matches)
            raise SystemExit(f"ambiguous chapter target '{target}': {labels}")
        if matches[0] not in selected:
            selected.append(matches[0])
    if not selected:
        raise SystemExit("no chapter selected; pass a chapter key such as ch28_first_user_sret or use --all")
    return selected


def cli_preflight(host: str, repo: str, sync: bool, rebuild_image: bool, verbose: bool) -> bool:
    cmds = ["set -e", f"cd {shlex.quote(repo)}", "test -d .git"]
    if sync:
        cmds.extend(
            [
                "git fetch origin --prune",
                "if git show-ref --verify --quiet refs/heads/main; then git checkout -q main; fi",
                "git pull --ff-only origin main || true",
            ]
        )
    if rebuild_image:
        cmds.append("podman build -t mmos-practice:latest .")
    else:
        cmds.append(
            "if ! podman image exists mmos-practice:latest >/dev/null 2>&1; then podman build -t mmos-practice:latest .; fi"
        )
    rc, _ = run_streaming_cli(host, "\n".join(cmds), verbose)
    return rc == 0


def run_cli(args: argparse.Namespace) -> int:
    if args.list:
        for ch in CHAPTERS:
            print(f"{ch.key}\t{ch.label}\t{','.join(ch.commit_candidates)}")
        return 0

    host = args.host
    repo = args.repo or repo_path_for_host(host)
    selected = select_cli_chapters(args.targets, args.all)
    state = AppState(checked=[False] * len(CHAPTERS))
    state.host_idx = HOSTS.index(host)
    state.console_idx = CONSOLES.index(args.console)
    state.boot_chain_idx = BOOT_CHAINS.index(args.boot_chain)

    if not cli_preflight(host, repo, args.sync, args.rebuild_image, not args.quiet):
        print("RESULT FAIL preflight", flush=True)
        return 1

    overall_pass = True
    results: List[StepResult] = []
    for chapter in selected:
        commit = resolve_commit(host, repo, chapter, state, None)
        if not commit:
            overall_pass = False
            print(f"RESULT FAIL {chapter.key} resolve-commit: no candidate commit exists", flush=True)
            continue

        print(f"\n===== {chapter.label} [{chapter.key}] @ {commit} =====", flush=True)
        chapter_pass = True
        for step in chapter.steps:
            step_cmd = apply_build_options(step.cmd, state)
            step_shell = wrap_checkout_command(repo, commit, step_cmd)
            start = time.time()
            rc, out = run_streaming_cli(host, step_shell, not args.quiet)
            elapsed = time.time() - start
            status = "PASS"
            detail = ""
            if rc != 0:
                if step.allow_failure and step.expected_pattern and step.expected_pattern in out:
                    status = "WARN"
                    detail = f"expected historical failure matched: {step.expected_pattern}"
                else:
                    status = "FAIL"
                    detail = f"exit code: {rc}"
            elif step.must_contain:
                found, missing = check_patterns(out, step.must_contain)
                if found and not args.quiet:
                    print(f"[signature:found] {', '.join(found)}", flush=True)
                if missing:
                    status = "FAIL"
                    detail = f"missing expected signature: {', '.join(missing)}"

            if status != "PASS":
                chapter_pass = False
                overall_pass = False
            results.append(
                StepResult(
                    chapter_key=chapter.key,
                    chapter_label=chapter.label,
                    step_name=step.name,
                    status=status,
                    elapsed_sec=elapsed,
                    commit=commit,
                    cmd=step_cmd,
                    detail=detail,
                )
            )
            print(f"STEP {status} {chapter.key}/{step.name} ({elapsed:.1f}s) {detail}".rstrip(), flush=True)
            if status != "PASS" and not args.keep_going:
                break
        print(f"CHAPTER {'PASS' if chapter_pass else 'FAIL'} {chapter.key} {chapter.label}", flush=True)
        if not chapter_pass and not args.keep_going:
            break

    print("\n===== Summary =====", flush=True)
    for r in results:
        suffix = f" - {r.detail}" if r.detail else ""
        print(f"{r.status:4} {r.chapter_key}/{r.step_name} @ {r.commit}{suffix}", flush=True)
    print(f"OVERALL {'PASS' if overall_pass else 'FAIL'}", flush=True)
    return 0 if overall_pass else 1


def parse_args(argv: Optional[Sequence[str]] = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Verify mmos-practice chapter checkpoints.")
    parser.add_argument("targets", nargs="*", help="chapter key/label/prefix, e.g. ch28_first_user_sret or ch28")
    parser.add_argument("--cli", action="store_true", help="run without TUI and print pass/fail results")
    parser.add_argument("--list", action="store_true", help="list available chapter keys and exit")
    parser.add_argument("--all", action="store_true", help="run all non-live chapter sets")
    parser.add_argument("--host", choices=HOSTS, default="localhost")
    parser.add_argument("--repo", default="", help="override target repository path")
    parser.add_argument("--console", choices=CONSOLES, default="sbi")
    parser.add_argument("--boot-chain", choices=BOOT_CHAINS, default="smode")
    parser.add_argument("--sync", action="store_true", help="fetch/pull main before running")
    parser.add_argument("--rebuild-image", action="store_true", help="force rebuild of the podman image before running")
    parser.add_argument("--keep-going", action="store_true", help="continue after a failed chapter/step")
    parser.add_argument("--quiet", action="store_true", help="hide command output; keep result summary")
    return parser.parse_args(argv)


def entrypoint() -> int:
    args = parse_args()
    if args.cli or args.list:
        return run_cli(args)
    return curses.wrapper(main)


if __name__ == "__main__":
    raise SystemExit(entrypoint())
