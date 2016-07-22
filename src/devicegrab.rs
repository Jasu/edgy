use xconn::*;
use std::os::raw::c_uchar;
use x11::{xlib, xinput2};
use std::vec::Vec;

// Functions to grab / ungrab the whole device, to prevent events from going thorugh.
pub static mut is_grabbed:bool = false;

pub fn grab_devices(device_ids:&Vec<i32>) {
    unsafe {
        if is_grabbed {
            return;
        }
        is_grabbed = true;
    }

    for &device_id in device_ids {
        let mut mask: [c_uchar; 4] = [0; 4];

        let mut input_event_mask = xinput2::XIEventMask {
            deviceid: 2, //xinput2::XIAllDevices,
            mask_len: mask.len() as i32,
            mask: mask.as_mut_ptr(),
        };

        for &event in &[ xinput2::XI_TouchBegin,
                         xinput2::XI_TouchUpdate,
                         xinput2::XI_TouchEnd,

                         xinput2::XI_ButtonPress,
                         xinput2::XI_ButtonRelease,
                         xinput2::XI_Motion, ]
        {
            xinput2::XISetMask(&mut mask, event);
        }

        println!("Grabbing device #{}", device_id);
        unsafe { xinput2::XIGrabDevice(display.unwrap(), 
                                    device_id, 
                                    root_window,
                                    xlib::CurrentTime,
                                    /*cursor=*/0,
                                    xlib::GrabModeSync,
                                    xlib::GrabModeAsync,
                                    /*owner_events*/xlib::False, 
                                    &mut input_event_mask); }
        unsafe { xlib::XSync(display.unwrap(), xlib::True); }
    }
}

pub fn ungrab_devices(device_ids:&Vec<i32>) {
    unsafe {
        if !is_grabbed {
            return;
        }
        is_grabbed = false;
    }

    for &device_id in device_ids {
        println!("Ungrabbing device #{}.", device_id);
        unsafe { xinput2::XIUngrabDevice(display.unwrap(), device_id, xlib::CurrentTime); } 
    }

    unsafe { xlib::XSync(display.unwrap(), xlib::True); }
}

pub fn toggle_grab_devices(device_ids:&Vec<i32>) {
    if unsafe { is_grabbed } {
        ungrab_devices(device_ids);
    }
    else {
        grab_devices(device_ids);
    }
}
