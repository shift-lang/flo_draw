/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use flo_curves::bezier;

#[test]
fn take_first_derivative() {
    assert!(bezier::derivative4(1.0, 2.0, 3.0, 4.0) == (3.0, 3.0, 3.0));
}
