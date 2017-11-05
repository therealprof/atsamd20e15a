#![feature(used)]
#![no_std]

#[macro_use(exception, interrupt)]
extern crate atsamd20e15a;
extern crate cortex_m;

use atsamd20e15a::{PORT, SYST, init_48_mhz_clock, setup_tc0};
use cortex_m::interrupt;
use cortex_m::peripheral::SystClkSource;

use atsamd20e15a::snowflake;


static mut LEDS: snowflake::LEDs = snowflake::LEDs::new();


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

        /* Set reload value, i.e. timer delay (== 1/12s) */
        syst.set_reload(4_000_000);

        /* Start counter */
        syst.enable_counter();

        /* Start interrupt generation */
        syst.enable_interrupt();
    });

    /* Setup timer interrupt with 480kHz frequency */
    setup_tc0(100);

    /* Initialise an LED gradient */
    unsafe {
        LEDS[0] = 255;
        LEDS[1] = 230;
        LEDS[2] = 210;
        LEDS[3] = 190;
        LEDS[4] = 170;
        LEDS[5] = 150;
        LEDS[6] = 130;
        LEDS[7] = 110;
        LEDS[8] = 90;
        LEDS[9] = 70;
        LEDS[10] = 50;
        LEDS[11] = 30;
        LEDS[12] = 10;
        LEDS[13] = 1;
    }
}


/* Define an exception handler, i.e. function to call when the specific exception occurs. Here our SysTick timer
 * trips the running function */
exception!(SYS_TICK, running);


fn running() {
    unsafe {
        /* Rotate LED values */
        LEDS.rshift(1);
    }
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
        let newstate = unsafe { LEDS.get_over_bitmask(l.time) };

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
