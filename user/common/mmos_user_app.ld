/* Copyright 2026 Sungjae Park
 * SPDX-License-Identifier: Apache-2.0
 */

OUTPUT_ARCH(riscv)
ENTRY(_start)

SECTIONS
{
  . = 0x40010000;

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

  .bss (NOLOAD) : {
    *(.bss .bss.*)
    *(COMMON)
  }
}
