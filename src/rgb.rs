
use oorandom::Rand32;

use stm32f1xx_hal::{
    prelude::*,
    pac::{ TIM3 },
    pwm::{ PwmChannel, C1, C2, C3 },
};

use crate::millis;

type PwmRed = PwmChannel<TIM3, C1>;
type PwmGreen = PwmChannel<TIM3, C2>;
type PwmBlue = PwmChannel<TIM3, C3>;

pub struct Stm32Rgb {
    pub red: PwmRed,
    pub green: PwmGreen,
    pub blue: PwmBlue,
    pub max_duty: u16,
}

impl Stm32Rgb {
    pub fn new(mut red: PwmRed, mut green: PwmGreen, mut blue: PwmBlue) -> Self {
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
        // Scale the values linearly
        self.red.set_duty((col.r as u16) * (self.max_duty / 256));
        self.green.set_duty((col.g as u16) * (self.max_duty / 256));
        self.blue.set_duty((col.b as u16) * (self.max_duty / 256));

        // Scale the values exponentially
        //self.red.set_duty(((col.r as u32).pow(2) * self.max_duty as u32 / 65536) as u16);
        //self.green.set_duty(((col.g as u32).pow(2) * self.max_duty as u32 / 65536) as u16);
        //self.blue.set_duty(((col.b as u32).pow(2) * self.max_duty as u32 / 65536) as u16);
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

pub struct HoldFrame {
    pub start: u32,
    pub time: u32,
}

pub struct FadeChannel {
    pub millis_per_change: i32,
    pub millis_countdown: i32,
}

pub struct FadeFrame {
    pub channels: [FadeChannel; 3],
    pub target: Colour,
    pub last: u32,
    pub remain: u32,
}

pub enum Frame {
    Stop,
    Hold(HoldFrame),
    Fade(FadeFrame),
}

impl FadeChannel {
    pub fn new(start: i32) -> Self {
        FadeChannel {
            millis_per_change: start,
            millis_countdown: start.abs(),
        }
    }

    pub fn adjust(&mut self, diff: i32, input: u8) -> u8 {
        let mut output = input;

        if self.millis_per_change != 0 {
            self.millis_countdown -= diff;
            if self.millis_countdown < 0 {
                self.millis_countdown += self.millis_per_change.abs();

                if self.millis_per_change >= 1 {
                    output = bounded!(input as i32 + 1) as u8;
                } else if self.millis_per_change <= 1 {
                    output = bounded!(input as i32 - 1) as u8;
                }
            }
        }
        output
    }
}

impl Frame {
    pub fn new_fade(current: Colour, target: Colour, delay: u32) -> Frame {
        Frame::Fade(FadeFrame {
            channels: [
                FadeChannel::new(divide_or_zero!(delay as i32, target.r as i32 - current.r as i32)),
                FadeChannel::new(divide_or_zero!(delay as i32, target.g as i32 - current.g as i32)),
                FadeChannel::new(divide_or_zero!(delay as i32, target.b as i32 - current.b as i32)),
            ],
            target,
            last: millis(),
            remain: delay,
        })
    }
}


pub enum RgbMode {
    Solid,
    Cycle(usize),
    Strobe(bool, bool),
    Swirl(bool, usize, bool),
}


pub struct RgbEngine {
    enabled: bool,
    intensity: u8,
    delay: u32,
    index: usize,
    output: Colour,
    mode: RgbMode,
    frame: Frame,
    rand: Rand32,
}

impl RgbEngine {
    pub fn new() -> Self {
        RgbEngine {
            enabled: false,
            intensity: 255,
            delay: 5000,
            index: COLOUR_CYCLE_MAX - 1,
            output: Colour::new(0xff, 0xff, 0xff),
            mode: RgbMode::Swirl(false, 0, false),
            frame: Frame::Stop,
            rand: Rand32::new(millis() as u64),
        }
    }

    // Device Control Functions

    pub fn power<D: RgbDevice>(&mut self, dev: &mut D, on: bool) {
        self.enabled = on;
        match self.enabled {
            true => dev.enable(),
            false => dev.disable(),
        }
    }

    pub fn toggle<D: RgbDevice>(&mut self, dev: &mut D) {
        self.power(dev, !self.enabled);
    }

    pub fn handle_animation<D: RgbDevice>(&mut self, dev: &mut D) {
        if self.enabled {
            self.update_frame();
            dev.set_colour(self.output.scale(self.intensity));
        }
    }

    // Public Adjustment Functions

    pub fn intensity(&mut self, update: Option<u8>) -> u8 {
        if let Some(update) = update {
            self.intensity = update;
        }
        self.intensity
    }

    pub fn delay(&mut self, update: Option<u32>) -> u32 {
        if let Some(update) = update {
            self.delay = update;
        }
        self.delay
    }

    pub fn index(&mut self, update: Option<usize>) -> usize {
        if let Some(update) = update {
            self.index = update;
        }
        self.index
    }

    pub fn index_up(&mut self) {
        self.index = (self.index + 1) % COLOUR_INDEX.len();
    }

    pub fn index_down(&mut self) {
        if self.index == 0 {
            self.index = COLOUR_INDEX.len() - 1;
        } else {
            self.index -= 1;
        }
    }

    pub fn get_colour(&self) -> Colour {
        self.output
    }

    pub fn set_colour(&mut self, colour: Colour) {
        self.output = colour;
    }

    pub fn solid_mode(&mut self) {
        self.mode = RgbMode::Solid;
    }

    pub fn cycle_mode(&mut self) {
        self.mode = RgbMode::Cycle(0);
    }

    pub fn swirl_mode(&mut self, random: bool) {
        self.mode = RgbMode::Swirl(random, 0, false);
    }

    pub fn strobe_mode(&mut self, random: bool) {
        self.mode = RgbMode::Strobe(random, false);
    }

    pub fn force_update(&mut self) {
        self.frame = self.get_next_frame();
    }


    // Private State Control Functions

    fn update_frame(&mut self) {
        match self.frame {
            Frame::Stop => {
                self.frame = self.get_next_frame();
            },
            Frame::Hold(ref hold) => {
                if (millis() - hold.start) > hold.time {
                    self.frame = Frame::Stop
                }
            },
            Frame::Fade(ref mut fade) => {
                let current = millis();
                let diff = current - fade.last;
                if diff < 1 { return; }

                self.output.r = fade.channels[0].adjust(diff as i32, self.output.r);
                self.output.g = fade.channels[1].adjust(diff as i32, self.output.g);
                self.output.b = fade.channels[2].adjust(diff as i32, self.output.b);

                if fade.remain > diff {
                    fade.remain -= diff;
                    fade.last = current;
                } else {
                    self.output = fade.target;
                    self.frame = Frame::Stop;
                }
            },
        }
    }

    fn get_next_frame(&mut self) -> Frame {
        match self.mode {
            RgbMode::Solid => {
                Frame::Hold(HoldFrame { start: millis(), time: 1000 })
            },
            RgbMode::Cycle(ref mut index) => {
                advance_colour_index(index, COLOUR_CYCLE_MAX);
                self.output = COLOUR_INDEX[*index];

                Frame::Hold(HoldFrame { start: millis(), time: self.delay })
            },
            RgbMode::Swirl(ref random, ref mut index, ref mut hold) => {
                *hold = !*hold;

                if !*hold {
                    Frame::Hold(HoldFrame { start: millis(), time: self.delay })
                } else {
                    if *random {
                        let r = self.rand.rand_u32() as usize;
                        *index = r % COLOUR_CYCLE_MAX;
                    } else {
                        advance_colour_index(index, COLOUR_CYCLE_MAX);
                    }

                    let next = COLOUR_INDEX[*index];
                    Frame::new_fade(self.output, next, self.delay * 2)
                }
            },
            RgbMode::Strobe(ref random, ref mut hold) => {
                *hold = !*hold;

                if !*hold {
                    self.output = Colour::new(0, 0, 0);
                    Frame::Hold(HoldFrame { start: millis(), time: self.delay })
                } else {
                    if *random {
                        let r = self.rand.rand_u32() as usize;
                        self.index = r % COLOUR_CYCLE_MAX;
                    }

                    self.output = COLOUR_INDEX[self.index];
                    Frame::Hold(HoldFrame { start: millis(), time: 70 })
                }
            },
        }
    }


}

fn advance_colour_index(index: &mut usize, max: usize) {
    *index += 1;
    if *index >= max {
        *index = 0;
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

// This is the highest colour index that will be used for cycle patterns
const COLOUR_CYCLE_MAX: usize = 24;

const COLOUR_INDEX: &[Colour] = &[
    // NOTE these were ported from RGBNode, which doesn't adjust the PWM output for non-linearity, so the colours might not be what's expected
    Colour { r: 255, g:   0, b:   0 },
    Colour { r: 255, g:  32, b:   0 },
    Colour { r: 255, g:  64, b:   0 },
    Colour { r: 255, g: 128, b:   0 },
    Colour { r: 255, g: 255, b:   0 },
    Colour { r: 128, g: 255, b:   0 },
    Colour { r:  64, g: 255, b:   0 },
    Colour { r:  32, g: 255, b:   0 },

    Colour { r:   0, g: 255, b:   0 },
    Colour { r:   0, g: 255, b:  32 },
    Colour { r:   0, g: 255, b:  64 },
    Colour { r:   0, g: 255, b: 128 },
    Colour { r:   0, g: 255, b: 255 },
    Colour { r:   0, g: 128, b: 255 },
    Colour { r:   0, g:  64, b: 255 },
    Colour { r:   0, g:  32, b: 255 },

    Colour { r:   0, g:   0, b: 255 },
    Colour { r:  32, g:   0, b: 255 },
    Colour { r:  64, g:   0, b: 255 },
    Colour { r: 128, g:   0, b: 255 },
    Colour { r: 255, g:   0, b: 255 },
    Colour { r: 255, g:   0, b: 128 },
    Colour { r: 255, g:   0, b:  64 },
    Colour { r: 255, g:   0, b:  32 },

    Colour { r: 255, g: 255, b: 255 },
    Colour { r: 255, g: 255, b: 128 },
    Colour { r: 255, g: 255, b:  64 },
    Colour { r: 255, g: 192, b:   0 },
    Colour { r:  16, g: 255, b:   0 },
    Colour { r:   0, g:  80, b: 255 }
];

