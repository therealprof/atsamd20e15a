#![feature(used)]
#![no_std]

extern crate cortex_m;
extern crate atsamd20e15a;

use atsamd20e15a::{PORT, SYSCTRL};
use cortex_m::interrupt;

fn main() {
    interrupt::free(|cs| {
        let port = PORT.borrow(cs);
        let sysctrl = SYSCTRL.borrow(cs);

        /* Use unscaled system oscillator (i.e. full 8MHz) */
        sysctrl.osc8m.write(|w| unsafe { w.presc().bits(0) });

        /* Initialise PA0 */
        port.outset.modify(|_, w| unsafe { w.outset().bits(1) });
        port.dir.modify(|_, w| unsafe { w.dir().bits(1) });

        loop {
            /* Turn PA0 on a million times in a row */
            for _ in 0..1_000_000 {
                port.outclr.modify(|_, w| unsafe { w.outclr().bits(1) });
            }
            /* Then turn PA0 off a million times in a row */
            for _ in 0..1_000_000 {
                port.outset.modify(|_, w| unsafe { w.outset().bits(1) });
            }
        }
    });
}
