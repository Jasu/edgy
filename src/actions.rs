use gesture_detector::{Direction, Side, GestureDetector};

use std::str::{from_utf8, FromStr};
use nom::{space, digit};
use nom::IResult::Done;
use std::process::Command;
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

pub struct Action {
    pub side: Side,
    pub num_fingers: u32,
    pub direction: Direction,
    pub function: Box<Fn(&mut GestureDetector) -> ()>
}

named!(direction<&[u8], Direction>, alt!(
          tag!("up")     => { |_| Direction::Up }
        | tag!("top")    => { |_| Direction::Up }
        | tag!("right")  => { |_| Direction::Right }
        | tag!("down")   => { |_| Direction::Down }
        | tag!("bottom") => { |_| Direction::Down }
        | tag!("left")   => { |_| Direction::Left }
        ));

named!(side<&[u8], Side>, alt!(
          tag!("top")    => { |_| Side::Top }
        | tag!("up")     => { |_| Side::Top }
        | tag!("right")  => { |_| Side::Right }
        | tag!("bottom") => { |_| Side::Bottom }
        | tag!("down")   => { |_| Side::Bottom }
        | tag!("left")   => { |_| Side::Left }
        ));

// Parses "from top to bottom" or "left to right" or "to top from down" 
// or "right from left" to pair of (Side, Direction)
named!(from_to<&[u8], (Side, Direction)>, 
       alt!(
         chain!(
          chain!(
            tag!("from") ~
            space,
            || {}
          )? ~
          side_val: side ~
          space ~
          tag!("to") ~
          space ~
          dir_val: direction,
          || { (side_val, dir_val) })
        | chain!(
          chain!(
            tag!("to") ~
            space, || {}
          )? ~
          dir_val: direction ~ 
          space ~
          tag!("from") ~
          side_val: side,
          || { (side_val, dir_val) })
        ));

named!(u32_str<&[u8], u32>, 
       alt!(
           map_res!(map_res!(digit, from_utf8), u32::from_str)
         | tag!("one")      => { |_| 1 }
         | tag!("two")      => { |_| 2 }
         | tag!("three")    => { |_| 3 }
         | tag!("four")     => { |_| 4 }
         | tag!("five")     => { |_| 5 }
         | tag!("six")      => { |_| 6 }
         | tag!("seven")    => { |_| 7 }
         | tag!("eight")    => { |_| 8 }
         | tag!("nine")     => { |_| 9 }
         | tag!("ten")      => { |_| 10 }
       ));


named!(fingers<&[u8], u32>, 
       alt!(
           chain!(
               num: u32_str ~ 
               space ~
               alt!(tag!("fingers") | tag!("touches")),
               || { num }
           )
           | chain!(
                 alt!(tag!("one") | tag!("1")) ~
                 space ~
                 alt!(tag!("finger") | tag!("touch")),
                 || { 1 }
             )
   ));
named!(from_to_fingers<&[u8], (Side, Direction, u32)>,
      alt!(
          chain!(
            chain!(tag!("with") ~ space, || {})? ~
            fingers_val: fingers ~
            space ~
            from_to_val: from_to,
          || { (from_to_val.0, from_to_val.1, fingers_val) }
          )
          | chain!(
            from_to_val: from_to ~
            space ~
            chain!(tag!("with") ~ space, || {})? ~
            fingers_val: fingers,
          || { (from_to_val.0, from_to_val.1, fingers_val) }
          )
      ));

named!(run_command<&[u8], Box<Fn(&mut GestureDetector) -> ()> >,
  chain!(
    alt!(tag!("run") | tag!("execute") | tag!("exec")) ~
    space ~
    chain!(alt!(tag!("command") | tag!("cmd")) ~ space, || {})? ~
    cmd: alt!(
        delimited!(
            char!('"'),
            is_not!("\""),
            char!('"')
        )
        | 
        delimited!(
            char!('\''),
            is_not!("'"),
            char!('\'')
        )
    ),
    || {
        let cmd_copy = OsStr::from_bytes(cmd).to_os_string();
        Box::new(move |_: &mut GestureDetector| {
           Command::new("setsid")
                   .arg(&cmd_copy)
                   .spawn()
                   .expect("Failed to run command.");
        })
       }
  ));

named!(touchscreen<&[u8], ()>, chain!(
                         tag!("touch") ~
                         space? ~
                         tag!("screen")?, || {}));

named!(disable_touchscreen<&[u8], Box<Fn(&mut GestureDetector) -> ()> >,
          chain!(
            alt!(tag!("disable")
               | tag!("stop")
               | chain!(tag!("turn") ~ space ~ tag!("off"), || { &[] as &[u8] })
            ) ~
            space ~
            touchscreen,
            || { 
                 Box::new(|gesture_detector:&mut GestureDetector| {
                   gesture_detector.accept_all = true;
                 })
               }
          ));

named!(enable_touchscreen<&[u8], Box<Fn(&mut GestureDetector) -> ()> >,
          chain!(
            alt!(tag!("enable") 
               | tag!("start")
               | chain!(tag!("turn") ~ space ~ tag!("on"), || { &[] as &[u8] })) ~
            space ~
            touchscreen,
            || { 
                 Box::new(|gesture_detector:&mut GestureDetector| {
                   gesture_detector.accept_all = false;
                 })
               }
          ));

named!(toggle_touchscreen<&[u8], Box<Fn(&mut GestureDetector) -> ()> >,
          chain!(
            tag!("toggle") ~
            space ~
            touchscreen,
            || { 
                 Box::new(|gesture_detector:&mut GestureDetector| {
                   gesture_detector.accept_all = !gesture_detector.accept_all;
                 })
               }
          )
      );

named!(command<&[u8], Box<Fn(&mut GestureDetector) -> ()> >,
      alt!(
            run_command
          | disable_touchscreen
          | enable_touchscreen
          | toggle_touchscreen));

named!(action<&[u8], Action>, 
           dbg_dmp!( alt!(
             chain!(
                 function_val: command ~
                 space ~
                 from_to_fingers_val: from_to_fingers,
                 || 
                 {
                     Action { 
                         side: from_to_fingers_val.0,
                         direction: from_to_fingers_val.1,
                         num_fingers: from_to_fingers_val.2,
                         function: function_val
                     }
                 }
           )
           | chain!(
                 from_to_fingers_val: from_to_fingers ~
                 space ~
                 function_val: command,
                 || 
                 {
                     Action { 
                         side: from_to_fingers_val.0,
                         direction: from_to_fingers_val.1,
                         num_fingers: from_to_fingers_val.2,
                         function: function_val
                     }
                 }
           )
        )));

pub fn parse_action(description:&[u8]) -> Option<Action> {
    match action(description) {
        Done(_, action) => Some(action),
        _ => None
    }
}
