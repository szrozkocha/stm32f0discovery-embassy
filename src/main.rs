#![no_std]
#![no_main]

use core::cell::RefCell;
use core::ops::{Deref};
use core::sync::atomic::{AtomicU16, Ordering};
use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::adc::{Adc, AdcPin, SampleTime};
use embassy_stm32::{adc, bind_interrupts};
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::{AnyPin, Input, Level, Output, Pin, Pull, Speed};
use embassy_stm32::peripherals::{ADC, PC0};
use embassy_sync::mutex::Mutex;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_time::{Delay, Timer};
//noinspection RsUnusedImport
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    ADC1_COMP => adc::InterruptHandler<ADC>;
});

#[derive(Clone, Copy, Format)]
enum LedType {
    Green,
    Blue
}

impl LedType {
    fn toggle(&mut self) {
        match self {
            LedType::Green => *self = LedType::Blue,
            LedType::Blue => *self = LedType::Green,
        }
    }
}

static LED: Mutex<ThreadModeRawMutex, RefCell<LedType>> = Mutex::new(RefCell::new(LedType::Green));
static IDLE_TIME: AtomicU16 = AtomicU16::new(0);

#[embassy_executor::task]
async fn led_task(mut green_led: Output<'static, AnyPin>, mut blue_led: Output<'static, AnyPin>) {
    info!("Led Task");

    loop {
        {
            let led_ref = LED.lock().await;

            let led = led_ref.borrow();

            match led.deref() {
                LedType::Green => {
                    blue_led.set_low();
                    green_led.toggle()
                }
                LedType::Blue => {
                    green_led.set_low();
                    blue_led.toggle()
                }
            }
        }

        Timer::after_millis(IDLE_TIME.load(Ordering::Relaxed) as u64).await;
    }
}

#[embassy_executor::task]
async fn potentiometer_task(mut potentiometer: PC0, adc: ADC) {
    info!("Potentiometer Task");

    let mut delay = Delay;
    let mut converter = Adc::new(adc, Irqs, &mut delay);

    converter.set_sample_time(SampleTime::Cycles71_5);

    loop {
        let value = converter.read(&mut potentiometer).await;
        IDLE_TIME.store(value, Ordering::Relaxed);
        Timer::after_millis(100).await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {

    info!("Init");
    let peripherals = embassy_stm32::init(Default::default());

    let green_led = Output::new(peripherals.PC9.degrade(), Level::High, Speed::Low);
    let blue_led = Output::new(peripherals.PC8.degrade(), Level::Low, Speed::Low);

    let mut button = ExtiInput::new(Input::new(peripherals.PA0, Pull::None), peripherals.EXTI0);

    let adc: ADC = peripherals.ADC;

    spawner.spawn(led_task(green_led, blue_led)).unwrap();
    spawner.spawn(potentiometer_task(peripherals.PC0, adc)).unwrap();

    let mut led_type = LedType::Green;
    loop {
        button.wait_for_rising_edge().await;
        led_type.toggle();
        info!("Set {}", led_type);
        set_led(led_type).await;
        button.wait_for_falling_edge().await;
    }
}

async fn set_led(led_type: LedType) {
    let led_ref = LED.lock().await;
    led_ref.replace(led_type);
}

//PC0 ADC_IN10