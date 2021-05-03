
use cortex_m_semihosting::{ hprintln };

use stm32f1xx_hal::{
    prelude::*,
    pac::{ TIM2 },
    pwm::{ PwmChannel, C1, C2, C3 },
};

use crate::millis;

pub struct Stm32Rgb {
    pub red: PwmChannel<TIM2, C1>,
    pub green: PwmChannel<TIM2, C2>,
    pub blue: PwmChannel<TIM2, C3>,
    pub max_duty: u16,
}

impl Stm32Rgb {
    pub fn new(mut red: PwmChannel<TIM2, C1>, mut green: PwmChannel<TIM2, C2>, mut blue: PwmChannel<TIM2, C3>) -> Self {
        let max_duty = red.get_max_duty();

        red.set_duty(max_duty);
        green.set_duty(max_duty);
        blue.set_duty(max_duty);

        Stm32Rgb {
            red,
            green,
            blue,
            max_duty,
        }
    }
}

impl RgbDevice for Stm32Rgb {
    fn enable(&mut self) {
        self.red.enable();
        self.green.enable();
        self.blue.enable();
    }

    fn disable(&mut self) {
        self.red.disable();
        self.green.disable();
        self.blue.disable();
    }

    fn set_colour(&mut self, col: Colour) {
        //self.red.set_duty((col.r as u16) * (self.max_duty / 256));
        //self.green.set_duty((col.g as u16) * (self.max_duty / 256));
        //self.blue.set_duty((col.b as u16) * (self.max_duty / 256));
        self.red.set_duty(((col.r as u32).pow(2) * self.max_duty as u32 / 65536) as u16);
        self.green.set_duty(((col.g as u32).pow(2) * self.max_duty as u32 / 65536) as u16);
        self.blue.set_duty(((col.b as u32).pow(2) * self.max_duty as u32 / 65536) as u16);
    }
}



pub trait RgbDevice {
    fn enable(&mut self);
    fn disable(&mut self);
    fn set_colour(&mut self, val: Colour);
}



macro_rules! divide_or_zero {
    ( $x:expr, $y:expr ) => {
        if $y == 0 { 0 } else { $x / $y }
    }
}

macro_rules! bounded {
    ( $x:expr ) => {
        if $x > 255 { 255 }
        else if $x < 0 { 0 }
        else { $x as u8 }
    }
}


#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Colour {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug)]
pub struct MillisPerNotch {
    pub r: i32,
    pub g: i32,
    pub b: i32,
}

pub enum Frame {
    Stop,
    Hold { start: u32, time: u32 },
    Fade { increments: MillisPerNotch, target: Colour, last: u32, remain: u32 },
}


pub enum RgbMode {
    Solid,
    Cycle(usize),
    Strobe,
    RandomStrobe,
    Swirl(usize),
    RandomSwirl,
}

impl Frame {
    pub fn new_fade(current: Colour, target: Colour, delay: u32) -> Frame {
        let increments = MillisPerNotch {
            r: divide_or_zero!(delay as i32, target.r as i32 - current.r as i32),
            g: divide_or_zero!(delay as i32, target.g as i32 - current.g as i32),
            b: divide_or_zero!(delay as i32, target.b as i32 - current.b as i32),
        };

        Frame::Fade {
            increments,
            target,
            last: millis(),
            remain: delay,
        }
    }
}


pub struct RgbEngine {
    enabled: bool,
    intensity: u8,
    output: Colour,
    mode: RgbMode,
    frame: Frame,
}

impl RgbEngine {
    pub fn new() -> Self {
        RgbEngine {
            enabled: false,
            intensity: 255,
            output: Colour::new(0xff, 0xff, 0xff),
            mode: RgbMode::Swirl(0),
            frame: Frame::Stop,
        }
    }

    pub fn toggle<D: RgbDevice>(&mut self, dev: &mut D) {
        self.enabled = !self.enabled;
        match self.enabled {
            true => dev.enable(),
            false => dev.disable(),
        }
    }

    pub fn handle_animation<D: RgbDevice>(&mut self, dev: &mut D) {
        if self.enabled {
            self.update_frame();
            dev.set_colour(self.output.scale(self.intensity));
        }
    }

    fn update_frame(&mut self) {
        match self.frame {
            Frame::Stop => {
                self.frame = self.get_next_frame();
            },
            Frame::Hold { start, time } => {
                if (millis() - start) > time {
                    self.frame = Frame::Stop
                }
            },
            Frame::Fade { ref increments, ref target, ref mut last, ref mut remain } => {
                let current = millis();
                let diff = current - *last;
                if diff < 20 { return; }

                self.output.r = bounded!(self.output.r as i32 + divide_or_zero!(diff as i32, increments.r));
                self.output.g = bounded!(self.output.g as i32 + divide_or_zero!(diff as i32, increments.g));
                self.output.b = bounded!(self.output.b as i32 + divide_or_zero!(diff as i32, increments.b));

                if *remain > diff {
                    *remain -= diff;
                    *last = current;
                } else {
                    self.output = *target;
                    self.frame = Frame::Stop;
                }
            },
        }
    }

    fn get_next_frame(&mut self) -> Frame {
        match self.mode {
            RgbMode::Cycle(ref mut index) => {
                *index += 1;
                if *index >= COLOUR_INDEX.len() {
                    *index = 0;
                }

                self.output = COLOUR_INDEX[*index];

                hprintln!("H: {}", index);
                Frame::Hold { start: millis(), time: 3000 }
            }
            RgbMode::Swirl(ref mut index) => {
                *index += 1;
                if *index >= COLOUR_INDEX.len() {
                    *index = 0;
                }

                let next = COLOUR_INDEX[*index];

                Frame::new_fade(self.output, next, 5000)
            }
            _ => Frame::Hold { start: millis(), time: 5000 }
        }
    }
}

impl Colour {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Colour { r, g, b }
    }

    pub fn scale(self, factor: u8) -> Self {
        Colour {
            r: ((self.r as u32) * factor as u32 / 255) as u8,
            g: ((self.g as u32) * factor as u32 / 255) as u8,
            b: ((self.b as u32) * factor as u32 / 255) as u8,
        }
    }
}

const COLOUR_INDEX: &[Colour] = &[
    Colour { r: 0xff, g: 0xff, b: 0xff },
    Colour { r: 0xff, g: 0,    b: 0 },
    Colour { r: 0,    g: 0xff, b: 0 },
    Colour { r: 0,    g: 0,    b: 0xff },
    Colour { r: 0,    g: 0xff, b: 0xff },
    Colour { r: 0xff, g: 0,    b: 0xff },
    Colour { r: 0xff, g: 0xff, b: 0 },
];

