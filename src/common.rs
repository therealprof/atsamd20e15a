use super::{PORT, SYST, SYSCTRL, GCLK, NVMCTRL, PM, TC0, NVIC, EIC, Interrupt};

extern crate cortex_m;

use cortex_m::interrupt;
use cortex_m::peripheral::SystClkSource;
use core::ptr;


pub fn delay_init()
{
    for _ in 0..200_000 {
        cortex_m::asm::nop();
    }
}


pub fn init_gpios()
{
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

        /* Set SysTick exception to lowest priority */
        unsafe { ptr::write_volatile(0xE000_ED20 as *mut u32, 0xC000_0000) }

        /* Initialise SysTick counter with a defined value */
        unsafe { syst.cvr.write(1) };

        /* Set source for SysTick counter, here full operating frequency (== 8MHz) */
        syst.set_clock_source(SystClkSource::Core);

        /* Set reload value, i.e. timer delay (== 1/24s) */
        syst.set_reload(4_000_000);

        /* Start counter */
        syst.enable_counter();

        /* Start interrupt generation */
        syst.enable_interrupt();
    });
}


pub fn init_systick(reload : u32)
{
    /* Enter critical section */
    interrupt::free(|cs| {
        let syst = SYST.borrow(cs);

        /* Set SysTick exception to lowest priority */
        unsafe { ptr::write_volatile(0xE000_ED20 as *mut u32, 0xC000_0000) }

        /* Initialise SysTick counter with a defined value */
        unsafe { syst.cvr.write(1) };

        /* Set source for SysTick counter, here full operating frequency (== 8MHz) */
        syst.set_clock_source(SystClkSource::Core);

        /* Set reload value, i.e. timer delay (== 1/24s) */
        syst.set_reload(reload);

        /* Start counter */
        syst.enable_counter();

        /* Start interrupt generation */
        syst.enable_interrupt();
    });
}


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


pub fn setup_tc0(divider: u16) {
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

        /* Enable EXTI IRQs, set prio 0 and clear any pending IRQs */
        nvic.enable(Interrupt::TC0);
        unsafe { nvic.set_priority(Interrupt::TC0, 0) };
        nvic.clear_pending(Interrupt::TC0);

        /* Enable clock for TC0 */
        pm.apbcmask.modify(|_, w| w.tc0().set_bit());

        tc0.ctrla.modify(|_, w| {
            w.mode().count16().prescaler().div8().wavegen().mfrq()
        });

        /* And wait */
        while tc0.status.read().syncbusy().bit_is_set() {}

        /* Make timer autorestart */
        tc0.ctrlbset.write(
            |w| w.oneshot().clear_bit().cmd().retrigger(),
        );

        /* And wait */
        while tc0.status.read().syncbusy().bit_is_set() {}

        /* Setup divider */
        tc0.cc[0].write(|w| unsafe { w.cc().bits(divider) });

        /* And wait */
        while tc0.status.read().syncbusy().bit_is_set() {}

        /* Set interrupt to trigger on overflow */
        tc0.intenset.write(|w| w.ovf().set_bit());

        /* And wait */
        while tc0.status.read().syncbusy().bit_is_set() {}

        /* Enable  */
        tc0.ctrla.modify(|_, w| w.enable().set_bit());

        /* And wait */
        while tc0.status.read().syncbusy().bit_is_set() {}
    });
}


/* Setup EIC and PA25 to register external interrupts */
pub fn setup_eic() {
    interrupt::free(|cs| {
        let eic = EIC.borrow(cs);
        let pm = PM.borrow(cs);
        let gclk = GCLK.borrow(cs);
        let nvic = NVIC.borrow(cs);

        /* Enable clock for EIC */
        pm.apbamask.modify(|_, w| w.eic().set_bit());

        /* Set up clock generator 0 as input for EIC */
        gclk.clkctrl.write(
            |w| w.clken().set_bit().gen().gclk0().id().eic(),
        );

        /* How about we wait? */
        while gclk.status.read().syncbusy().bit_is_set() {}

        /* Reset the EIC */
        eic.ctrl.modify(|_, w| w.swrst().set_bit());

        /* More waiting... */
        while eic.ctrl.read().swrst().bit_is_set() {}
        while eic.status.read().syncbusy().bit_is_set() {}

        /* Configure PA25/EXTINT13 to register rising edges */
        eic.config[1].modify(|_, w| w.sense5().rise().filten5().set_bit());

        /* More waiting... */
        while eic.status.read().syncbusy().bit_is_set() {}

        /* Far more waiting... */
        while eic.status.read().syncbusy().bit_is_set() {}

        /* Clear EXTINT13 interrupt*/
        eic.intflag.write(|w| w.extint13().set_bit());

        /* Enable EXTINT13 */
        eic.intenset.write(|w| w.extint13().set_bit());

        /* Even nore waiting... */
        while eic.status.read().syncbusy().bit_is_set() {}

        /* Enable EIC */
        eic.ctrl.modify(|_, w| w.enable().set_bit());

        /* And yet more waiting... */
        while eic.status.read().syncbusy().bit_is_set() {}

        /* Enable EXTI IRQs, set prio 2 and clear any pending IRQs */
        nvic.enable(Interrupt::EIC);
        unsafe { nvic.set_priority(Interrupt::EIC, 2) };
        nvic.clear_pending(Interrupt::EIC);
    });
}
