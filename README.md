 
RGBNode-rs
==========

###### *Started April 30, 2021*

This is a port of [RGBNode](https://github.com/transistorfet/rgbnode) to Rust using a Bluepill with a STM32F103
microcontroller.  It uses [usbd-serial](https://github.com/mvirkkunen/usbd-serial) to connect to a computer via
serial, which can control the RGB output using a simple line-based text protocol.


Note: the OpenOCD target in openocd.cfg has been set to stm32f1x-clone.cfg, which is a copy of the standard
scripts/target/stm32f1x.cfg with `set _CPUTAPID 0x1ba01477` changed to `set _CPUTAPID 0x2ba01477` in order to work with
the CKS clone of the STM32 chip on my very cheap bluepill boards.

These boards also have an incorrect pullup resistor for D+ (10kOhm instead of 1.8kOhm), which makes the USB device
unable to be recognized by my computer.  I've attached an external 1.8kOhm resistor between 3.3V and PA12.  See
https://cgit.pinealservo.com/BluePill_Rust/resources/src/branch/master/notes.org#headline-2 for more details

