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
    for _ in 0..500_000 {
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

        /* Set reload value, i.e. timer delay (== 100ms) */
        syst.set_reload(480_000);

        /* Start counter */
        syst.enable_counter();

        /* Start interrupt generation */
        syst.enable_interrupt();
    });

    setup_tc0();
}


/* Define an exception, i.e. function to call when exception occurs. Here our SysTick timer
 * trips the sparkle function */
exception!(SYS_TICK, sparkle, locals: {
    rand: u32 = 2;
    time: u8 = 0;
});


fn sparkle(l: &mut SYS_TICK::Locals) {
    /* Enter critical section */
    cortex_m::interrupt::free(|_cs| {
        l.time -= 1;

        unsafe {
            LEDS.sub(1);
        }

        if l.time % 32 == 0 {
            /* Use PRBS20 to generate next LED sequence */
            let a = l.rand;
            let newbit = ((a >> 19) ^ (a >> 2)) & 1;
            l.rand = ((a << 1) | newbit) & 1_048_575;
            unsafe {
                for (i, item) in LEDS.into_iter().enumerate() {
                    if (l.rand & (1 << i)) != 0 {
                        let mut value: u16 = u16::from(*item) + 48;
                        LEDS[i] = if value > 255 { 255 } else { value as u8 };
                    }
                }
            }
        }
    });
}


interrupt!(TC0, fade, locals: {
    time: u8 = 0;
});


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
