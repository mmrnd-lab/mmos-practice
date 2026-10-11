/* Copyright 2026 Sungjae Park
 * SPDX-License-Identifier: Apache-2.0
 */

OUTPUT_ARCH(riscv)
ENTRY(_start)

SECTIONS
{
  . = 0x80200000;

  .text : {
    *(.text.entry)
    *(.text .text.*)
  }

  .rodata : {
    *(.rodata .rodata.*)
  }

  .data : {
    *(.data .data.*)
  }

  .bss : {
    *(.bss .bss.*)
    *(COMMON)
  }
}
