#![feature(used)]
#![no_std]

#[macro_use(exception, interrupt)]
extern crate atsamd20e15a;
extern crate cortex_m;

use atsamd20e15a::{PORT, SYST, init_48_mhz_clock, setup_tc0, setup_eic};
use cortex_m::interrupt;
use cortex_m::peripheral::SystClkSource;

use atsamd20e15a::snowflake;


fn main() {
    for _ in 0..200_000 {
        cortex_m::asm::nop();
    }

    /* Initialise clock, has its own critical section */
    init_48_mhz_clock();

    /* Initialise EIC and EXTINT13 for PA25 */
    setup_eic();

    /* Enter critical section */
    interrupt::free(|cs| {
        let port = PORT.borrow(cs);
        let syst = SYST.borrow(cs);

        /* Initialise PA0-PA24 to high */
        port.outset.write(
            |w| unsafe { w.outset().bits(0x1FF_FFFF) },
        );

        /* Set PA0-PA24 as output */
        port.dir.write(|w| unsafe { w.dir().bits(0x1FF_FFFF) });

        /* Set PA25 to input with pull-up and external interrupt enabled */
        port.pincfg[25].modify(|_, w| {
            w.inen().set_bit().pullen().set_bit().pmuxen().set_bit()
        });

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

    /* Setup timer interrupt with 480kHz frequency */
    setup_tc0(2599);

    /* Initialise an LED to full brightness to get started */
    snowflake::leds()[0].set(255);
}


/* Define an exception handler, i.e. function to call when the specific exception occurs. Here our SysTick timer
 * trips the running function */
exception!(SYS_TICK, running);

/* Circle LEDs and let them fade out */
fn running() {
    snowflake::leds().subs(1);

    /* Rotate LED values, skipping a few positions */
    snowflake::leds().lshift(4);
}


/* Define an interrupt handler, i.e. function to call when the specific interrupt occurs. Here our
 * input pin PA25 is connected to the external interrupt EXTINT13 and trips the glow function */
interrupt!(EIC, glow);

/* Light up the the first LED when triggered */
fn glow() {
    cortex_m::interrupt::free(|cs| {
        let eic = atsamd20e15a::EIC.borrow(cs);
        eic.intflag.modify(|_, w| w.extint13().set_bit());
        snowflake::leds()[0].set(255);
    });
}


/* Define an interrupt handler, i.e. function to call when the specific interrupt occurs. Here our
 * timer to handle the PWM trips the fade function */
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
