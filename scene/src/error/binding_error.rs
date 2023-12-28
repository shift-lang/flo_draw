/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

///
/// Error associated with an error binding a property
///
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum BindingError {
    /// The requested binding was not available
    Missing,

    /// The request was dropped before the binding could be retrieved
    Abandoned,
}
