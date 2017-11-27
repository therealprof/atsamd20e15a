#![feature(used)]
#![no_std]

#[macro_use(exception, interrupt)]
extern crate atsamd20e15a;
extern crate cortex_m;

use atsamd20e15a::{init_48_mhz_clock, setup_tc0, delay_init, init_gpios, init_systick, snowflake,
                   pull_pins_low, pull_pins_high};

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

    /* Setup timer interrupt with 240kHz frequency */
    setup_tc0(200);

    /* Initialise an LED gradient using the values of a sine wave */
    let leds = snowflake::proto_leds();
    for i in 1..19 {
        leds[i].set(snowflake::PWMSINE[i]);
    }

    /* Override the first value so we don't have a black dot */
    leds[0].set(14);

    /* Recalculate PWM values */
    snowflake::pwmcache().calculate_perceived(leds);
}


/* Define an exception handler, i.e. function to call when the specific exception occurs. Here our SysTick timer
 * trips the running function */
exception!(SYS_TICK, running);


fn running() {
    let leds = &mut snowflake::proto_leds();

    /* Rotate LED values */
    leds.lshift(1);

    if DEBUG {
        pull_pins_high(snowflake::DATAOUT);
    }

    /* Recalculate PWM values */
    snowflake::pwmcache().calculate_perceived(leds);

    if DEBUG {
        pull_pins_low(snowflake::DATAOUT);
    }
}


/* Define an interrupt handler, i.e. function to call when the specific interrupt occurs. Here our
 * timer to handle the PWM trips the fade function */
interrupt!(TC0, pwm_handler, locals: {
    time: u8 = 0;
});


/* The interrupt handler to call our main fade function residing in RAM */
fn pwm_handler(l: &mut TC0::Locals) {
    /* Call into handler placed in RAM to avoid flash wait states */
    l.time = do_pwm(l.time);
}


/* Place function into RAM to avoid flash wait states */
#[link_section = ".data"]
#[inline(never)]
/* Apply the current LED intensity of all LEDs */
fn do_pwm(time: u8) -> u8 {
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
