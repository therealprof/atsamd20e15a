#![feature(used)]
#![no_std]

extern crate panic_abort;

#[macro_use(exception, interrupt)]
extern crate atsamd20e15a;
extern crate cortex_m;

use atsamd20e15a::{init_48_mhz_clock, setup_tc0, PORT, SYST};
use cortex_m::interrupt;
use cortex_m::peripheral::SystClkSource;

use atsamd20e15a::snowflake;

fn main() {
    for _ in 0..200_000 {
        cortex_m::asm::nop();
    }

    /* Initialise clock, has its own critical section */
    init_48_mhz_clock();

    /* Enter critical section */
    interrupt::free(|cs| {
        let port = PORT.borrow(cs);
        let syst = SYST.borrow(cs);

        /* Initialise PA0-PA24 */
        port.outset
            .modify(|_, w| unsafe { w.outset().bits(0x1FF_FFFF) });
        port.dir.modify(|_, w| unsafe { w.dir().bits(0x1FF_FFFF) });

        /* Initialise SysTick counter with a defined value */
        unsafe { syst.cvr.write(1) };

        /* Set source for SysTick counter, here full operating frequency (== 8MHz) */
        syst.set_clock_source(SystClkSource::Core);

        /* Set reload value, i.e. timer delay (== 1/96s) */
        syst.set_reload(500_000);

        /* Start counter */
        syst.enable_counter();

        /* Start interrupt generation */
        syst.enable_interrupt();
    });

    /* Set timer to fire every 480kHz */
    setup_tc0(100);

    /* Initialise a few LEDs with a gradient */
    let leds = snowflake::snowflake_leds();
    leds[0].set(255);
    leds[1].set(127);
    leds[2].set(15);
    leds[3].set(7);
    leds[4].set(1);
}

/* Define an exception, i.e. function to call when exception occurs. Here our SysTick timer
 * trips the running function */
exception!(SYS_TICK, running, locals: {
    time: u8 = 0;
});

fn running(l: &mut SYS_TICK::Locals) {
    l.time -= 1;

    let leds = &mut snowflake::snowflake_leds();

    /* Rotate LED values in one direction for a few rounds, then the other */
    if l.time < 127 {
        leds.rshift(1);
    } else {
        leds.lshift(1);
    }

    /* Recalculate PWM values */
    snowflake::pwmcache().calculate_perceived(leds);
}

/* Define an interrupt handler, i.e. function to call when the specific interrupt occurs. Here our
 * timer to handle the PWM trips the fade function */
interrupt!(TC0, fade_handler, locals: {
    time: u8 = 0;
});

/* Place function into RAM to avoid flash wait states */
#[link_section = ".data"]
#[inline(never)]
/* Apply the current LED intensity of all LEDs */
fn fade(time: u8) -> u8 {
    /* Enter critical section */
    cortex_m::interrupt::free(|cs| {
        let port = PORT.borrow(cs);
        let tc0 = atsamd20e15a::TC0.borrow(cs);
        tc0.intflag.write(|w| w.ovf().set_bit().err().set_bit());

        /* Retrieve PWM values for current time */
        let newstate = snowflake::pwmcache()[time];

        /* Enable LEDs */
        port.outclr
            .modify(|_, w| unsafe { w.outclr().bits(newstate) });

        /* Disable LEDs */
        port.outset
            .modify(|_, w| unsafe { w.outset().bits(!newstate) });
    });

    time - 1
}

/* The interrupt handler to call our main fade function residing in RAM */
fn fade_handler(l: &mut TC0::Locals) {
    /* Call into handler placed in RAM to avoid flash wait states */
    l.time = fade(l.time);
}
