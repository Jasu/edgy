use std::collections::HashMap;

use config::Config;

#[derive(Debug, Copy, Clone)]
struct Touch {
    touch_id: i32,
    device_id: i32,
    start_x: f64,
    start_y: f64,
    // Is the touch accepted or rejected.
    is_decided: bool,
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum Side {
    Top,
    Right,
    Bottom,
    Left,
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum Direction {
    Up,
    Right,
    Down,
    Left,
}

pub type GestureHook = FnMut(Side, Direction, u32);

pub struct GestureDetector<'a> {
    pub config: & 'a Config,
    // Total number of touches in the current gesture. Note that they might not 
    // be down at the same time.
    current_num_touches: u32,
    current_side: Option<Side>,
    current_direction: Option<Direction>,
    // If true, there has been an unrecognized or conflicting touch. Thus, the 
    // gesture is not detected.
    current_is_ruined: bool,

    // Currently pressed touches, keyed by touch id
    active_touches: HashMap<i32, Touch>,

    on_accept_touch: & 'a mut (FnMut(i32, i32) + 'a),
    on_reject_touch: & 'a mut (FnMut(i32, i32) + 'a),

    on_gesture: Option<& 'a mut (FnMut(&mut GestureDetector, Side, Direction, u32) + 'a)>,

    // If true, no touch is rejected, thus blocking touch screen from other 
    // applications.
    pub accept_all: bool,
}

impl <'a>GestureDetector<'a> {
    pub fn new(config: & 'a Config,
               on_accept_touch: & 'a mut (FnMut (i32, i32) + 'a),
               on_reject_touch: & 'a mut (FnMut (i32, i32) + 'a),
               on_gesture: & 'a mut (FnMut (&mut GestureDetector, Side, Direction, u32) + 'a))
        -> GestureDetector<'a>
        {
            GestureDetector {
                config: config,
                current_num_touches: 0,
                current_side: None,
                current_direction: None,
                current_is_ruined: false,
                active_touches: HashMap::new(),
                on_accept_touch: on_accept_touch,
                on_reject_touch: on_reject_touch,
                on_gesture: Some(on_gesture),
                accept_all: false,
            }
        }

    fn get_touch_side(&self, x: f64, y: f64) -> Option<Side> {
        let left_distance = x;
        let top_distance = y;
        let bottom_distance = self.config.screen_height - y - 1.0;
        let right_distance = self.config.screen_width - x - 1.0;

        if left_distance > self.config.zone_width && right_distance > self.config.zone_width && bottom_distance > self.config.zone_width && top_distance > self.config.zone_width 
        {
            None
        }
        else if left_distance < top_distance && left_distance < bottom_distance && left_distance < right_distance {
            Some(Side::Left)
        }
        else if right_distance < top_distance && right_distance < bottom_distance && right_distance < left_distance {
            Some(Side::Right)
        }
        else if bottom_distance < top_distance && bottom_distance < right_distance && bottom_distance < left_distance {
            Some(Side::Bottom)
        }
        else if top_distance < bottom_distance && top_distance < right_distance && top_distance < left_distance {
            Some(Side::Top)
        }
        else {
            None
        }
    }

    fn reject_touch(&mut self, touch_id: i32, device_id: i32) {
        if self.accept_all {
            (*self.on_accept_touch)(touch_id, device_id);
        }
        else {
            (*self.on_reject_touch)(touch_id, device_id);
        }
    }

    pub fn handle_touch_start(&mut self, touch_id:i32, device_id:i32, x:f64, y:f64) {
        if self.current_is_ruined {
            self.reject_touch(touch_id, device_id);
            return;
        }

        if let Some(side) = self.get_touch_side(x, y) {
            match self.current_side {
                Some(current_side) if current_side != side => {
                    self.reject_touch(touch_id, device_id);
                    if !self.active_touches.is_empty() {
                        // Don't mess up the state if there is no active gesture.
                        self.current_is_ruined = true;
                        reject_touches(&mut self.active_touches, if self.accept_all { self.on_accept_touch } else { self.on_reject_touch}  );
                    }
                    return;
                },
                _ => {
                    if self.accept_all {
                        (*self.on_accept_touch)(touch_id, device_id);
                    }
                    self.current_side = Some(side);
                },
            }

            self.active_touches.insert(touch_id, Touch { 
                touch_id: touch_id,
                device_id: device_id,
                start_x: x,
                start_y: y, 
                is_decided: self.accept_all,
            });
        }
        else {
            self.reject_touch(touch_id, device_id);
            if !self.active_touches.is_empty() {
                // Don't mess up the state if there is no active gesture.
                self.current_is_ruined = true;
                reject_touches(&mut self.active_touches, if self.accept_all { self.on_accept_touch } else { self.on_reject_touch });
            }
        }
    }

    pub fn handle_touch_end(&mut self, touch_id:i32, x:f64, y:f64) {
        if !self.active_touches.contains_key(&touch_id) {
            // When a touch event is rejected, a TouchEnd is sent after it.
            return;
        }


        self.handle_touch_update(touch_id, x, y);

        {
            let touch = self.active_touches.get_mut(&touch_id).unwrap();
            if !touch.is_decided {
                self.current_is_ruined = true;
            }
        }

        if self.active_touches.len() <= 1 {
            // The only touch in the vector is the touch that just ended.
            if self.current_direction != None && self.current_side != None && !self.current_is_ruined {
                let on_gesture = self.on_gesture.take().unwrap();
                let current_side = self.current_side.unwrap();
                let current_direction = self.current_direction.unwrap();
                let current_num_touches = self.current_num_touches;
                self.on_gesture = None;
                (*on_gesture)(
                    self,
                    current_side,
                    current_direction,
                    current_num_touches);
                self.on_gesture = Some(on_gesture);
            }
            else {
                reject_touches(&mut self.active_touches,
                               if self.accept_all { self.on_accept_touch } else { self.on_reject_touch });
            }

            self.reset_state();
        }
        else {
            self.active_touches.remove(&touch_id);
        }
    }

    pub fn handle_touch_update(&mut self, touch_id: i32, x: f64, y: f64) {
        if !self.active_touches.contains_key(&touch_id) {
            // Do not reject it, otherwise BadValue crashes the system, in case
            // it was already accepted.
            //
            // Also, the event might be from a touch that comes from a failed
            // touch when the device is grabbed.
            return;
        }

        {
            let touch = self.active_touches.get_mut(&touch_id).unwrap();

            match get_touch_direction(self.config, &touch, x, y) {
                Some(ref direction) if self.current_direction == None => {
                    if !is_valid_combination(self.current_side.unwrap(), *direction) {
                        self.current_is_ruined = true;
                    }
                    else if !touch.is_decided {
                        self.current_direction = Some(*direction);
                        self.current_num_touches += 1;
                        (*self.on_accept_touch)(touch.touch_id, touch.device_id);
                        touch.is_decided = true;
                    }
                },
                Some(ref direction)  if self.current_direction.unwrap() == *direction => {
                    if !touch.is_decided {
                        (*self.on_accept_touch)(touch.touch_id, touch.device_id);
                        self.current_num_touches += 1;
                        touch.is_decided = true;
                    }
                },
                Some(_) => {
                    self.current_is_ruined = true;
                },
                None => {
                },
            }
        }

        if self.current_is_ruined {
            reject_touches(&mut self.active_touches,
                           if self.accept_all { self.on_accept_touch } else { self.on_reject_touch });
        }
    }

    fn reset_state(&mut self) {
        self.active_touches.clear();
        self.current_side = None;
        self.current_direction = None;
        self.current_num_touches = 0;
        self.current_is_ruined = false;
    }
}


fn get_touch_direction(config: &Config, touch: &Touch, end_x: f64, end_y: f64) -> Option<Direction> {
    let diff_x = end_x - touch.start_x;
    let diff_y = end_y - touch.start_y;
    if diff_x.abs() > diff_y.abs() && diff_x.abs() > config.detection_threshold {
        if diff_x > 0.0 {
            Some(Direction::Right)
        }
        else {
            Some(Direction::Left)
        }
    }
    else if diff_y.abs() > diff_x.abs() && diff_y.abs() > config.detection_threshold {
        if diff_y > 0.0 {
            Some(Direction::Down)
        }
        else {
            Some(Direction::Up)
        }
    }
    else {
        None
    }
}

fn reject_touches(touches: &mut HashMap<i32, Touch>, on_reject_touch: &mut FnMut(i32, i32))
{
    for (_, touch) in touches {
        if !touch.is_decided {
            on_reject_touch(touch.touch_id, touch.device_id);
            touch.is_decided = true;
        }
    }
}


fn is_valid_combination(side:Side, direction:Direction) -> bool {
    (side == Side::Left || side == Side::Right) && (direction == Direction::Left || direction == Direction::Right) ||
        (side == Side::Top || side == Side::Bottom) && (direction == Direction::Up || direction == Direction::Down)
}
