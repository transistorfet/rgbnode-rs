#![no_std]
#![no_main]

extern crate panic_semihosting;

use cortex_m::asm::delay;
use cortex_m::peripheral::syst::SystClkSource;
use cortex_m_rt::{ entry, exception };
#[allow(unused_imports)]
use cortex_m_semihosting::{ debug, hprintln };

use embedded_hal::digital::v2::OutputPin;
use stm32f1xx_hal::{
    stm32,
    prelude::*,
    time::U32Ext,
    timer::{ Event, Timer, Tim3NoRemap },
    usb::{ Peripheral, UsbBus },
};


mod ir;
mod rgb;
mod node;
mod serial;

use ir::{ IrDevice };
use rgb::{ Stm32Rgb };
use node::{ RgbNode };
use serial::{ SerialDevice, InputLine };


//// System Timer for millisecond counting ////

static mut ELAPSED_MS: u32 = 0u32;

#[exception]
fn SysTick() {
    unsafe { ELAPSED_MS += 1; }
}

fn millis() -> u32 {
    return unsafe { ELAPSED_MS };
}


//// Main Setup ////

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().unwrap();
    let cp = stm32::CorePeripherals::take().unwrap();

    let mut flash = dp.FLASH.constrain();
    let mut rcc = dp.RCC.constrain();
    let mut afio = dp.AFIO.constrain(&mut rcc.apb2);

    // Configure the clocks
    let clocks = rcc
        .cfgr
        .use_hse(8.mhz())
        .sysclk(48.mhz())
        .pclk1(24.mhz())
        .freeze(&mut flash.acr);


    // Configure SysTick to generate a 1ms interrupt
    let mut syst = cp.SYST;
    syst.set_reload(48_000 - 1);
    syst.clear_current();
    syst.set_clock_source(SystClkSource::Core);
    syst.enable_counter();
    syst.enable_interrupt();


    // Fetch the port devices we'll need
    let mut gpioa = dp.GPIOA.split(&mut rcc.apb2);
    let mut gpiob = dp.GPIOB.split(&mut rcc.apb2);
    let mut gpioc = dp.GPIOC.split(&mut rcc.apb2);


    // Configure the on-board LED (PC13, green)
    let mut led = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);
    led.set_high().ok(); // Turn off


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


    // Configure IR
    let ir_pin = gpiob.pb8.into_floating_input(&mut gpiob.crh);
    let mut ir_timer = Timer::tim2(dp.TIM2, &clocks, &mut rcc.apb1).start_count_down(ir::SAMPLERATE.hz());
    ir_timer.listen(Event::Update);

    IrDevice::init(ir_pin, ir_timer);


    // Configure PWM
    let channels = (
        gpioa.pa6.into_alternate_push_pull(&mut gpioa.crl),
        gpioa.pa7.into_alternate_push_pull(&mut gpioa.crl),
        gpiob.pb0.into_alternate_push_pull(&mut gpiob.crl),
    );

    let pwm = Timer::tim3(dp.TIM3, &clocks, &mut rcc.apb1).pwm::<Tim3NoRemap, _, _, _>(
        channels,
        &mut afio.mapr,
        1.khz(),
    ).split();
    hprintln!("DUTY: {}", pwm.0.get_max_duty()).ok();
    let rgb = Stm32Rgb::new(pwm.0, pwm.1, pwm.2);


    // Create RgbNode object and run
    let rgbnode = RgbNode::new(rgb, serial);

    mainloop(rgbnode);
}

fn mainloop(mut rgbnode: RgbNode) -> ! {
    let mut input = InputLine::new();

    rgbnode.engine.toggle(&mut rgbnode.rgb);
    loop {
        rgbnode.process_input(&mut input);
        rgbnode.handle_animation();

        if let Some(code) = IrDevice::poll() {
            rgbnode.process_ir_code(code);
        }
    }

    /*
    let mut next_mil = millis();
    let mut on = true;
    rgbnode.rgb.red.enable();
    loop {
        let mil = millis();
        if mil > next_mil {
            on = !on;
            if on { rgbnode.rgb.red.set_duty(1000) } else { rgbnode.rgb.red.set_duty(1) }
            next_mil += 1000;
            //hprintln!("{}", mil);
        }
    }
    */
}

