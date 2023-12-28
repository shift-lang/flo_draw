/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use flo_canvas as canvas;

use super::canvas_renderer::*;

impl CanvasRenderer {
    ///
    /// Clears the currently selected sprite
    ///
    #[inline]
    pub(super) fn tes_namespace(&mut self, namespace: canvas::NamespaceId) {
        // The current namespace is used to identify different groupds of resources
        self.current_namespace = namespace.local_id();
    }
}
