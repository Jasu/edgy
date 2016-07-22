use x11::xlib;
use std::ptr::{null_mut, null};

pub static mut display:Option<*mut xlib::Display> = None;
pub static mut root_window:xlib::Window = 0;

pub fn initialize_x11() {
    unsafe { 
        let d = xlib::XOpenDisplay(null());

        if d == null_mut() {
            panic!("Display could not be opened.");
        }

        display = Some(d);

        root_window = xlib::XRootWindow(display.unwrap(), xlib::XDefaultScreen(d));
    };
}
