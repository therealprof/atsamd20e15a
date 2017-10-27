use core::ops::{Index, IndexMut};


pub struct LEDs {
    pwm_state: [u8; 19],
}

impl Index<usize> for LEDs {
    type Output = u8;

    fn index(&self, i: usize) -> &u8 {
        &self.pwm_state[i]
    }
}


impl IndexMut<usize> for LEDs {
    fn index_mut(&mut self, i: usize) -> &mut u8 {
        &mut self.pwm_state[i]
    }
}


impl LEDs {
    pub const fn new () -> LEDs {
        LEDs {
            pwm_state: [0; 19],
        }
    }

    fn led_to_pinbit(&self, l: usize) -> u32 {
        match l {
            0 => 1,
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

    pub fn add(&mut self, other: u8) {
        for i in &mut self.pwm_state {
            *i = if u16::from(*i) + u16::from(other) > 255 {
                255
            } else {
                *i + other
            };
        }
    }

    pub fn sub(&mut self, other: u8) {
        for i in &mut self.pwm_state {
            *i = if i16::from(*i) - i16::from(other) < 0 {
                0
            } else {
                *i - other
            };
        }
    }

    pub fn get_over_bitmask(&self, value: u8) -> u32 {
        let mut mask = 0;

        for i in 0..19 {
            if value < self.pwm_state[i] {
                mask |= self.led_to_pinbit(i);
            }
        }

        mask
    }
}
