/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::*;
use flo_curves::bezier;

#[test]
fn basis_at_t0_is_w1() {
    assert!(bezier::basis(0.0, 2.0, 3.0, 4.0, 5.0) == 2.0);
}

#[test]
fn basis_at_t1_is_w4() {
    assert!(bezier::basis(1.0, 2.0, 3.0, 4.0, 5.0) == 5.0);
}

#[test]
fn basis_agrees_with_de_casteljau() {
    for x in 0..100 {
        let t = (x as f64) / 100.0;

        let basis = bezier::basis(t, 2.0, 3.0, 4.0, 5.0);
        let de_casteljau = bezier::de_casteljau4(t, 2.0, 3.0, 4.0, 5.0);

        assert!(approx_equal(basis, de_casteljau));
    }
}
