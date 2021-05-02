
use stm32f1xx_hal::{
    prelude::*,
    pac::{ TIM2 },
    pwm::{ Pwm, C1, C2, C3 },
};

pub struct Stm32Rgb {
    pub red: Pwm<TIM2, C1>,
    pub green: Pwm<TIM2, C2>,
    pub blue: Pwm<TIM2, C3>,
    pub max_duty: u16,
}

impl Stm32Rgb {
    pub fn new(mut red: Pwm<TIM2, C1>, mut green: Pwm<TIM2, C2>, mut blue: Pwm<TIM2, C3>) -> Self {
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
        self.red.set_duty((col.r as u16) * (self.max_duty / 256));
        self.green.set_duty((col.g as u16) * (self.max_duty / 256));
        self.blue.set_duty((col.b as u16) * (self.max_duty / 256));
    }
}



pub trait RgbDevice {
    fn enable(&mut self);
    fn disable(&mut self);
    fn set_colour(&mut self, val: Colour);
}




pub struct Colour {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

pub enum RgbMode {
    Solid,
    Strobe,
    RandomStrobe,
    Swirl,
    RandomSwirl,
}

pub struct RgbEngine {
    enabled: bool,
    mode: RgbMode,
    delay: u32,
    output: Colour,
    target: Colour,
}

impl RgbEngine {
    pub fn new() -> Self {
        RgbEngine {
            enabled: false,
            mode: RgbMode::Solid,
            delay: 0,
            output: Colour::new(0xff, 0xff, 0xff),
            target: Colour::new(0xff, 0xff, 0xff),
        }
    }

    pub fn toggle<D: RgbDevice>(&mut self, dev: &mut D) {
        self.enabled = !self.enabled;
        match self.enabled {
            true => dev.enable(),
            false => dev.disable(),
        }
    }

    pub fn set_colour<D: RgbDevice>(&mut self, dev: &mut D, val: Colour) {
        dev.set_colour(val);
    }

    fn setup_hold(&mut self, target: Colour, delay: u32) {

    }

    fn setup_fade(&mut self, target: Colour, delay: u32) {

    }
}

impl Colour {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Colour { r, g, b }
    }
}

