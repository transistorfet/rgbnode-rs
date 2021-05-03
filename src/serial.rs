
use stm32f1xx_hal::usb::{ Peripheral, UsbBus };
use usb_device::{ prelude::*, bus::UsbBusAllocator };
use usbd_serial::{ SerialPort, USB_CLASS_CDC };


pub struct SerialDevice<'a> {
    usb_dev: UsbDevice<'a, UsbBus<Peripheral>>,
    serial: SerialPort<'a, UsbBus<Peripheral>>,
}

impl<'a> SerialDevice<'a> {
    pub fn new(usb_bus: &'a UsbBusAllocator<UsbBus<Peripheral>>) -> SerialDevice<'a> {
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

    pub fn poll_read(&mut self, input: &mut InputLine) -> bool {
        let mut buf = [0u8; 64];

        if self.poll() {
            return false;
        }

        match self.serial.read(&mut buf) {
            Ok(count) if count > 0 => {
                if input.push_data(&buf[0..count]) {
                    return true;
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

    pub fn write(&mut self, string: &[u8]) {
        self.serial.write(string).ok();
    }
}

pub struct InputLine {
    pub term: usize,
    pub length: usize,
    pub data: [u8; 128]
}

impl InputLine {
    pub fn new() -> InputLine {
        InputLine {
            term: 0,
            length: 0,
            data: [0u8; 128]
        }
    }

    pub fn push_data(&mut self, s: &[u8]) -> bool {
        let start = self.length;

        for ch in s {
            self.data[self.length] = *ch;
            self.length += 1;
        }

        for i in start..self.length {
            if self.data[i] == '\n' as u8 {
                self.term = i + 1;
                return true;
            }
        }

        return false;
    }

    pub fn clear(&mut self) {
        self.term = 0;
        self.length = 0;
    }

    pub fn discard(&mut self) {
        let diff = self.length - self.term;
        for i in 0..diff {
            self.data[i] = self.data[self.term + i];
        }

        self.length = diff;
        self.term = 0;
    }

    pub fn to_str(&self) -> Result<&str, core::str::Utf8Error> {
        core::str::from_utf8(&self.data[0..self.term])
    }
}

