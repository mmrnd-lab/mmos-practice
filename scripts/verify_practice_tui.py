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
ANSI_CSI_RE = re.compile(r"\x1B\[[0-?]*[ -/]*[@-~]")
ANSI_OSC_RE = re.compile(r"\x1B\][^\x07\x1B]*(?:\x07|\x1B\\)")
ANSI_ESC_RE = re.compile(r"\x1B[@-Z\\-_]")


@dataclass
class Step:
    name: str
    cmd: str
    allow_failure: bool = False
    expected_pattern: Optional[str] = None


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


CHAPTERS: List[ChapterSet] = [
    ChapterSet(
        key="ch04_kernel",
        label="Ch04 Kernel Bringup",
        commit_candidates=["41b8d41"],
        steps=[
            Step("build", "make mmos.bin PROFILE=release"),
            Step("run", "make run-nographic PROFILE=release RUN_SECONDS=4"),
        ],
        note="old stage: make test target not available",
    ),
    ChapterSet(
        key="ch05_test_core",
        label="Ch05 Test Core",
        commit_candidates=["27dfe5b"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step("run", "make run-nographic PROFILE=release RUN_SECONDS=4"),
        ],
    ),
    ChapterSet(
        key="ch06_memory",
        label="Ch06 Memory",
        commit_candidates=["1ff4bbe"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step("run", "make run-nographic PROFILE=release RUN_SECONDS=4"),
        ],
    ),
    ChapterSet(
        key="ch07_uboot",
        label="Ch07 U-Boot Flow",
        commit_candidates=["960cd95", "0c49470"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step("run-u-boot", "make run-u-boot PROFILE=release RUN_SECONDS=4"),
        ],
    ),
    ChapterSet(
        key="ch08_trap_scaffold",
        label="Ch08 Trap Scaffold",
        commit_candidates=["47757de", "4b2aaac", "a18fdd7"],
        steps=[
            Step(
                "test+build",
                "make test && make mmos.bin PROFILE=release",
                allow_failure=True,
                expected_pattern="csrsi sie, 0x222",
            ),
            Step("run-u-boot", "make run-u-boot PROFILE=release RUN_SECONDS=4"),
        ],
        note="known historical failure on old chain due csr immediate",
    ),
    ChapterSet(
        key="ch10_trap_syscall",
        label="Ch10 Trap+Syscall",
        commit_candidates=["352ac4e", "7be437e", "9b432b6"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step("run-u-boot", "make run-u-boot PROFILE=release RUN_SECONDS=4"),
        ],
    ),
    ChapterSet(
        key="ch11_sv39",
        label="Ch11 Sv39",
        commit_candidates=["636b73d", "5198a8b", "35455eb"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step("run-u-boot", "make run-u-boot PROFILE=release RUN_SECONDS=4"),
        ],
    ),
    ChapterSet(
        key="ch12_user_init",
        label="Ch12 User Init",
        commit_candidates=["075d672", "7864d66", "aebd5a6"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step("run-u-boot", "make run-u-boot PROFILE=release RUN_SECONDS=4"),
        ],
    ),
    ChapterSet(
        key="ch14_device",
        label="Ch14 Device Layer",
        commit_candidates=["a363c93", "23e4dac", "f56d0b8"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step("run-u-boot", "make run-u-boot PROFILE=release RUN_SECONDS=4"),
        ],
    ),
    ChapterSet(
        key="ch14_fs",
        label="Ch14 FS Layer",
        commit_candidates=["184b812", "09576d8", "2e62b95"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step("run-u-boot", "make run-u-boot PROFILE=release RUN_SECONDS=4"),
        ],
    ),
    ChapterSet(
        key="ch16_elf",
        label="Ch16 ELF Loader",
        commit_candidates=["2bd3c2c", "c417d4a", "7c47a7a"],
        steps=[
            Step("test+build", "make test && make mmos.bin PROFILE=release"),
            Step("run-u-boot", "make run-u-boot PROFILE=release RUN_SECONDS=4"),
        ],
    ),
]


def repo_path_for_host(host: str) -> str:
    if host == "localhost":
        return os.environ.get("MMOS_PRACTICE_LOCAL", LOCAL_REPO_DEFAULT)
    return os.environ.get("MMOS_PRACTICE_REMOTE", REMOTE_REPO_DEFAULT)


def host_exec_argv(host: str, shell_cmd: str) -> List[str]:
    if host == "localhost":
        return ["bash", "-lc", shell_cmd]
    return ["ssh", host, f"bash -lc {shlex.quote(shell_cmd)}"]


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


def commit_exists(host: str, repo: str, commit: str, state: AppState, stdscr: curses.window) -> bool:
    cmd = f"cd {shlex.quote(repo)} && git cat-file -e {shlex.quote(commit)}^{{commit}}"
    rc, _ = run_streaming(host, cmd, state, stdscr)
    return rc == 0


def resolve_commit(host: str, repo: str, chapter: ChapterSet, state: AppState, stdscr: curses.window) -> Optional[str]:
    for cand in chapter.commit_candidates:
        state.current_action = f"resolving commit for {chapter.label}: {cand}"
        draw_ui(stdscr, state)
        if commit_exists(host, repo, cand, state, stdscr):
            return cand
    return None


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
        if ! podman image exists mmos-practice:latest >/dev/null 2>&1; then
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


def run_selected(stdscr: curses.window, state: AppState, chapters: Sequence[ChapterSet]) -> None:
    selected = [ch for i, ch in enumerate(chapters) if state.checked[i]]
    if not selected:
        state.status_line = "No chapter selected"
        return

    host = HOSTS[state.host_idx]
    repo = repo_path_for_host(host)
    state.running = True
    state.stop_requested = False
    state.status_line = f"Running {len(selected)} chapter set(s) on {host}: {repo}"

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
            step_shell = wrap_checkout_command(repo, commit, step.cmd)
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

            state.results.append(
                StepResult(
                    chapter_key=chapter.key,
                    chapter_label=chapter.label,
                    step_name=step.name,
                    status=status,
                    elapsed_sec=elapsed,
                    commit=commit,
                    cmd=step.cmd,
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


def draw_ui(stdscr: curses.window, state: AppState) -> None:
    stdscr.erase()
    h, w = stdscr.getmaxyx()

    if h < 24 or w < 100:
        msg = "Terminal too small. Need at least 100x24. Resize and retry."
        stdscr.addnstr(0, 0, msg, max(1, w - 1))
        stdscr.refresh()
        return

    host = HOSTS[state.host_idx]
    repo = repo_path_for_host(host)

    header_h = 3
    body_h = h - header_h
    top_h = max(10, int(body_h * 0.52))
    bottom_h = body_h - top_h
    left_w = max(42, int(w * 0.48))
    right_w = w - left_w

    header = stdscr.derwin(header_h, w, 0, 0)
    left = stdscr.derwin(top_h, left_w, header_h, 0)
    right = stdscr.derwin(top_h, right_w, header_h, left_w)
    logs = stdscr.derwin(bottom_h, w, header_h + top_h, 0)

    draw_box(header, "MMOS Practice Verifier")
    hotkeys = "[UP/DOWN] move  [SPACE] toggle  [A] all  [C] clear  [H] host  [R] run  [S] stop  [PgUp/PgDn] logs  [/] find  [n/N] next/prev  [End] tail  [Q] quit"
    header.addnstr(1, 2, hotkeys, w - 4)

    draw_box(left, "Chapter Sets")
    max_items = top_h - 2
    start = 0
    if state.selected_idx >= max_items:
        start = state.selected_idx - max_items + 1

    for row, idx in enumerate(range(start, min(len(CHAPTERS), start + max_items))):
        ch = CHAPTERS[idx]
        mark = "x" if state.checked[idx] else " "
        cursor = ">" if idx == state.selected_idx else " "
        line = f"{cursor} [{mark}] {ch.label}"
        attr = curses.A_REVERSE if idx == state.selected_idx else curses.A_NORMAL
        left.addnstr(1 + row, 1, line, left_w - 2, attr)

    draw_box(right, "Run Status")
    p, f, wcnt, s = summarize_results(state.results)
    right.addnstr(1, 2, f"Host: {host}", right_w - 4)
    right.addnstr(2, 2, f"Repo: {repo}", right_w - 4)
    right.addnstr(3, 2, f"State: {'RUNNING' if state.running else 'IDLE'}", right_w - 4)
    right.addnstr(4, 2, f"Summary: PASS={p} FAIL={f} WARN={wcnt} SKIP={s}", right_w - 4)

    y = 6
    y += render_wrapped(right, y, 2, f"Action: {state.current_action}", right_w - 4, 2)
    y += render_wrapped(right, y, 2, f"Cmd: {state.current_cmd}", right_w - 4, 4)

    if y < top_h - 2:
        right.addnstr(y, 2, f"Info: {state.status_line}", right_w - 4)
    if y + 1 < top_h - 2:
        if state.log_search:
            hit_info = f"{state.search_pos + 1}/{len(state.search_hits)}" if state.search_hits and state.search_pos >= 0 else "0/0"
            right.addnstr(y + 1, 2, f"Search: /{state.log_search} ({hit_info})", right_w - 4)
        else:
            right.addnstr(y + 1, 2, "Search: (none)", right_w - 4)

    draw_box(logs, f"Live Logs ({'tail' if state.log_follow else 'scroll'})")
    log_h, log_w = logs.getmaxyx()
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
        attr = curses.A_REVERSE if abs_idx == current_hit else curses.A_NORMAL
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
    draw_ui(stdscr, state)

    while True:
        ch = stdscr.getch()

        if ch == -1:
            time.sleep(0.03)
            continue

        if ch in (ord("q"), ord("Q")) and not state.running:
            break

        if state.running:
            draw_ui(stdscr, state)
            time.sleep(0.03)
            continue

        if ch in (curses.KEY_UP, ord("k"), ord("K")):
            state.selected_idx = (state.selected_idx - 1) % len(CHAPTERS)
        elif ch in (curses.KEY_DOWN, ord("j"), ord("J")):
            state.selected_idx = (state.selected_idx + 1) % len(CHAPTERS)
        elif ch == ord(" "):
            state.checked[state.selected_idx] = not state.checked[state.selected_idx]
        elif ch in (ord("a"), ord("A")):
            state.checked = [True] * len(CHAPTERS)
        elif ch in (ord("c"), ord("C")):
            state.checked = [False] * len(CHAPTERS)
        elif ch in (ord("h"), ord("H")):
            state.host_idx = (state.host_idx + 1) % len(HOSTS)
            state.status_line = f"Host switched to {HOSTS[state.host_idx]}"
        elif ch in (ord("r"), ord("R")):
            state.log_lines.clear()
            state.log_follow = True
            state.log_anchor = 0
            rebuild_search_hits(state)
            state.results.clear()
            run_selected(stdscr, state, CHAPTERS)
        elif ch in (ord("s"), ord("S")):
            state.stop_requested = True
            state.status_line = "Nothing running"
        elif ch in (curses.KEY_PPAGE,):
            adjust_log_scroll(state, -8, max(1, stdscr.getmaxyx()[0] - 8))
        elif ch in (curses.KEY_NPAGE,):
            adjust_log_scroll(state, 8, max(1, stdscr.getmaxyx()[0] - 8))
        elif ch in (curses.KEY_HOME, ord("g")):
            scroll_log_top(state)
        elif ch in (curses.KEY_END, ord("G")):
            scroll_log_bottom(state)
        elif ch == ord("/"):
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
        elif ch == ord("n"):
            jump_search(state, +1, max(1, stdscr.getmaxyx()[0] - 8))
        elif ch == ord("N"):
            jump_search(state, -1, max(1, stdscr.getmaxyx()[0] - 8))

        draw_ui(stdscr, state)

    return 0


if __name__ == "__main__":
    raise SystemExit(curses.wrapper(main))
