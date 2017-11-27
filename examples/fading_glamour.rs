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

    /* Setup timer interrupt with 185kHz frequency */
    setup_tc0(259);
}


/* Define an exception handler, i.e. function to call when the specific exception occurs. Here our SysTick timer
 * trips the running function */
exception!(SYS_TICK, running);

/* Circle LEDs and let them fade out */
fn running() {
    let leds = &mut snowflake::proto_leds();

    /* Fade out */
    leds.subs(1);

    /* Rotate LED values, skipping a few positions */
    leds.lshift(5);

    /* Recalculate PWM values */
    snowflake::pwmcache().calculate(leds);
}


/* Define an interrupt handler, i.e. function to call when the specific interrupt occurs. Here our
 * input pin PA25 is connected to the external interrupt EXTINT13 and trips the glow function */
interrupt!(EIC, glow);

/* Light up the the first LED when triggered */
fn glow() {
    cortex_m::interrupt::free(|cs| {
        let eic = atsamd20e15a::EIC.borrow(cs);
        eic.intflag.modify(|_, w| w.extint13().set_bit());
    });

    let leds = &mut snowflake::proto_leds();

    leds[0].set(255);

    /* Recalculate PWM values */
    snowflake::pwmcache().calculate(leds);
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
        port.outclr.modify(
            |_, w| unsafe { w.outclr().bits(newstate) },
        );

        /* Disable LEDs */
        port.outset.modify(
            |_, w| unsafe { w.outset().bits(!newstate) },
        );
    });

    time - 1
}


/* The interrupt handler to call our main fade function residing in RAM */
fn fade_handler(l: &mut TC0::Locals) {
    /* Call into handler placed in RAM to avoid flash wait states */
    l.time = fade(l.time);
}
