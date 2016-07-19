mod gesture_detector;
mod config;
mod actions;

#[macro_use]
extern crate nom;

extern crate x11;
extern crate clap;

use std::ffi::{
    CString, 
    CStr,
};

use std::mem::{zeroed, transmute};

use std::ptr::{
    null,
    null_mut,
};

use std::vec::Vec;

use std::os::raw::{c_int, c_ulong, c_uchar};
use x11::{xlib, xinput2};

use clap::{App, Arg};

use config::Config;
use gesture_detector::GestureDetector;
use actions::{Action, parse_action};

fn main () {
    let matches = App::new("Edgy")
        .version("0.1.0")
        .author("Jasper Mattsson <jasu@njomotys.info>")
        .about("Launch commands on  multi-touch swipes over screen edges.")
        .arg(Arg::with_name("device-name")
             .short("d")
             .help("Which named XInput device to use. For the list of available devices, run xinput --list.")
             .takes_value(true)
             .multiple(true))
        .arg(Arg::with_name("zone-width")
             .short("o")
             .help("Width, in pixels, of the region where a movement from outside to inside must start.")
             .takes_value(true))
        .arg(Arg::with_name("minimum-distance")
             .short("m")
             .help("Minimum distance in pixels required to register as a swipe.")
             .takes_value(true))
        .arg(Arg::with_name("detection-threshold")
             .short("t")
             .help("How many pixels the touch must move to the wrong direction before discarding it.")
             .takes_value(true))
        .arg(Arg::with_name("action")
             .short("a")
             .multiple(true)
             .help("E.g. \"from down to up with 2 fingers run command 'rm -rf/'\"")
             .takes_value(true))
        .get_matches();

    let mut device_names = Vec::new();
    for name in matches.values_of("device-name").unwrap() {
        device_names.push(String::from(name));
    }

    let (display, root_window) = initialize_x();

    if !has_xinput(display) {
        panic!("XInput extension is not available.");
    }

    if !has_xinput_2_2(display) {
        panic!("XInput extension is below XInput 2.2.");
    }



    grab_touch_begin(display, root_window);

    let (width, height) = get_window_size(display, root_window);

    let mut config = Config {
        screen_width: width as f64,
        screen_height: height as f64,

        zone_width: matches.value_of("zone-width")
            .unwrap_or("256")
            .parse::<f64>()
            .unwrap(),

        minimum_distance: matches.value_of("minimum-distance")
            .unwrap_or("64")
            .parse::<f64>()
            .unwrap(),

        detection_threshold: matches.value_of("detection-threshold")
            .unwrap_or("24")
            .parse::<f64>()
            .unwrap(),

        device_ids: Vec::new(),
    };

    config.device_ids = find_xinput_devices_by_name(display, device_names);

    let mut actions = Vec::new();
    for action_string in matches.values_of("action").unwrap() {
        let s = action_string.to_string();
        match parse_action(s.as_bytes()) {
            Some(action) => actions.push(action),
            None => panic!("Action was not parsed."),
        }
    }

    run_event_loop(display, root_window, &config, actions);
}

fn get_window_size(display: *mut xlib::Display, window: xlib::Window) -> (u32, u32) {
    let mut root:xlib::Window = 0;
    let mut x = 0;
    let mut y = 0;
    let mut width = 0;
    let mut height = 0;
    let mut border_width = 0;
    let mut depth = 0;
    if unsafe { xlib::XGetGeometry(display,
                                   window,
                                   &mut root,
                                   &mut x,
                                   &mut y,
                                   &mut width,
                                   &mut height,
                                   &mut border_width, &mut depth) } == xlib::False
    {
        panic!("Could not get window geometry.");
    }

    (width, height)
}

fn grab_touch_begin(display: *mut xlib::Display,
                    window: xlib::Window)
{
    // The pointer root device (id=2) is grabbed here. Grabbing a single device
    // does not seem to work (events are received AFTER they are processed by 
    // other applications, even for grabs.)

    let mut mask: [c_uchar; 4] = [0; 4];
    let mut modifiers =  xinput2::XIGrabModifiers {
        modifiers: 1 << 31,
        status: 0,
    };
    let mut input_event_mask = xinput2::XIEventMask {
        deviceid: 2,
        mask_len: mask.len() as i32,
        mask: mask.as_mut_ptr(),
    };

    for &event in &[xinput2::XI_TouchBegin,
                    xinput2::XI_TouchUpdate,
                    xinput2::XI_TouchEnd, ]
    {
        xinput2::XISetMask(&mut mask, event);
    }

    if unsafe { xinput2::XIGrabTouchBegin(display,
                                          2,
                                          window,
                                          /*owner_events=*/xlib::False,
                                          &mut input_event_mask,
                                          /*num_modifiers=*/1,
                                          &mut modifiers) } != 0
    {
        panic!("Could not grab TouchBegin.");
    }
}

fn accept_touch(display: *mut xlib::Display, window: xlib::Window, touch_id:i32) {
    unsafe { 
        xinput2::XIAllowTouchEvents(display, /*device_id=*/2, touch_id as u32, window, xinput2::XIAcceptTouch);
    }
}

fn reject_touch(display: *mut xlib::Display, window: xlib::Window, touch_id:i32) {
    unsafe {
        xinput2::XIAllowTouchEvents(display, /*device_id=*/2, touch_id as u32, window, xinput2::XIRejectTouch);
    }
}

fn run_event_loop(display: *mut xlib::Display, window: xlib::Window, config: &Config, actions: Vec<Action>) {

    let mut event: xlib::XEvent = unsafe { zeroed() };

    let mut on_accept_touch = & mut |touch_id| {
        accept_touch(display, window, touch_id);
    };

    let mut on_reject_touch = & mut |touch_id| {
        reject_touch(display, window, touch_id);
    };

    let mut on_gesture = &mut |gd : &mut GestureDetector, side, direction, num_touches| {
        for action in &actions {
            if action.side == side && action.direction == direction && action.num_fingers == num_touches
            {
                (action.function)(gd);
            }
        }
    };

    let mut gesture_detector = GestureDetector::new(
        config,
        on_accept_touch,
        on_reject_touch,
        on_gesture
        );

    loop {
        unsafe { xlib::XNextEvent(display, &mut event) };
        if event.get_type() != xlib::GenericEvent {
            continue;
        }

        // Logic: count touches that have crossed the same boundary.
        // when all touches end, check to see if the counts match any action,
        // then execute it. Reset counts.

        let mut cookie:xlib::XGenericEventCookie = From::from(event);

        if unsafe { xlib::XGetEventData(display, &mut cookie) } != xlib::True {
            println!("XGetEventData failed.");
            continue;
        }

        // Check if the state matches. Must have only touches for a single 
        // direction, otherwise discard the events due to ambiguity.

        match cookie.evtype {
            xinput2::XI_TouchBegin => {
                let event_data: &xinput2::XIDeviceEvent = unsafe{ transmute(cookie.data) };

                // If the touch does not come from a device that is being listened to, discard it.
                // (the root pointer device is grabbed since grabbing a single device does not work,
                //  thus check sourceid, not deviceid)
                let mut found = false;
                for &device_id in &config.device_ids {
                    if event_data.sourceid == device_id {
                        found = true;
                    }
                }

                if !found {
                    reject_touch(display, window, event_data.detail);
                    continue;
                }

                gesture_detector.handle_touch_start(event_data.detail, event_data.root_x, event_data.root_y);
            },
            xinput2::XI_TouchUpdate => {
                let event_data: &xinput2::XIDeviceEvent = unsafe{ transmute(cookie.data) };
                gesture_detector.handle_touch_update(event_data.detail, event_data.root_x, event_data.root_y);
            },
            xinput2::XI_TouchEnd => {
                let event_data: &xinput2::XIDeviceEvent = unsafe{ transmute(cookie.data) };
                gesture_detector.handle_touch_end(event_data.detail, event_data.root_x, event_data.root_y);
            },
            xinput2::XI_TouchOwnership => {
            }
            _ => {
                println!("Unknown event.");
            }
        }
    }
}

fn has_xinput(display:*mut xlib::Display) -> bool {
    let mut opcode:c_int = 0;
    let mut event:c_int = 0;
    let mut error:c_int = 0;

    let xinput_name = CString::new("XInputException").unwrap();

    unsafe{
        xlib::XQueryExtension(display, xinput_name.as_ptr(), &mut opcode, &mut event, &mut error) != xlib::True
    }
}

fn has_xinput_2_2(display:*mut xlib::Display) -> bool {
    let mut version_major = 2;
    let mut version_minor = 2;

    unsafe {
        if xinput2::XIQueryVersion(display, &mut version_major, &mut version_minor) != xlib::Success as c_int 
        {
            return false;
        }
    }

    version_major > 2 || version_major == 2 && version_minor >= 2
}

fn for_each_xinput_device<F>(display:*mut xlib::Display, f: &mut F) -> ()
    where F: FnMut(*mut xinput2::XIDeviceInfo) -> () {
        let mut device_count = 0;
        unsafe {
            let devices = xinput2::XIQueryDevice(display, xinput2::XIAllDevices, &mut device_count);
            for i in 0..device_count {
                let device = devices.offset(i as isize);
                f(device);
            }
            xinput2::XIFreeDeviceInfo(devices);
        }

    }

fn find_xinput_devices_by_name(display:*mut xlib::Display, names:Vec<String>) -> Vec<c_int> {
    let mut result:Vec<c_int> = Vec::new();

    for_each_xinput_device(display, &mut |device:*mut xinput2::XIDeviceInfo| {
        unsafe {
            let current_name = CStr::from_ptr((*device).name).to_string_lossy();
            for name in &names {
                if *name == current_name {
                    result.push((*device).deviceid);
                }
            }
        }
    });

    if result.len() == 0 {
        panic!("Could not find any device that was requested.");
    }
    if names.len() != result.len() {
        println!("Warning: found {} devices out of {} requested.",
                 result.len(),
                 names.len());
    }

    result
}

fn initialize_x() -> (*mut xlib::Display, c_ulong) {
    let display = unsafe { xlib::XOpenDisplay(null()) };

    if display == null_mut() {
        panic!("Display could not be opened.");
    }

    let root_window = unsafe { 
        xlib::XRootWindow(display, xlib::XDefaultScreen(display))
    };

    (display, root_window)
}

