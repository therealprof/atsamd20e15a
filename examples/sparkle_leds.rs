#![feature(used)]
#![no_std]

#[macro_use(exception, interrupt)]
extern crate atsamd20e15a;
extern crate cortex_m;

use atsamd20e15a::{
    delay_init, init_48_mhz_clock, init_gpios, init_systick, pull_pins_high, pull_pins_low,
    setup_tc0, snowflake,
};

/* If set to true, enables a high edge on data out pin during PWM value calculation for measurement
 * via oscilloscope */
const DEBUG: bool = false;

fn main() {
    /* ATSAMD is bitchy, let's delay a bit so we can attach with a debugger if we need to */
    delay_init();

    /* Initialise clock, has its own critical section */
    init_48_mhz_clock();

    /* Initialise the used GPIOs */
    init_gpios();

    /* Initialise the SysTick timer and exception */
    init_systick(4_000_000);

    /* Set timer to fire every 480kHz */
    setup_tc0(100);

    let leds = snowflake::snowflake_leds();
    leds.set(255);
}

/* Define an exception, i.e. function to call when exception occurs. Here our SysTick timer
 * trips the sparkle function */
exception!(SYS_TICK, sparkle, locals: {
    rand: u32 = 2;
    time: u8 = 0;
});

fn sparkle(l: &mut SYS_TICK::Locals) {
    let leds = &mut snowflake::snowflake_leds();

    if DEBUG {
        pull_pins_high(snowflake::DATAOUT);
    }

    l.time -= 1;

    /* Use PRBS20 to generate next LED sequence */
    let a = l.rand;
    let newbit = ((a >> 19) ^ (a >> 2)) & 1;
    let newrand = ((a << 1) | newbit) & 1_048_575;
    for (i, _) in snowflake::snowflake_leds().into_iter().enumerate() {
        if l.time & 2 == 2 {
            l.rand = newrand;
        }
        if (l.rand & (1 << i)) != 0 {
            leds[i].add(15);
        }
    }
    leds.subs(18);
    leds.add(10);
    for (i, _) in snowflake::snowflake_leds().into_iter().enumerate() {
        if leds[i].get() < 12 {
            leds[i].set(12);
        }
    }

    snowflake::pwmcache().calculate_perceived(leds);

    if DEBUG {
        pull_pins_low(snowflake::DATAOUT);
    }
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
        let tc0 = atsamd20e15a::TC0.borrow(cs);
        tc0.intflag.write(|w| w.ovf().set_bit().err().set_bit());
    });

    /* Enable LEDs */
    pull_pins_low(snowflake::pwmcache().get_clear_bits(time));

    /* Disable LEDs */
    pull_pins_high(snowflake::pwmcache().get_set_bits(time));

    time - 1
}

/* The interrupt handler to call our main fade function residing in RAM */
fn fade_handler(l: &mut TC0::Locals) {
    /* Call into handler placed in RAM to avoid flash wait states */
    l.time = fade(l.time);
}
