
use core::cell::RefCell;
use core::ops::DerefMut;
use cortex_m::interrupt::{ Mutex };

use stm32f1xx_hal::{
    stm32::{ interrupt, Interrupt, TIM2, NVIC },
    gpio::{ gpiob::PB8, Floating, Input },
    timer::{ CountDownTimer },
};

use infrared::{
    hal::{ PeriodicReceiver },
    protocols::{ Nec },
};


type IrProtocol = Nec;
type IrPin = PB8<Input<Floating>>;
type IrTimer = CountDownTimer<TIM2>;
type IrReceiver = PeriodicReceiver<IrProtocol, IrPin>;


pub const SAMPLERATE: u32 = 20_000;

static IR_CODE: Mutex<RefCell<Option<IrCode>>> = Mutex::new(RefCell::new(None));

static mut IR_TIMER: Option<CountDownTimer<TIM2>> = None;
static mut IR_RECEIVER: Option<IrReceiver> = None;


#[interrupt]
fn TIM2() {
    let timer = unsafe { IR_TIMER.as_mut().unwrap() };
    let receiver = unsafe { IR_RECEIVER.as_mut().unwrap() };

    timer.clear_update_interrupt_flag();

    if let Ok(Some(cmd)) = receiver.poll() {
        cortex_m::interrupt::free(|cs| {
            if !cmd.repeat {
                let mut data = IR_CODE.borrow(cs).borrow_mut();
                let ref mut code = *data.deref_mut();
                *code = Some(IrCode { protocol: IrType::Nec, addr: cmd.addr, cmd: cmd.cmd });
            }
        });
    }

}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum IrType {
    Nec,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct IrCode {
    pub protocol: IrType,
    pub addr: u8,
    pub cmd: u8,
}

pub struct IrDevice;

impl IrDevice {
    pub fn init(ir_pin: IrPin, ir_timer: IrTimer) {
        let ir_receiver: IrReceiver = PeriodicReceiver::new(ir_pin, SAMPLERATE);
        unsafe {
            IR_RECEIVER = Some(ir_receiver);
            IR_TIMER = Some(ir_timer);
            NVIC::unmask(Interrupt::TIM2);
        }
    }

    pub fn poll() -> Option<IrCode> {
        cortex_m::interrupt::free(|cs| {
            let mut data = IR_CODE.borrow(cs).borrow_mut();
            let ref mut code = *data.deref_mut();

            let result = code.clone();
            *code = None;
            result
        })
    }
}

