#![feature(used)]
#![no_std]

#[macro_use(exception)]
extern crate atsamd20e15a;
extern crate cortex_m;

use atsamd20e15a::{PORT, SYST, init_48_mhz_clock};
use cortex_m::interrupt;
use cortex_m::peripheral::SystClkSource;


fn main() {
    /* Initialise clock, has its own critical section */
    init_48_mhz_clock();

    /* Enter critical section */
    interrupt::free(|cs| {
        let port = PORT.borrow(cs);
        let syst = SYST.borrow(cs);

        /* Initialise PA0-P04 */
        port.outset.modify(|_, w| unsafe { w.outset().bits(0x1FF_FFFF) });
        port.dir.modify(|_, w| unsafe { w.dir().bits(0x1FF_FFFF) });

        /* Initialise SysTick counter with a defined value */
        unsafe { syst.cvr.write(1) };

        /* Set source for SysTick counter, here full operating frequency (== 8MHz) */
        syst.set_clock_source(SystClkSource::Core);

        /* Set reload value, i.e. timer delay (== 100ms) */
        syst.set_reload(4_800_000);

        /* Start counter */
        syst.enable_counter();

        /* Start interrupt generation */
        syst.enable_interrupt();

    });
}


/* Define an exception, i.e. function to call when exception occurs. Here our SysTick timer
 * trips the flicker function */
exception!(SYS_TICK, flicker, locals: {
    state: bool = false;
    rand: u32 = 2;
});


fn flicker(l: &mut SYS_TICK::Locals) {
    /* Enter critical section */
    cortex_m::interrupt::free(|cs| {
        let port = PORT.borrow(cs);

        /* If next state is true */
        if l.state {
            /* Enable LEDs */
            port.outclr.modify(
                |_, w| unsafe { w.outclr().bits(l.rand) },
            );

            /* And set next state to false */
            l.state = false;
        } else {
            /* Disable LEDs */
            port.outset.modify(
                |_, w| unsafe { w.outset().bits(l.rand) },
            );

            /* And set next state to false */
            l.state = true;

            /* Use PRBS31 to generate next LED sequence */
            let a = l.rand;
            let newbit = ((a >> 31) ^ (a >> 28)) & 1;
            l.rand = (a << 1) | newbit;
        }
    });
}