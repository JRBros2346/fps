//! Blink example for "Blue Pill" (STM32F103) using stm32f1xx-hal
#![no_std]
#![no_main]

use cortex_m_rt::entry;
use panic_halt as _;
use stm32f1xx_hal::{gpio::PinState, pac, prelude::*};

mod fun;

#[entry]
fn main() -> ! {
    // Acquire device & core peripherals
    let dp = pac::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();

    // Configure clocks
    let mut flash = dp.FLASH.constrain();
    let rcc = dp.RCC.constrain();
    let clocks = rcc.cfgr.freeze(&mut flash.acr);

    // Configure PC13 (on-board LED) as push-pull output
    let mut gpioc = dp.GPIOC.split();
    let mut led = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);

    // SYST-based delay driver
    let mut delay = cp.SYST.delay(&clocks);

    // Blink loop
    let mut i = 0;
    loop {
        led.set_state(if fun::r#fn(i) {
            PinState::High
        } else {
            PinState::Low
        });
        delay.delay_ms(500_u16);
        i += 1;
    }
}
