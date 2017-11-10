use core::ops::{Index, IndexMut};
use core::slice;


pub struct PWMCache {
    bitmask: [u32; 256],
}


impl PWMCache {
    pub const fn new() -> PWMCache {
        PWMCache { bitmask: [0; 256] }
    }

    pub fn calculate(&mut self, leds: &LEDs) {
        let mut pop: [bool; 256] = [false; 256];

        leds.into_iter().for_each(
            |l| pop[l.pwm_state as usize] = true,
        );

        let mut bitmask = 0;
        for i in 0..256 {
            if pop[i] {
                bitmask = leds.into_iter().fold(0, |a, l| if (i as u8) < l.pwm_state {
                    a | l.pos
                } else {
                    a
                });
            }

            self.bitmask[i as usize] = bitmask;
        }
    }
}


pub fn pwmcache() -> &'static mut PWMCache {
    static mut SINGLETON: PWMCache = PWMCache::new();
    unsafe { &mut SINGLETON }
}


impl Index<u8> for PWMCache {
    type Output = u32;

    fn index(&self, i: u8) -> &Self::Output {
        &self.bitmask[i as usize]
    }
}


pub struct LED {
    pwm_state: u8,
    pos: u32,
}


impl LED {
    pub const fn new(pos: u32) -> LED {
        LED {
            pwm_state: 0,
            pos: pos,
        }
    }

    pub fn set(&mut self, pwm: u8) {
        self.pwm_state = pwm;
    }

    pub fn get(&self) -> u8 {
        self.pwm_state
    }
}


pub struct LEDs {
    leds: [LED; 19],
}


pub fn leds() -> &'static mut LEDs {
    static mut SINGLETON: LEDs = LEDs::new();
    unsafe { &mut SINGLETON }
}


impl<'a> IntoIterator for &'a LEDs {
    type Item = &'a LED;
    type IntoIter = slice::Iter<'a, LED>;

    fn into_iter(self) -> Self::IntoIter {
        self.leds.iter()
    }
}


impl Index<usize> for LEDs {
    type Output = LED;

    fn index(&self, i: usize) -> &Self::Output {
        &self.leds[i]
    }
}


impl IndexMut<usize> for LEDs {
    fn index_mut(&mut self, i: usize) -> &mut LED {
        &mut self.leds[i]
    }
}


impl LEDs {
    pub const fn new() -> LEDs {
        LEDs {
            leds: [
                LED::new(1 << 0),
                LED::new(1 << 1),
                LED::new(1 << 2),
                LED::new(1 << 3),
                LED::new(1 << 4),
                LED::new(1 << 5),
                LED::new(1 << 6),
                LED::new(1 << 7),
                LED::new(1 << 8),
                LED::new(1 << 9),
                LED::new(1 << 10),
                LED::new(1 << 11),
                LED::new(1 << 24),
                LED::new(1 << 23),
                LED::new(1 << 22),
                LED::new(1 << 19),
                LED::new(1 << 18),
                LED::new(1 << 17),
                LED::new(1 << 16),
            ],
        }
    }

    /* Saturated addition of constant to all LED PWM values */
    pub fn adds(&mut self, other: u8) {
        for i in &mut self.leds {
            i.pwm_state = if u16::from(i.pwm_state) + u16::from(other) > 255 {
                255
            } else {
                i.pwm_state + other
            };
        }
    }

    /* Overflowing addition of constant to all LED PWM values */
    pub fn add(&mut self, other: u8) {
        for i in &mut self.leds {
            i.pwm_state += other
        }
    }

    /* Saturated substraction of constant from all LED PWM values */
    pub fn subs(&mut self, other: u8) {
        for i in &mut self.leds {
            i.pwm_state = if i16::from(i.pwm_state) - i16::from(other) < 0 {
                0
            } else {
                i.pwm_state - other
            };
        }
    }

    /* Underflowing substraction of constant from all LED PWM values */
    pub fn sub(&mut self, other: u8) {
        for i in &mut self.leds {
            i.pwm_state -= other
        }
    }

    /* Shift clockwise, i.e. left */
    pub fn lshift(&mut self, amount: usize) {
        for _ in 0..amount {
            let temp = self[18].pwm_state;
            for i in 0..18 {
                self[18 - i].pwm_state = self[17 - i].pwm_state;
            }
            self[0].pwm_state = temp;
        }
    }

    /* Shift counter-clockwise, i.e. right */
    pub fn rshift(&mut self, amount: usize) {
        for _ in 0..amount {
            let temp = self[0].pwm_state;
            for i in 0..18 {
                self[i].pwm_state = self[i + 1].pwm_state;
            }
            self[18].pwm_state = temp;
        }
    }

    pub fn get_over_bitmask(&self, value: u8) -> u32 {
        self.into_iter().fold(0, |a, l| if value < l.pwm_state {
            a | l.pos
        } else {
            a
        })
    }
}
