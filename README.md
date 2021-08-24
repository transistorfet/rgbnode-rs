
RGBNode-rs
==========

###### *Started April 30, 2021*

This is a port of [RGBNode](https://github.com/transistorfet/rgbnode) to Rust using a Bluepill with a STM32F103
microcontroller.  It uses [usbd-serial](https://github.com/mvirkkunen/usbd-serial) to connect to a computer via
serial, which can control the RGB output from a computer using a simple line-based text protocol.  It also has
an IR receiver using the [infrared](https://github.com/jkristell/infrared) library, with some custom codes that
can also control the RGB output.

I've been using OpenOCD to flash and debug the board.  To build the code and flash the board:
```
cargo build --release
openocd -f openocd.cfg -c "program target/thumbv7m-none-eabi/release/rgbnode-rs verify reset exit"
```

It is also possible to flash the program from within gdb:
```
openocd

# then, in a separate terminal run

gdb-multiarch target/thumbv7m-none-eabi/release/rgbnode-rs
(gdb) target extended-remote :3333
(gdb) monitor arm semihosting enable
(gdb) load
(gdb) run
```

A .gdbinit file is include which will connect to the remote debugger and enable semihosting, but it must be explicitly
allow from ~/.gdbinit with a line like:
```
add-auto-load-safe-path /path/to/project/rgbnode-rs/.gdbinit
```

Note: the OpenOCD target in openocd.cfg has been set to stm32f1x-clone.cfg, which is a copy of the standard
scripts/target/stm32f1x.cfg with `set _CPUTAPID 0x1ba01477` changed to `set _CPUTAPID 0x2ba01477` in order to work with
the CKS clone of the STM32 chip on my very cheap bluepill boards.

These boards also have an incorrect pullup resistor for D+ (10kOhm instead of 1.8kOhm), which makes the USB device
unable to be recognized by my computer.  I've attached an external 1.8kOhm resistor between 3.3V and PA12.  See
https://cgit.pinealservo.com/BluePill_Rust/resources/src/branch/master/notes.org#headline-2 for more details


Using via Serial
================

The following commands are recognized over serial:

`power <0|1>`
    Toggle power.  If the optional argument is provided, turn on (1) or off (0)

`intensity [0-255]`
    Change the intensity (brightness) to the given value

`index [0-30]`
    Change the colour to a preset indexed colour (the actual number will be mod the number of index colours)

`delay [0-100_000]`
    Change the delay used by animations to the given value.  For strobe, this will be the time between flashes.
    For colour swirl, this will be the fade time, follow by twice this delay of hold time between colour changes

`channel [0-9]`
    Change the colour mode to use (this is mapped to the IR remote channel numbers)

`red [0-255]`
    Change just the red colour channel to the given value

`green [0-255]`
    Change just the green colour channel to the given value

`blue [0-255]`
    Change just the blue colour channel to the given value

`indexup`
    Increment the indexed colour to use

`indexdown`
    Decrement the indexed colour to use

`version`
    Print the firmware version number

