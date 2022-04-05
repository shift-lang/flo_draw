use super::canvas_renderer::*;

impl CanvasRenderer {
    ///
    /// Stores the content of the clipping path from the current layer in a background buffer
    ///
    pub (super) fn tes_store(&mut self) {
        // TODO: this does not support the clipping behaviour (it stores/restores the whole layer)
        // (We currently aren't using the clipping behaviour for anything so it might be easier to just
        // remove that capability from the documentation?)
        self.core.sync(|core| core.layer(self.current_layer).state.restore_point = Some(core.layer(self.current_layer).render_order.len()));
    }

    ///
    /// Restores what was stored in the background buffer. This should be done on the
    /// same layer that the Store operation was called upon.
    ///
    /// The buffer is left intact by this operation so it can be restored again in the future.
    ///
    /// (If the clipping path has changed since then, the restored image is clipped against the new path)
    ///
    pub (super) fn tes_restore(&mut self) {
        // Roll back the layer to the restore point
        // TODO: need to reset the blend mode
        self.core.sync(|core| {
            if let Some(restore_point) = core.layer(self.current_layer).state.restore_point {
                let mut layer = core.layer(self.current_layer);

                // Remove entries from the layer until we reach the restore point
                while layer.render_order.len() > restore_point {
                    let removed_entity = layer.render_order.pop();
                    removed_entity.map(|removed| core.free_entity(removed));

                    // Reborrow the layer after removal
                    layer = core.layer(self.current_layer);
                }
            }
        })
    }

    ///
    /// Releases the buffer created by the last 'Store' operation
    ///
    /// Restore will no longer be valid for the current layer
    ///
    pub (super) fn tes_free_stored_buffer(&mut self) {
        self.core.sync(|core| core.layer(self.current_layer).state.restore_point = None);
    }

    ///
    /// Push the current state of the canvas (line settings, stored image, current path - all state)
    ///
    pub (super) fn tes_push_state(&mut self) {
        self.transform_stack.push(self.active_transform);

        self.core.sync(|core| {
            for layer_id in core.layers.clone() {
                core.layer(layer_id).push_state();
            }
        })
    }

    ///
    /// Restore a state previously pushed
    ///
    pub (super) fn tes_pop_state(&mut self) {
        // The current transform is applied globally
        self.transform_stack.pop()
            .map(|transform| self.active_transform = transform);

        self.core.sync(|core| {
            core.layer(self.current_layer).update_transform(&self.active_transform);

            for layer_id in core.layers.clone() {
                // Pop the state for the layer
                core.layer(layer_id).pop_state();
            }
        })
    }
}