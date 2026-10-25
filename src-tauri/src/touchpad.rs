//! Touchpad pinches on Linux. WebKitGTK handles a pinch itself (zooming the
//! whole page) and never tells the page, so the canvases' own zoom can't be
//! reached from a touchpad. Pinches are caught before GTK delivers them to
//! the web view and sent to the page as `touchpad-pinch` events, which the
//! page turns into the Ctrl+wheel zoom the canvases already understand.
//! Windows (WebView2) already sends a pinch as Ctrl+wheel; macOS sends
//! gesture events the page handles itself.

use gtk::prelude::*;
use serde::Serialize;
use tauri::{Emitter, WebviewWindow};

/// One step of a pinch; `scale` is relative to where the pinch began.
#[derive(Clone, Serialize)]
struct Pinch {
    phase: &'static str,
    scale: f64,
}

/// Installs one GDK event handler for all windows. It sees every event before
/// GTK hands it to a widget, so a pinch stopped here never reaches WebKit;
/// everything else goes on to GTK as usual. (GTK 3 has no public signal for
/// its capture phase, so this is the earliest place to catch them.)
pub fn forward_pinches(windows: Vec<WebviewWindow>) {
    gtk::gdk::Event::set_handler(Some(move |event: &mut gtk::gdk::Event| {
        if let Some(pinch) = event.downcast_ref::<gtk::gdk::EventTouchpadPinch>()
            && let Some(target) = window_of(&windows, event)
        {
            let raw: &gtk::gdk::ffi::GdkEventTouchpadPinch = pinch.as_ref();
            let phase = match raw.phase as i32 {
                gtk::gdk::ffi::GDK_TOUCHPAD_GESTURE_PHASE_BEGIN => "begin",
                gtk::gdk::ffi::GDK_TOUCHPAD_GESTURE_PHASE_END => "end",
                gtk::gdk::ffi::GDK_TOUCHPAD_GESTURE_PHASE_CANCEL => "cancel",
                _ => "update",
            };
            let _ = target.emit(
                "touchpad-pinch",
                Pinch {
                    phase,
                    scale: raw.scale,
                },
            );
            return;
        }
        gtk::main_do_event(event);
    }));
}

/// The app window whose GDK window (or one of its children) got `event`.
fn window_of<'a>(
    windows: &'a [WebviewWindow],
    event: &gtk::gdk::Event,
) -> Option<&'a WebviewWindow> {
    let toplevel = event.window()?.toplevel();
    windows
        .iter()
        .find(|w| w.gtk_window().ok().and_then(|g| g.window()).as_ref() == Some(&toplevel))
}
