
use lexical_core;
use cortex_m_semihosting::{ hprintln };

use crate::rgb::{ Stm32Rgb, RgbEngine };
use crate::serial::{ SerialDevice, InputLine };


struct Command {
    pub name: &'static str,
    pub min: u8,
    pub func: fn(&mut RgbNode, &[&str]) -> (),
}

const COMMANDS: &[Command] = &[
    Command { name: "power", min: 0, func: command_power },
    Command { name: "red", min: 1, func: command_red },
    Command { name: "green", min: 1, func: command_green },
    Command { name: "blue", min: 1, func: command_blue },
    Command { name: "delay", min: 1, func: command_delay },
    Command { name: "index", min: 1, func: command_index },
    Command { name: "channel", min: 1, func: command_channel },
    Command { name: "intensity", min: 1, func: command_intensity },
    Command { name: "indexup", min: 0, func: command_indexup },
    Command { name: "indexdown", min: 0, func: command_indexdown },
    Command { name: "version", min: 0, func: command_version },
    //{ "ir", 2, command_ir },
    //{ "key", 1, command_key },
    //{ "color", 1, command_color },
    //{ "chanup", 0, command_chanup },
    //{ "chandown", 0, command_chandown },
    //{ "calibrate", 1, command_calibrate },
];

fn command_power(rgbnode: &mut RgbNode, args: &[&str]) {
    if args.len() > 1 {
        if let Ok(i) = lexical_core::parse::<i32>(args[1].as_bytes()) {
            rgbnode.engine.power(&mut rgbnode.rgb, if i > 0 { true } else { false });
        }
    } else {
        rgbnode.engine.toggle(&mut rgbnode.rgb);
    }
}

fn command_red(rgbnode: &mut RgbNode, args: &[&str]) {
    let mut colour = rgbnode.engine.get_colour();
    if let Ok(i) = lexical_core::parse::<u8>(args[1].as_bytes()) {
        colour.r = i;
        rgbnode.engine.set_colour(colour);
    }
}

fn command_green(rgbnode: &mut RgbNode, args: &[&str]) {
    let mut colour = rgbnode.engine.get_colour();
    if let Ok(i) = lexical_core::parse::<u8>(args[1].as_bytes()) {
        colour.g = i;
        rgbnode.engine.set_colour(colour);
    }
}

fn command_blue(rgbnode: &mut RgbNode, args: &[&str]) {
    let mut colour = rgbnode.engine.get_colour();
    if let Ok(i) = lexical_core::parse::<u8>(args[1].as_bytes()) {
        colour.b = i;
        rgbnode.engine.set_colour(colour);
    }
}

fn command_delay(rgbnode: &mut RgbNode, args: &[&str]) {
    if let Ok(ms) = lexical_core::parse::<u32>(args[1].as_bytes()) {
        rgbnode.engine.delay(Some(ms));
    }
}

fn command_index(rgbnode: &mut RgbNode, args: &[&str]) {
    if let Ok(i) = lexical_core::parse::<usize>(args[1].as_bytes()) {
        rgbnode.engine.index(Some(i));
    }
}

fn command_channel(rgbnode: &mut RgbNode, args: &[&str]) {
    if let Ok(ch) = lexical_core::parse::<u8>(args[1].as_bytes()) {
        rgbnode.change_channel(ch);
    }
}

fn command_intensity(rgbnode: &mut RgbNode, args: &[&str]) {
    if let Ok(i) = lexical_core::parse::<u8>(args[1].as_bytes()) {
        rgbnode.engine.intensity(Some(i));
    }
}

fn command_indexup(rgbnode: &mut RgbNode, _args: &[&str]) {
    rgbnode.engine.index_up();
}

fn command_indexdown(rgbnode: &mut RgbNode, _args: &[&str]) {
    rgbnode.engine.index_down();
}

fn command_version(rgbnode: &mut RgbNode, _args: &[&str]) {
    rgbnode.send_response("version 0.1");
}



pub struct RgbNode<'a> {
    pub rgb: Stm32Rgb,
    pub engine: RgbEngine,
    pub serial: SerialDevice<'a>,
    sent: bool,
}

impl<'a> RgbNode<'a> {
    pub fn new(rgb: Stm32Rgb, serial: SerialDevice<'a>) -> Self {
        RgbNode {
            rgb,
            serial,
            engine: RgbEngine::new(),
            sent: false,
        }
    }

    pub fn process_input(&mut self, input: &mut InputLine) {
        if self.serial.poll_read(input) {
            if let Ok(line) = input.to_str() {
                self.process_command(line.trim_end());
            } else {
                self.return_error();
            }
            input.discard();
        }
    }

    pub fn process_command(&mut self, line: &str) {
        hprintln!("{}", line).ok();

        let mut i = 0;
        let mut args: [&str; 10] = [""; 10];
        for string in line.split_whitespace() {
            args[i] = string;
            i += 1;
        }
        if args[0] == "" {
            return;
        }

        self.sent = false;
        for cmd in COMMANDS {
            if cmd.name == args[0] {
                if i > cmd.min as usize {
                    (cmd.func)(self, &args[0..i]);

                    if !self.sent {
                        self.send_response(line);
                    }
                } else {
                    self.return_error();
                }
                return;
            }
        }

        // No Command Found
        self.return_error();
    }

    pub fn handle_animation(&mut self) {
        self.engine.handle_animation(&mut self.rgb);
    }

    fn send_response(&mut self, response: &str) {
        self.sent = true;
        self.serial.write(response.as_bytes());
        self.serial.write("\n".as_bytes());
    }

    fn return_error(&mut self) {
        self.serial.write("error\n".as_bytes());
    }

    pub fn change_channel(&mut self, ch: u8) {
        match ch {
            0 => self.engine.cycle_mode(),
            1 => { self.engine.solid_mode(); self.engine.index(Some(26)); },
            2 => { self.engine.solid_mode(); self.engine.index(Some(27)); },
            3 => { self.engine.solid_mode(); self.engine.index(Some(28)); },
            4 => self.engine.solid_mode(),
            5 => self.engine.strobe_mode(false),
            6 => self.engine.strobe_mode(true),
            7 => self.engine.swirl_mode(false),
            8 => self.engine.swirl_mode(true),
            _ => { },
        }
    }
}

