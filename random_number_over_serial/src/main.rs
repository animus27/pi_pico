#![no_std]
#![no_main]

use panic_halt as _;
use rp_pico::entry;
use rp_pico::hal;
use rp_pico::hal::pac;
use usb_device::{class_prelude::*, prelude::*};
use usbd_serial::SerialPort;

use hal::rosc::RingOscillator;
#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();

    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    let clocks = hal::clocks::init_clocks_and_plls(
        rp_pico::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    #[cfg(feature = "rp2040-e5")]
    {
        let sio = hal::Sio::new(pac.SIO);
        let _pins = rp_pico::Pins::new(
            pac.IO_BANK0,
            pac.PADS_BANK0,
            sio.gpio_bank0,
            &mut pac.RESETS,
        );
    }

    // Set up the USB driver
    let usb_bus = UsbBusAllocator::new(hal::usb::UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));

    // Set up the USB Communications Class Device driver
    let mut serial = SerialPort::new(&usb_bus);

    // Create a USB device with a fake VID and PID
    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
        .manufacturer("Fake company")
        .product("Serial port")
        .serial_number("TEST")
        .device_class(2) // from: https://www.usb.org/defined-class-codes
        .build();

    let timer = hal::Timer::new(pac.TIMER, &mut pac.RESETS);
    let mut ptime = timer.get_counter();
    let mut test = true;

    let osc = RingOscillator::new(pac.ROSC);
    let mut ring: RingOscillator<hal::rosc::Enabled> = osc.initialize();


    loop {
        let _ = usb_dev.poll(&mut [&mut serial]);
        while test {
            match serial.flush() {
                Ok(_) => {
                    test = false;
                },
                Err(_) => {
                    let _ = usb_dev.poll(&mut [&mut serial]);
                },
            }
        }
        test = true;
        let mut rand: u64 = 0;
        (ring, rand) = random_generator(ring);
        if timer.get_counter() - ptime > 1_000 {
            let data = format_u64(rand);
            for i in data {
                if i != 0 {
                    match serial.write(&[i]) {
                        Ok(_) => (),
                        Err(_) => (),
                    }
                }
            }
            match serial.write(b"\n\r") {
                Ok(_) => (),
                Err(_) => (),
            }
            ptime = timer.get_counter();
        }
    }
}

fn format_u64(mut num: u64) -> [u8; 20] {
    let mut test: bool = false;
    let mut buffer: [u8; 20] = [0; 20];
    let mut buff: [u8; 20] = [0; 20];
    for i in (0..20).rev() {
        buffer[i] = (num % 10) as u8;
        num /= 10;
    }

    for i in 0..20 {
        if buffer[i] != 0 || test == true {
            test = true;
            buff[i] = buffer[i] + 48;
        }
    }

    buff
}

fn random_generator(ring: RingOscillator<hal::rosc::Enabled>) -> (RingOscillator<hal::rosc::Enabled>,u64) {
    let mut random_number: u16 = 0;
    for i in 0..16 {
        random_number |= (ring.get_random_bit() as u16) << i;
    }
    (ring, (random_number % 10000) as u64)
}

// End of file
