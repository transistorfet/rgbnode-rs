#![no_std]
#![no_main]

extern crate panic_semihosting;

use cortex_m::asm::delay;
use cortex_m_rt::entry;
#[allow(unused_imports)]
use cortex_m_semihosting::{ debug, hprintln };

use embedded_hal::digital::v2::OutputPin;
use usb_device::{ prelude::*, bus::UsbBusAllocator };
use usbd_serial::{ SerialPort, USB_CLASS_CDC };

use stm32f1xx_hal::{
    stm32,
    prelude::*,
    time::U32Ext,
    pac::{ TIM2 },
    timer::{ Tim2NoRemap, Timer },
    usb::{ Peripheral, UsbBus },
    pwm::{ Pwm, C1, C2, C3 },
};

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().unwrap();

    let mut flash = dp.FLASH.constrain();
    let mut rcc = dp.RCC.constrain();

    // Configure the clocks
    let clocks = rcc
        .cfgr
        .use_hse(8.mhz())
        .sysclk(48.mhz())
        .pclk1(24.mhz())
        .freeze(&mut flash.acr);

    // Fetch the port devices we'll need
    let mut gpioa = dp.GPIOA.split(&mut rcc.apb2);
    let mut gpioc = dp.GPIOC.split(&mut rcc.apb2);


    // Configure the on-board LED (PC13, green)
    let mut led = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);
    led.set_high().ok(); // Turn off


    // Configure PWM
    let channels = (
        gpioa.pa0.into_alternate_push_pull(&mut gpioa.crl),
        gpioa.pa1.into_alternate_push_pull(&mut gpioa.crl),
        gpioa.pa2.into_alternate_push_pull(&mut gpioa.crl),
    );

    let mut afio = dp.AFIO.constrain(&mut rcc.apb2);
    //let pwm = pwm::tim2(dp.TIM2, channels, clocks, 20u32.khz());
    let pwm = Timer::tim2(dp.TIM2, &clocks, &mut rcc.apb1).pwm::<Tim2NoRemap, _, _, _>(
        channels,
        &mut afio.mapr,
        1.khz(),
    );
    let (mut ch1, ch2, ch3) = pwm;
    let max_duty = ch1.get_max_duty();
    ch1.set_duty(max_duty / 2);
    ch1.enable();

    let rgb = RGB::new(ch1, ch2, ch3);

    // Configure USB Serial
    assert!(clocks.usbclk_valid());

    // BluePill board has a pull-up resistor on the D+ line.
    // Pull the D+ pin down to send a RESET condition to the USB bus.
    // This forced reset is needed only for development, without it host
    // will not reset your device when you upload new firmware.
    let mut usb_dp = gpioa.pa12.into_push_pull_output(&mut gpioa.crh);
    usb_dp.set_low().ok();
    delay(clocks.sysclk().0 / 100);

    let usb = Peripheral {
        usb: dp.USB,
        pin_dm: gpioa.pa11,
        pin_dp: usb_dp.into_floating_input(&mut gpioa.crh),
    };
    let usb_bus = UsbBus::new(usb);

    let serial = SerialDevice::new(&usb_bus);

    let rgbnode = RGBNode {
        rgb,
        serial,
    };

    mainloop(rgbnode);
}

fn mainloop(mut rgbnode: RGBNode) -> ! {
    let mut input = InputLine::new();
    loop {
        if rgbnode.serial.poll_read(&mut input) {
            rgbnode.serial.write(&input.data[0..input.length]);
            rgbnode.process_command(core::str::from_utf8(&mut input.data).unwrap());
            input.clear();
        }

        /*
        for i in 0..rgbnode.rgb.max_duty {
            rgbnode.rgb.red.set_duty(i);
            delay(1000 - i as u32);
        }

        for i in (0..rgbnode.rgb.max_duty).rev() {
            rgbnode.rgb.red.set_duty(i);
            delay(1000 - i as u32);
        }
        */
    }
}



struct RGBNode<'a> {
    pub rgb: RGB,
    pub serial: SerialDevice<'a>
}

impl<'a> RGBNode<'a> {
    fn process_command(&mut self, input: &str) {
        for line in input.lines() {
            hprintln!("{}", line).ok();
            let mut args = line.split_whitespace();
            let command = match args.next() {
                Some(x) => x,
                _ => return,
            };

            if command == "on" {
                self.rgb.red.set_duty(self.rgb.max_duty);
            } else if command == "off" {
                self.rgb.red.set_duty(1);
            }
        }
    }
}



struct RGB {
    pub red: Pwm<TIM2, C1>,
    pub green: Pwm<TIM2, C2>,
    pub blue: Pwm<TIM2, C3>,
    pub max_duty: u16,
}

impl RGB {
    fn new(red: Pwm<TIM2, C1>, green: Pwm<TIM2, C2>, blue: Pwm<TIM2, C3>) -> RGB {
        let max_duty = red.get_max_duty();

        RGB {
            red,
            green,
            blue,
            max_duty,
        }
    }

}

struct SerialDevice<'a> {
    usb_dev: UsbDevice<'a, UsbBus<Peripheral>>,
    serial: SerialPort<'a, UsbBus<Peripheral>>,
}

impl<'a> SerialDevice<'a> {
    fn new(usb_bus: &'a UsbBusAllocator<UsbBus<Peripheral>>) -> SerialDevice<'a> {
        let serial = SerialPort::new(&usb_bus);

        let usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
            .manufacturer("Fake company")
            .product("Serial port")
            .serial_number("TEST")
            .device_class(USB_CLASS_CDC)
            .build();

        SerialDevice {
            usb_dev: usb_dev,
            serial: serial,
        }
    }

    fn poll_read(&mut self, input: &mut InputLine) -> bool {
        let mut buf = [0u8; 64];

        if self.poll() {
            return false;
        }

        match self.serial.read(&mut buf) {
            Ok(count) if count > 0 => {
                for c in buf[0..count].iter() {
                    input.push(*c);
                    if *c == '\n' as u8 {
                        return true;
                    }
                }
            },
            Ok(_) | Err(UsbError::WouldBlock) => { },
            Err(_) => { input.clear(); }
        }

        return false;
    }

    fn poll(&mut self) -> bool {
        !self.usb_dev.poll(&mut [&mut self.serial])
    }

    fn write(&mut self, string: &[u8]) {
        self.serial.write(string).ok();
    }
}

struct InputLine {
    pub length: usize,
    pub data: [u8; 128]
}

impl InputLine {
    pub fn new() -> InputLine {
        InputLine {
            length: 0,
            data: [0u8; 128]
        }
    }

    pub fn push(&mut self, ch: u8) {
        self.data[self.length] = ch;
        self.length += 1;
    }

    pub fn clear(&mut self) {
        self.length = 0;
    }
}

