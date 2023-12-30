/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use winit::window::{CursorIcon, Theme, WindowLevel};
use flo_binding::*;
use flo_canvas_events::*;

///
/// Trait implemented by objects that can provide properties for creating/updating a flo_draw window
///
/// Window properties are supplied as bindings to make it possible to update them after the window has
/// been created.
///
pub trait FloWindowProperties {
    ///
    /// The initial size of the window
    ///
    fn size(&self) -> BindRef<(u64, u64)>;
    fn min_size(&self) -> BindRef<Option<(u64, u64)>>;
    fn max_size(&self) -> BindRef<Option<(u64, u64)>>;
    ///
    /// The title of the window
    ///
    fn title(&self) -> BindRef<String>;
    fn is_transparent(&self) -> BindRef<bool>;
    fn is_visible(&self) -> BindRef<bool>;
    fn is_resizable(&self) -> BindRef<bool>;
    fn is_minimized(&self) -> BindRef<bool>;
    fn is_maximized(&self) -> BindRef<bool>;
    ///
    /// Set to true if the window should be fullscreen
    ///
    fn fullscreen(&self) -> BindRef<bool>;
    ///
    /// Set to true if the window should have decorations
    ///
    fn has_decorations(&self) -> BindRef<bool>;
    fn window_level(&self) -> BindRef<WindowLevel>;
    fn ime_position(&self) -> BindRef<(u64, u64)>;
    fn ime_allowed(&self) -> BindRef<bool>;
    fn theme(&self) -> BindRef<Option<Theme>>;
    fn cursor_position(&self) -> BindRef<(u64, u64)>;
    ///
    /// The mouse pointer to show for a window
    ///
    fn cursor_icon(&self) -> BindRef<MousePointer>;
}

///
/// '()' can be used to create a window with the default title
///
impl FloWindowProperties for () {
    fn size(&self) -> BindRef<(u64, u64)> {
        BindRef::from(bind((1024, 768)))
    }
    fn min_size(&self) -> BindRef<Option<(u64, u64)>> {
        BindRef::from(bind(None))
    }
    fn max_size(&self) -> BindRef<Option<(u64, u64)>> {
        BindRef::from(bind(None))
    }
    fn title(&self) -> BindRef<String> {
        BindRef::from(bind("flo_draw".to_string()))
    }
    fn is_transparent(&self) -> BindRef<bool> {
        BindRef::from(bind(false))
    }
    fn is_visible(&self) -> BindRef<bool> {
        BindRef::from(bind(true))
    }
    fn is_resizable(&self) -> BindRef<bool> {
        BindRef::from(bind(true))
    }
    fn is_minimized(&self) -> BindRef<bool> {
        BindRef::from(bind(false))
    }
    fn is_maximized(&self) -> BindRef<bool> {
        BindRef::from(bind(false))
    }
    fn fullscreen(&self) -> BindRef<bool> {
        BindRef::from(bind(false))
    }
    fn has_decorations(&self) -> BindRef<bool> {
        BindRef::from(bind(true))
    }
    fn window_level(&self) -> BindRef<WindowLevel> {
        BindRef::from(bind(WindowLevel::Normal))
    }
    fn ime_position(&self) -> BindRef<(u64, u64)> {
        BindRef::from(bind((0, 0)))
    }
    fn ime_allowed(&self) -> BindRef<bool> {
        BindRef::from(bind(false))
    }
    fn theme(&self) -> BindRef<Option<Theme>> {
        BindRef::from(bind(None))
    }
    fn cursor_position(&self) -> BindRef<(u64, u64)> {
        BindRef::from(bind((0, 0)))
    }
    fn cursor_icon(&self) -> BindRef<MousePointer> {
        BindRef::from(bind(MousePointer::SystemDefault(CursorIcon::Default)))
    }
}

///
/// A string can be used to set just the window title
///
impl<'a> FloWindowProperties for &'a str {
    fn size(&self) -> BindRef<(u64, u64)> {
        BindRef::from(bind((1024, 768)))
    }
    fn min_size(&self) -> BindRef<Option<(u64, u64)>> {
        BindRef::from(bind(None))
    }
    fn max_size(&self) -> BindRef<Option<(u64, u64)>> {
        BindRef::from(bind(None))
    }
    fn title(&self) -> BindRef<String> {
        BindRef::from(bind(self.to_string()))
    }
    fn is_transparent(&self) -> BindRef<bool> {
        BindRef::from(bind(false))
    }
    fn is_visible(&self) -> BindRef<bool> {
        BindRef::from(bind(true))
    }
    fn is_resizable(&self) -> BindRef<bool> {
        BindRef::from(bind(true))
    }
    fn is_minimized(&self) -> BindRef<bool> {
        BindRef::from(bind(false))
    }
    fn is_maximized(&self) -> BindRef<bool> {
        BindRef::from(bind(false))
    }
    fn fullscreen(&self) -> BindRef<bool> {
        BindRef::from(bind(false))
    }
    fn has_decorations(&self) -> BindRef<bool> {
        BindRef::from(bind(true))
    }
    fn window_level(&self) -> BindRef<WindowLevel> {
        BindRef::from(bind(WindowLevel::Normal))
    }
    fn ime_position(&self) -> BindRef<(u64, u64)> {
        BindRef::from(bind((0, 0)))
    }
    fn ime_allowed(&self) -> BindRef<bool> {
        BindRef::from(bind(false))
    }
    fn theme(&self) -> BindRef<Option<Theme>> {
        BindRef::from(bind(None))
    }
    fn cursor_position(&self) -> BindRef<(u64, u64)> {
        BindRef::from(bind((0, 0)))
    }
    fn cursor_icon(&self) -> BindRef<MousePointer> {
        BindRef::from(bind(MousePointer::SystemDefault(CursorIcon::Default)))
    }
}

///
/// The window properties struct provides a copy of all of the bindings for a window, and is a good way to provide
/// custom bindings (for example, if you want to be able to toggle the window betwen fullscreen and a normal display)
///
#[derive(Clone)]
pub struct WindowProperties {
    pub size: BindRef<(u64, u64)>,
    pub min_size: BindRef<Option<(u64, u64)>>,
    pub max_size: BindRef<Option<(u64, u64)>>,
    pub title: BindRef<String>,
    pub is_transparent: BindRef<bool>,
    pub is_visible: BindRef<bool>,
    pub is_resizable: BindRef<bool>,
    pub is_minimized: BindRef<bool>,
    pub is_maximized: BindRef<bool>,
    pub fullscreen: BindRef<bool>,
    pub has_decorations: BindRef<bool>,
    pub window_level: BindRef<WindowLevel>,
    pub ime_position: BindRef<(u64, u64)>,
    pub ime_allowed: BindRef<bool>,
    pub theme: BindRef<Option<Theme>>,
    pub cursor_position: BindRef<(u64, u64)>,
    pub cursor_icon: BindRef<MousePointer>,
}

impl WindowProperties {
    ///
    /// Creates a clone of an object implementing the FloWindowProperties trait
    ///
    pub fn from<T: FloWindowProperties>(properties: &T) -> WindowProperties {
        WindowProperties {
            size: properties.size(),
            min_size: properties.min_size(),
            max_size: properties.max_size(),
            title: properties.title(),
            is_transparent: properties.is_transparent(),
            is_visible: properties.is_visible(),
            is_resizable: properties.is_resizable(),
            is_minimized: properties.is_minimized(),
            is_maximized: properties.is_maximized(),
            fullscreen: properties.fullscreen(),
            has_decorations: properties.has_decorations(),
            window_level: properties.window_level(),
            ime_position: properties.ime_position(),
            ime_allowed: properties.ime_allowed(),
            theme: properties.theme(),
            cursor_position: properties.cursor_position(),
            cursor_icon: properties.cursor_icon(),
        }
    }
}

impl FloWindowProperties for WindowProperties {
    fn size(&self) -> BindRef<(u64, u64)> { self.size.clone() }
    fn min_size(&self) -> BindRef<Option<(u64, u64)>> { self.min_size.clone() }
    fn max_size(&self) -> BindRef<Option<(u64, u64)>> { self.max_size.clone() }
    fn title(&self) -> BindRef<String> { self.title.clone() }
    fn is_transparent(&self) -> BindRef<bool> { self.is_transparent.clone() }
    fn is_visible(&self) -> BindRef<bool> { self.is_visible.clone() }
    fn is_resizable(&self) -> BindRef<bool> { self.is_resizable.clone() }
    fn is_minimized(&self) -> BindRef<bool> { self.is_minimized.clone() }
    fn is_maximized(&self) -> BindRef<bool> { self.is_maximized.clone() }
    fn fullscreen(&self) -> BindRef<bool> { self.fullscreen.clone() }
    fn has_decorations(&self) -> BindRef<bool> { self.has_decorations.clone() }
    fn window_level(&self) -> BindRef<WindowLevel> { self.window_level.clone() }
    fn ime_position(&self) -> BindRef<(u64, u64)> { self.ime_position.clone() }
    fn ime_allowed(&self) -> BindRef<bool> { self.ime_allowed.clone() }
    fn theme(&self) -> BindRef<Option<Theme>> { self.theme.clone() }
    fn cursor_position(&self) -> BindRef<(u64, u64)> { self.cursor_position.clone() }
    fn cursor_icon(&self) -> BindRef<MousePointer> { self.cursor_icon.clone() }
}
