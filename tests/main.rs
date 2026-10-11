// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use mmos_practice::align_up;

#[test]
fn integration_align_check() {
    assert_eq!(align_up(17, 8), 24);
}
