use super::{SYSCTRL, GCLK, NVMCTRL, PM, TC0, NVIC, Interrupt};
use cortex_m::interrupt;

pub fn init_48_mhz_clock() {
    interrupt::free(|cs| {
        let sysctrl = SYSCTRL.borrow(cs);
        let gclk = GCLK.borrow(cs);
        let nvmctrl = NVMCTRL.borrow(cs);

        /* Set wait states */
        nvmctrl.ctrlb.modify(|_, w| unsafe { w.rws().bits(1) });

        /* Reset generic clock controller for good measure */
        gclk.ctrl.modify(|_, w| w.swrst().bit(true));

        /* Wait until it is reset */
        while gclk.ctrl.read().swrst().bit_is_set() && gclk.status.read().syncbusy().bit_is_set() {}

        /* Disable on-demand mode of 8 MHz oscillator */
        sysctrl.osc8m.modify(|_, w| w.ondemand().clear_bit());

        /* Set up divisor on clock generator 3 */
        gclk.gendiv.write(
            |w| unsafe { w.div().bits(64).id().bits(3) },
        );

        /* Set up 8 MHz clock as source for clock generator 3 */
        gclk.genctrl.write(|w| unsafe {
            w.id().bits(3).genen().set_bit().src().osc8m()
        });

        /* Wait, again... */
        while gclk.status.read().syncbusy().bit_is_set() {}

        /* Set up clock generator 3 as input for DFLL */
        gclk.clkctrl.write(|w| unsafe {
            w.clken().set_bit().gen().bits(3).id().dfll48m()
        });

        /* Wait, again... */
        while gclk.status.read().syncbusy().bit_is_set() {}

        /* Disable on-demand mode of DFLL */
        sysctrl.dfllctrl.modify(|_, w| w.ondemand().clear_bit());

        /* Wait, again... */
        while sysctrl.pclksr.read().dfllrdy().bit_is_clear() {}

        /* Set multiplicator for DFLL */
        sysctrl.dfllmul.write(|w| unsafe {
            w.cstep().bits(1).fstep().bits(1).mul().bits(3072)
        });

        /* Wait, again... */
        while sysctrl.pclksr.read().dfllrdy().bit_is_clear() {}

        /* Disable quick lock and enable open-loop mode */
        sysctrl.dfllctrl.modify(
            |_, w| w.mode().set_bit().qldis().set_bit(),
        );

        /* Wait, again... */
        while sysctrl.pclksr.read().dfllrdy().bit_is_clear() {}

        /* Enable DFLL fine and coarse lock and clean interrupt */
        sysctrl.intflag.modify(|_, w| {
            w.dflllckc()
                .set_bit()
                .dflllckf()
                .set_bit()
                .dfllrdy()
                .set_bit()
        });

        /* Fire up DFLL */
        sysctrl.dfllctrl.modify(|_, w| w.enable().set_bit());

        /* Wait, again... */
        while sysctrl.pclksr.read().dfllrdy().bit_is_clear() {}

        /* Wait for the DFLL to lock  */
        while sysctrl.intflag.read().dflllckc().bit_is_clear() &&
            sysctrl.intflag.read().dflllckf().bit_is_clear()
        {}

        /* Wait, again... */
        while sysctrl.intflag.read().dfllrdy().bit_is_clear() {}

        /* Set up clock generator 0 (== CPU clock) without divisor */
        gclk.gendiv.write(
            |w| unsafe { w.div().bits(0).id().bits(0) },
        );

        /* Wait, again... */
        while gclk.status.read().syncbusy().bit_is_set() {}

        /* Set up clock generator 0 (== CPU clock) from DFLL source */
        gclk.genctrl.write(|w| unsafe {
            w.id()
                .bits(0)
                .genen()
                .set_bit()
                .src()
                .dfll48m()
                .idc()
                .set_bit()
        });

        /* Wait, again... */
        while gclk.status.read().syncbusy().bit_is_set() {}
    });
}


pub fn setup_tc0() {
    interrupt::free(|cs| {
        let gclk = GCLK.borrow(cs);
        let pm = PM.borrow(cs);
        let tc0 = TC0.borrow(cs);
        let nvic = NVIC.borrow(cs);

        /* Setup CPU clock for TC0 and TC1 */
        gclk.clkctrl.write(|w| {
            w.clken().set_bit().gen().gclk0().id().tc0_tc1()
        });

        /* And wait */
        while gclk.status.read().syncbusy().bit_is_set() {}

        /* Enable EXTI IRQs, set prio 1 and clear any pending IRQs */
        nvic.enable(Interrupt::TC0);
        unsafe { nvic.set_priority(Interrupt::TC0, 1) };
        nvic.clear_pending(Interrupt::TC0);

        /* Enable clock for TC0 */
        pm.apbcmask.modify(|_, w| w.tc0().set_bit());

        tc0.ctrla.modify(|_, w| {
            w.mode().count16().prescaler().div8().wavegen().mfrq()
        });
        while tc0.status.read().syncbusy().bit_is_set() {}
        tc0.ctrlbset.write(
            |w| w.oneshot().clear_bit().cmd().retrigger(),
        );
        while tc0.status.read().syncbusy().bit_is_set() {}

        tc0.cc[0].write(|w| unsafe { w.cc().bits(375) });
        while tc0.status.read().syncbusy().bit_is_set() {}
        tc0.intenset.write(|w| w.ovf().set_bit());
        while tc0.status.read().syncbusy().bit_is_set() {}

        tc0.ctrla.modify(|_, w| w.enable().set_bit());
        while tc0.status.read().syncbusy().bit_is_set() {}
    });
}
