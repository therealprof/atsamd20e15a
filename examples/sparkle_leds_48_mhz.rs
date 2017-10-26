#![feature(used)]
#![no_std]

#[macro_use(exception, interrupt)]
extern crate atsamd20e15a;
extern crate cortex_m;

use atsamd20e15a::{PORT, SYST, init_48_mhz_clock, setup_tc0};
use cortex_m::interrupt;
use cortex_m::peripheral::SystClkSource;


static mut PWM_STATE: [u8; 19] = [0; 19];


fn main() {
    for _ in 0..1_000_000 {
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

        for i in 0..19 {
            unsafe {
                if PWM_STATE[i] != 0 {
                    PWM_STATE[i] -= 1;
                }
            }
        }

        if l.time % 32 == 0{
            /* Use PRBS20 to generate next LED sequence */
            let a = l.rand;
            let newbit = ((a >> 19) ^ (a >> 2)) & 1;
            l.rand = ((a << 1) | newbit) & 1048575;
            for i in 0..19 {
                if (l.rand & (1 << i)) != 0 {
                    unsafe {
                        let mut value: u16 = u16::from(PWM_STATE[i]) + 48;
                        PWM_STATE[i] = if value > 255 { 255 } else { value as u8 };
                    }
                }
            }
        }
    });
}


interrupt!(TC0, fade, locals: {
    time: u8 = 0;
});


fn led_to_pinbit(l: usize) -> u32 {
    match l {
        0 => 1 << 0,
        1 => 1 << 1,
        2 => 1 << 2,
        3 => 1 << 3,
        4 => 1 << 4,
        5 => 1 << 5,
        6 => 1 << 6,
        7 => 1 << 7,
        8 => 1 << 8,
        9 => 1 << 9,
        10 => 1 << 10,
        11 => 1 << 11,
        12 => 1 << 24,
        13 => 1 << 23,
        14 => 1 << 22,
        15 => 1 << 19,
        16 => 1 << 18,
        17 => 1 << 17,
        18 => 1 << 16,
        _ => 1 << 14,
    }
}


fn fade(l: &mut TC0::Locals) {
    /* Enter critical section */
    cortex_m::interrupt::free(|cs| {
        let port = PORT.borrow(cs);
        let tc0 = atsamd20e15a::TC0.borrow(cs);
        tc0.intflag.write(|w| w.ovf().set_bit().err().set_bit());

        l.time -= 1;
        let mut newstate: u32 = 0;

        for i in 0..19 {
            if l.time < unsafe { PWM_STATE[i] } {
                newstate |= led_to_pinbit(i);
            }
        }

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
