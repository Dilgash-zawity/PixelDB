//! Touchpad pinches on Linux. WebKitGTK handles a pinch itself (zooming the
//! whole page) and never tells the page, so the canvases' own zoom can't be
//! reached from a touchpad. The window catches pinches before the web view
//! does and sends them to the page as `touchpad-pinch` events, which the
//! page turns into the Ctrl+wheel zoom the canvases already understand.
//! Windows (WebView2) already sends a pinch as Ctrl+wheel; macOS sends
//! gesture events the page handles itself.

use gtk::glib::prelude::*;
use serde::Serialize;
use tauri::{Emitter, WebviewWindow};

/// One step of a pinch; `scale` is relative to where the pinch began.
#[derive(Clone, Serialize)]
struct Pinch {
    phase: &'static str,
    scale: f64,
}

pub fn forward_pinches(window: &WebviewWindow) -> tauri::Result<()> {
    let target = window.clone();
    // `captured-event` runs from the window down before any widget sees the
    // event, so stopping it here keeps WebKit from zooming the page too.
    window.gtk_window()?.connect_local("captured-event", false, move |values| {
        let event = values.get(1)?.get::<gtk::gdk::Event>().ok()?;
        let pinch = event.downcast_ref::<gtk::gdk::EventTouchpadPinch>()?;
        let raw: &gtk::gdk::ffi::GdkEventTouchpadPinch = pinch.as_ref();
        let phase = match raw.phase as i32 {
            gtk::gdk::ffi::GDK_TOUCHPAD_GESTURE_PHASE_BEGIN => "begin",
            gtk::gdk::ffi::GDK_TOUCHPAD_GESTURE_PHASE_END => "end",
            gtk::gdk::ffi::GDK_TOUCHPAD_GESTURE_PHASE_CANCEL => "cancel",
            _ => "update",
        };
        let _ = target.emit("touchpad-pinch", Pinch { phase, scale: raw.scale });
        Some(true.to_value())
    });
    Ok(())
}
