/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#[allow(dead_code)]
///
/// Errors that can happen when trying to initialise the renderer
///
#[derive(Clone, Debug, PartialEq)]
pub enum RenderInitError {
    /// The required rendering API is not available
    ApiNotAvailable,

    /// Indicates that the graphics device could not be opened
    CannotOpenGraphicsDevice,

    /// Indicates that the graphics device could not be attached to
    CannotCreateGraphicsDevice,

    /// The graphics driver failed to initialise
    CannotStartGraphicsDriver,

    /// The graphics display is not available
    DisplayNotAvailable,

    /// A required extension was missing
    MissingRequiredExtension,

    /// Unable to configure the display
    CouldNotConfigureDisplay,

    /// The context failed to create
    CouldNotCreateContext,

    /// The render surface failed to create
    CouldNotCreateSurface,

    /// Could not set the active context
    ContextDidNotStart,
}
