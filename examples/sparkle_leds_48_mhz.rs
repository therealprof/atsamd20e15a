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

        /* Set reload value, i.e. timer delay (== 1/12s) */
        syst.set_reload(4_000_000);

        /* Start counter */
        syst.enable_counter();

        /* Start interrupt generation */
        syst.enable_interrupt();
    });

    /* Set timer to fire every 480kHz */
    setup_tc0(100);
}


/* Define an exception, i.e. function to call when exception occurs. Here our SysTick timer
 * trips the sparkle function */
exception!(SYS_TICK, sparkle, locals: {
    rand: u32 = 2;
    time: u8 = 0;
});


fn sparkle(l: &mut SYS_TICK::Locals) {
    let leds = &mut snowflake::leds();

    /* Enter critical section */
    l.time -= 1;

    /* Use PRBS20 to generate next LED sequence */
    let a = l.rand;
    let newbit = ((a >> 19) ^ (a >> 2)) & 1;
    let newrand = ((a << 1) | newbit) & 1_048_575;
    for (i, item) in snowflake::leds().into_iter().enumerate() {
        if l.time & 2 == 2 {
            l.rand = newrand;
        }
        if (l.rand & (1 << i)) != 0 && l.time & 4 == 4 {
            let mut value: u16 = u16::from(item.get()) + 15;
            leds[i].set(if value > 255 { 255 } else { value as u8 });
        } else {
            let value = leds[i].get();
            if value > 9 {
                leds[i].set(value - 8);
            }
        }
    }

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
