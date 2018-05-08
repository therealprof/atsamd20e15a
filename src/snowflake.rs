use core::mem;
use core::ops::{Deref, DerefMut, Index, IndexMut};
use core::ptr;
use core::slice;

/* Note: constants are defined all the way at the bottom due to the space needy rust standard
 * formatting */

pub struct PWMCache {
    bitmask: [u32; 256],
}

impl PWMCache {
    pub const fn new() -> PWMCache {
        PWMCache { bitmask: [0; 256] }
    }

    /* Precalculate the bitmasks using for PWMing so we don't have to do that somewhat costly
     * operation on the fly in the interrupt handler.  This runs at an average of around
     * 129µs@48Mhz if all 19 LEDs are actively used */
    pub fn calculate(&mut self, leds: &LEDs) {
        let mut _state: [u8; 256] = [0; 256];
        _state[0] = 1;
        _state[255] = 1;

        for v in leds.into_iter() {
            _state[v.get() as usize] = 1;
        }

        let mut bitmask = 0;
        for (i, v) in _state.iter_mut().enumerate() {
            if *v == 1 {
                bitmask = leds.into_iter()
                    .enumerate()
                    .filter(|l| (i as u8) < l.1.get())
                    .fold(0, |a, l| a | leds.pos[l.0]);
            }
            unsafe { ptr::write(&mut self.bitmask[i], bitmask) };
        }
    }

    /* Pretty much the same as calculate() but scales the PWM values according to the perception of
     * the brightness to the human eye which usually yields a slightly more pleasing effect.
     * This runs at an average of around 130µs@48Mhz if all 19 LEDs are actively used */
    pub fn calculate_perceived(&mut self, leds: &LEDs) {
        let mut _state: [u8; 256] = [0; 256];
        _state[0] = 1;
        _state[255] = 1;

        for v in leds.into_iter() {
            _state[PWMPERC[v.get() as usize] as usize] = 1;
        }

        let mut bitmask = 0;
        for (i, v) in _state.iter_mut().enumerate() {
            if *v == 1 {
                bitmask = leds.into_iter()
                    .enumerate()
                    .filter(|l| (i as u8) < PWMPERC[l.1.get() as usize])
                    .fold(0, |a, l| a | leds.pos[l.0]);
            }
            unsafe { ptr::write(&mut self.bitmask[i], bitmask) };
        }
    }

    pub fn get_clear_bits(&self, time: u8) -> u32 {
        self.bitmask[time as usize]
    }

    pub fn get_set_bits(&self, time: u8) -> u32 {
        !self.bitmask[time as usize] & !(1 << 15)
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
}

impl LED {
    pub const fn new() -> LED {
        LED { pwm_state: 0 }
    }

    pub fn set(&mut self, pwm: u8) {
        self.pwm_state = pwm;
    }

    pub fn add(&mut self, value: u8) {
        self.pwm_state = self.pwm_state.saturating_add(value);
    }

    pub fn sub(&mut self, value: u8) {
        self.pwm_state = self.pwm_state.saturating_sub(value);
    }

    pub fn get(&self) -> u8 {
        self.pwm_state
    }
}

impl Deref for LED {
    type Target = u8;

    fn deref(&self) -> &Self::Target {
        &self.pwm_state
    }
}

impl DerefMut for LED {
    fn deref_mut(&mut self) -> &mut u8 {
        &mut self.pwm_state
    }
}

pub struct LEDs {
    leds: [LED; 19],
    pos: [u32; 19],
}

impl LEDs {
    pub fn set(&mut self, pwm: u8) {
        for l in &mut self.leds {
            l.set(pwm)
        }
    }
}

pub fn proto_leds() -> &'static mut LEDs {
    static mut SINGLETON: LEDs = LEDs::new(PROTO_LED_MAPPING);
    unsafe { &mut SINGLETON }
}

pub fn snowflake_leds() -> &'static mut LEDs {
    static mut SINGLETON: LEDs = LEDs::new(SNOWFLAKE_LED_MAPPING);
    unsafe { &mut SINGLETON }
}

pub fn get_neighbours(which: usize) -> &'static [usize] {
    match which {
        0 => &[6],
        1 => &[7],
        2 => &[8],
        3 => &[9],
        4 => &[10],
        5 => &[11],
        6 => &[0, 12, 17],
        7 => &[1, 12, 13],
        8 => &[2, 13, 14],
        9 => &[3, 14, 15],
        10 => &[4, 15, 16],
        11 => &[5, 16, 17],
        12 => &[6, 7, 13, 18, 17],
        13 => &[7, 8, 14, 18, 12],
        14 => &[8, 9, 15, 18, 13],
        15 => &[9, 10, 16, 18, 14],
        16 => &[10, 11, 17, 18, 15],
        17 => &[11, 6, 12, 18, 16],
        18 => &[12, 13, 14, 15, 16, 17],
        _ => &[],
    }
}

pub enum SNOWFLAKE_RING {
    INNER,
    ONE,
    TWO,
    OUTER,
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
    pub const fn new(mapping: [u32; 19]) -> LEDs {
        LEDs {
            leds: [
                LED::new(),
                LED::new(),
                LED::new(),
                LED::new(),
                LED::new(),
                LED::new(),
                LED::new(),
                LED::new(),
                LED::new(),
                LED::new(),
                LED::new(),
                LED::new(),
                LED::new(),
                LED::new(),
                LED::new(),
                LED::new(),
                LED::new(),
                LED::new(),
                LED::new(),
            ],
            pos: mapping,
        }
    }

    pub fn get_ring(&mut self, which: &SNOWFLAKE_RING) -> &[LED] {
        match *which {
            SNOWFLAKE_RING::INNER => &self.leds[18..19],
            SNOWFLAKE_RING::ONE => &self.leds[12..18],
            SNOWFLAKE_RING::TWO => &self.leds[6..12],
            SNOWFLAKE_RING::OUTER => &self.leds[0..6],
        }
    }

    pub fn get_ring_mut(&mut self, which: &SNOWFLAKE_RING) -> &mut [LED] {
        match *which {
            SNOWFLAKE_RING::INNER => &mut self.leds[18..19],
            SNOWFLAKE_RING::ONE => &mut self.leds[12..18],
            SNOWFLAKE_RING::TWO => &mut self.leds[6..12],
            SNOWFLAKE_RING::OUTER => &mut self.leds[0..6],
        }
    }

    pub fn set_ring(&mut self, which: &SNOWFLAKE_RING, value: u8) {
        for l in self.get_ring_mut(which) {
            l.pwm_state = value;
        }
    }

    pub fn shift_outwards(&mut self) {
        let inner: u8 = self.get_ring(&SNOWFLAKE_RING::INNER)
            .iter()
            .nth(0)
            .unwrap()
            .get();
        let mut state: [u8; 6] = [inner; 6];

        self.set_ring(&SNOWFLAKE_RING::INNER, 0);

        for (l, s) in self.get_ring_mut(&SNOWFLAKE_RING::ONE)
            .iter_mut()
            .zip(state.iter_mut())
        {
            mem::swap(&mut **l, s);
        }

        for (l, s) in self.get_ring_mut(&SNOWFLAKE_RING::TWO)
            .iter_mut()
            .zip(state.iter_mut())
        {
            mem::swap(&mut **l, s);
        }

        for (l, s) in self.get_ring_mut(&SNOWFLAKE_RING::OUTER)
            .iter_mut()
            .zip(state.iter_mut())
        {
            mem::swap(&mut **l, s);
        }
    }

    pub fn shift_inwards(&mut self) {
        let mut state: [u8; 6] = [0; 6];

        for (l, s) in self.get_ring_mut(&SNOWFLAKE_RING::OUTER)
            .iter_mut()
            .zip(state.iter_mut())
        {
            mem::swap(&mut **l, s);
        }

        for (l, s) in self.get_ring_mut(&SNOWFLAKE_RING::TWO)
            .iter_mut()
            .zip(state.iter_mut())
        {
            mem::swap(&mut **l, s);
        }

        for (l, s) in self.get_ring_mut(&SNOWFLAKE_RING::ONE)
            .iter_mut()
            .zip(state.iter_mut())
        {
            mem::swap(&mut **l, s);
        }

        self.set_ring(&SNOWFLAKE_RING::INNER, *state.iter().max().unwrap_or(&0));
    }

    /* Saturated addition of constant to all LED PWM values */
    pub fn adds(&mut self, other: u8) {
        for l in &mut self.leds {
            l.add(other);
        }
    }

    /* Overflowing addition of constant to all LED PWM values */
    pub fn add(&mut self, other: u8) {
        for l in &mut self.leds {
            l.pwm_state += other
        }
    }

    /* Saturated substraction of constant from all LED PWM values */
    pub fn subs(&mut self, other: u8) {
        for l in &mut self.leds {
            l.sub(other);
        }
    }

    /* Underflowing substraction of constant from all LED PWM values */
    pub fn sub(&mut self, other: u8) {
        for l in &mut self.leds {
            l.pwm_state -= other
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
}

pub const DATAOUT: u32 = 1 << 15;

/* LED to pin mapping for the protoboard */
const PROTO_LED_MAPPING: [u32; 19] = [
    1,
    1 << 1,
    1 << 2,
    1 << 3,
    1 << 4,
    1 << 5,
    1 << 6,
    1 << 7,
    1 << 8,
    1 << 9,
    1 << 10,
    1 << 11,
    1 << 24,
    1 << 23,
    1 << 22,
    1 << 19,
    1 << 18,
    1 << 17,
    1 << 16,
];

/* LED to pin mapping for real snowflake */
const SNOWFLAKE_LED_MAPPING: [u32; 19] = [
    1,
    1 << 3,
    1 << 6,
    1 << 9,
    1 << 16,
    1 << 19,
    1 << 1,
    1 << 4,
    1 << 7,
    1 << 10,
    1 << 17,
    1 << 22,
    1 << 2,
    1 << 5,
    1 << 8,
    1 << 11,
    1 << 18,
    1 << 23,
    1 << 24,
];

pub const PWMSINE: [u8; 19] = [
    0, 42, 83, 121, 157, 188, 213, 234, 247, 254, 254, 247, 234, 213, 188, 157, 121, 83, 42,
];

/* An array mapping physical PWM values (255 == fully on, 0 = off, inbetween determines percentage
 * of duty cycle) to perceived PWM values */
pub const PWMPERC: [u8; 256] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4,
    4, 4, 5, 5, 5, 5, 6, 6, 6, 7, 7, 7, 8, 8, 8, 9, 9, 9, 10, 10, 11, 11, 11, 12, 12, 13, 13, 14,
    14, 15, 15, 16, 16, 17, 17, 18, 18, 19, 19, 20, 20, 21, 21, 22, 23, 23, 24, 24, 25, 26, 26, 27,
    28, 28, 29, 30, 30, 31, 32, 32, 33, 34, 35, 35, 36, 37, 38, 38, 39, 40, 41, 42, 42, 43, 44, 45,
    46, 47, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67,
    68, 69, 70, 71, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 84, 85, 86, 87, 88, 89, 91, 92, 93, 94,
    95, 97, 98, 99, 100, 102, 103, 104, 105, 107, 108, 109, 111, 112, 113, 115, 116, 117, 119, 120,
    121, 123, 124, 126, 127, 128, 130, 131, 133, 134, 136, 137, 139, 140, 142, 143, 145, 146, 148,
    149, 151, 152, 154, 155, 157, 158, 160, 162, 163, 165, 166, 168, 170, 171, 173, 175, 176, 178,
    180, 181, 183, 185, 186, 188, 190, 192, 193, 195, 197, 199, 200, 202, 204, 206, 207, 209, 211,
    213, 215, 217, 218, 220, 222, 224, 226, 228, 230, 232, 233, 235, 237, 239, 241, 243, 245, 247,
    249, 251, 253, 255,
];

/* An array mapping humanly perceived PWM values (255 == fully on, 0 = off, inbetween determines percentage
 * of duty cycle) to physical PWM values. This is the inverse mappting to PWMPERC and mostly for
 * reference. Since both are calculated from the formula PWMINVPERC[i] = round (255 * sqrt(i / 255))
 * the indices will due to rounding point to the middle value instead of the first */
pub const PWMINVPERC: [u8; 256] = [
    0, 16, 23, 28, 32, 36, 39, 42, 45, 48, 50, 53, 55, 58, 60, 62, 64, 66, 68, 70, 71, 73, 75, 77,
    78, 80, 81, 83, 84, 86, 87, 89, 90, 92, 93, 94, 96, 97, 98, 100, 101, 102, 103, 105, 106, 107,
    108, 109, 111, 112, 113, 114, 115, 116, 117, 118, 119, 121, 122, 123, 124, 125, 126, 127, 128,
    129, 130, 131, 132, 133, 134, 135, 135, 136, 137, 138, 139, 140, 141, 142, 143, 144, 145, 145,
    146, 147, 148, 149, 150, 151, 151, 152, 153, 154, 155, 156, 156, 157, 158, 159, 160, 160, 161,
    162, 163, 164, 164, 165, 166, 167, 167, 168, 169, 170, 170, 171, 172, 173, 173, 174, 175, 176,
    176, 177, 178, 179, 179, 180, 181, 181, 182, 183, 183, 184, 185, 186, 186, 187, 188, 188, 189,
    190, 190, 191, 192, 192, 193, 194, 194, 195, 196, 196, 197, 198, 198, 199, 199, 200, 201, 201,
    202, 203, 203, 204, 204, 205, 206, 206, 207, 208, 208, 209, 209, 210, 211, 211, 212, 212, 213,
    214, 214, 215, 215, 216, 217, 217, 218, 218, 219, 220, 220, 221, 221, 222, 222, 223, 224, 224,
    225, 225, 226, 226, 227, 228, 228, 229, 229, 230, 230, 231, 231, 232, 233, 233, 234, 234, 235,
    235, 236, 236, 237, 237, 238, 238, 239, 240, 240, 241, 241, 242, 242, 243, 243, 244, 244, 245,
    245, 246, 246, 247, 247, 248, 248, 249, 249, 250, 250, 251, 251, 252, 252, 253, 253, 254, 254,
    255,
];
