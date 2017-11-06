#![feature(used)]
#![no_std]

#[macro_use(exception, interrupt)]
extern crate atsamd20e15a;
extern crate cortex_m;

use atsamd20e15a::{PORT, SYST, init_48_mhz_clock, setup_tc0};
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
        port.outset.modify(
            |_, w| unsafe { w.outset().bits(0x1FF_FFFF) },
        );
        port.dir.modify(|_, w| unsafe { w.dir().bits(0x1FF_FFFF) });

        /* Initialise SysTick counter with a defined value */
        unsafe { syst.cvr.write(1) };

        /* Set source for SysTick counter, here full operating frequency (== 8MHz) */
        syst.set_clock_source(SystClkSource::Core);

        /* Set reload value, i.e. timer delay (== 1/48s) */
        syst.set_reload(1_000_000);

        /* Start counter */
        syst.enable_counter();

        /* Start interrupt generation */
        syst.enable_interrupt();
    });

    /* Set timer to fire every 480kHz */
    setup_tc0(100);

    /* Initialise a few LEDs with a gradient */
    let leds = snowflake::leds();
    leds[0] = 255;
    leds[1] = 127;
    leds[2] = 15;
    leds[3] = 7;
}


/* Define an exception, i.e. function to call when exception occurs. Here our SysTick timer
 * trips the running function */
exception!(SYS_TICK, running, locals: {
    time: u8 = 0;
});


fn running(l: &mut SYS_TICK::Locals) {
    l.time -= 1;

    /* Rotate LED values in one direction for a few rounds, then the other */
    if l.time < 127 {
        snowflake::leds().rshift(1);
    } else {
        snowflake::leds().lshift(1);
    }
}


interrupt!(TC0, fade, locals: {
    time: u8 = 0;
});


/* Apply the current LED intensity of all LEDs */
fn fade(l: &mut TC0::Locals) {
    /* Enter critical section */
    cortex_m::interrupt::free(|cs| {
        let port = PORT.borrow(cs);
        let tc0 = atsamd20e15a::TC0.borrow(cs);
        tc0.intflag.write(|w| w.ovf().set_bit().err().set_bit());

        l.time -= 1;
        let newstate = snowflake::leds().get_over_bitmask(l.time);

        /* Enable LEDs */
        port.outclr.modify(
            |_, w| unsafe { w.outclr().bits(newstate) },
        );

        /* Disable LEDs */
        port.outset.modify(
            |_, w| unsafe { w.outset().bits(!newstate) },
        );
    });
}
