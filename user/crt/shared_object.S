/* Copyright 2026 Sungjae Park
 * SPDX-License-Identifier: Apache-2.0
 */

.section .text
.global mmos_crt_shared_probe
.type mmos_crt_shared_probe, @function
mmos_crt_shared_probe:
    ret

.section .rodata
.global mmos_crt_shared_marker
mmos_crt_shared_marker:
    .asciz "mmos-user-crt shared object candidate"
