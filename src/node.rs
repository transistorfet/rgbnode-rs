
use lexical_core;
use core::str::SplitWhitespace;
use cortex_m_semihosting::{ hprintln };

use crate::rgb::{ Stm32Rgb, RgbDevice, RgbEngine, Colour };
use crate::serial::{ SerialDevice, InputLine };


struct Command {
    pub name: &'static str,
    pub func: fn(&mut RgbNode, SplitWhitespace) -> (),
}

const COMMANDS: &[Command] = &[
    Command { name: "on", func: command_on },
    Command { name: "off", func: command_off },
];

fn command_on(rgbnode: &mut RgbNode, mut args: SplitWhitespace) {
    rgbnode.rgb.enable();
    if let Some(arg) = args.next() {
        if let Ok(i) = lexical_core::parse(arg.as_bytes()) {
            rgbnode.rgb.set_colour(Colour::new(i, i, i));
        }
    }
}

fn command_off(rgbnode: &mut RgbNode, args: SplitWhitespace) {
    rgbnode.rgb.disable();
}


pub struct RgbNode<'a> {
    pub rgb: Stm32Rgb,
    pub engine: RgbEngine,
    pub serial: SerialDevice<'a>
}

impl<'a> RgbNode<'a> {
    pub fn process_input(&mut self, input: &mut InputLine) {
        if self.serial.poll_read(input) {
            self.serial.write(&input.data[0..input.length]);
            if let Ok(line) = input.to_str() {
                self.process_command(line);
            }
            input.clear();
        }
    }

    pub fn process_command(&mut self, input: &str) {
        for line in input.lines() {
            hprintln!("{}", line).ok();
            let mut args = line.split_whitespace();
            let command = match args.next() {
                Some(x) => x,
                _ => return,
            };

            for cmd in COMMANDS {
                if cmd.name == command {
                    (cmd.func)(self, args);
                    return;
                }
            }
        }
    }

    pub fn handle_animation(&mut self) {
        self.engine.handle_animation(&mut self.rgb);
    }
}

