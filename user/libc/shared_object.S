/* Copyright 2026 Sungjae Park
 * SPDX-License-Identifier: Apache-2.0
 */

    .section .text
    .global mmos_libc_shared_probe
    .type mmos_libc_shared_probe, @function
mmos_libc_shared_probe:
    ret

    .section .rodata
    .global mmos_libc_shared_marker
mmos_libc_shared_marker:
    .asciz "mmos-user-libc shared object candidate"
