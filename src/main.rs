#![no_std]
#![no_main]

use core::cell::RefCell;
use core::ops::{Deref};
use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::{AnyPin, Input, Level, Output, Pin, Pull, Speed};
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_time::Timer;
//noinspection RsUnusedImport
use {defmt_rtt as _, panic_probe as _};

enum Led {
    Green,
    Blue
}

static LED: Mutex<ThreadModeRawMutex, RefCell<Led>> = Mutex::new(RefCell::new(Led::Green));

#[embassy_executor::task]
async fn led_task(green_pin: AnyPin, blue_pin: AnyPin) {
    info!("Led Task");

    let mut green_led = Output::new(green_pin, Level::High, Speed::Low);
    let mut blue_led = Output::new(blue_pin, Level::Low, Speed::Low);

    loop {
        LED.lock(|ref_cell| {
            let borrowed = ref_cell.borrow();
            let led = borrowed.deref();

            match led {
                Led::Green => {
                    blue_led.set_low();
                    green_led.toggle()
                }
                Led::Blue => {
                    green_led.set_low();
                    blue_led.toggle()
                }
            }
        });

        Timer::after_millis(300).await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {

    info!("Init");
    let peripherals = embassy_stm32::init(Default::default());
    let button_input = Input::new(peripherals.PA0, Pull::None);
    let mut button = ExtiInput::new(button_input, peripherals.EXTI0);

    spawner.spawn(led_task(peripherals.PC9.degrade(), peripherals.PC8.degrade())).unwrap();

    loop {
        button.wait_for_rising_edge().await;
        info!("Set green");
        set_led(Led::Green).await;
        button.wait_for_falling_edge().await;

        button.wait_for_rising_edge().await;
        info!("Set blue");
        set_led(Led::Blue).await;
        button.wait_for_falling_edge().await;
    }
}

async fn set_led(led: Led) {
    LED.lock(|ref_cell| {
        ref_cell.replace(led);
    });
}