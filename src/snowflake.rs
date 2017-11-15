use core::mem;
use core::ops::Deref;
use core::ops::{Index, IndexMut};
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
     * 156µs@48Mhz if all 19 LEDs are actively used */
    pub fn calculate(&mut self, leds: &LEDs) {
        let mut _leds: [u8; 19] = unsafe { mem::uninitialized() };
        let mut _values: [u8; 20] = unsafe { mem::uninitialized() };

        for i in 0..19 {
            let pwmvalue = leds[i].pwm_state;
            _leds[i] = pwmvalue;
            _values[i + 1] = pwmvalue;
        }
        _values[0] = 0;
        _values.sort_unstable();

        for values in _values.windows(2) {
            let (start, end) = (values[0], values[1]);
            if start != end {
                let bitmask = _leds.iter().enumerate().fold(
                    0,
                    |a, l| if (start as u8) < *l.1 {
                        a | leds[l.0].pos
                    } else {
                        a
                    },
                );

                for entry in self.bitmask[start as usize..end as usize].iter_mut() {
                    unsafe { ptr::write(entry, bitmask) };
                }
            }
        }
    }

    /* Pretty much the same as calculate() but scales the PWM values according to the perception of
     * the brightness to the human eye which usually yields a slightly more pleasing effect.
     * This runs at an average of around 156µs@48Mhz if all 19 LEDs are actively used */
    pub fn calculate_perceived(&mut self, leds: &LEDs) {
        let mut _leds: [u8; 19] = unsafe { mem::uninitialized() };
        let mut _values: [u8; 20] = unsafe { mem::uninitialized() };

        for i in 0..19 {
            let pwmvalue = PWMPERC[leds[i].pwm_state as usize];
            _leds[i] = pwmvalue;
            _values[i + 1] = pwmvalue;
        }
        _values[0] = 0;
        _values.sort_unstable();

        for values in _values.windows(2) {
            let (start, end) = (values[0], values[1]);
            if start != end {
                let bitmask = _leds.iter().enumerate().fold(
                    0,
                    |a, l| if (start as u8) < *l.1 {
                        a | leds[l.0].pos
                    } else {
                        a
                    },
                );

                for entry in self.bitmask[start as usize..end as usize].iter_mut() {
                    unsafe { ptr::write(entry, bitmask) };
                }
            }
        }
    }

    pub fn get_clear_bits(&self, time: u8) -> u32 {
        self.bitmask[time as usize]
    }

    pub fn get_set_bits(&self, time: u8) -> u32 {
        !self.bitmask[time as usize] & !(1<<15)
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


impl Deref for LED {
    type Target = u8;

    fn deref(&self) -> &Self::Target {
        &self.pwm_state
    }
}


pub struct LEDs {
    leds: [LED; 19],
}


impl LEDs {
    pub fn set(&mut self, pwm: u8) {
        for l in &mut self.leds {
            l.set(pwm)
        }
    }
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
            i.pwm_state = if (i32::from(i.pwm_state) - i32::from(other)) < 0 {
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


pub const PWMSINE: [u8; 19] = [
    0,
    42,
    83,
    121,
    157,
    188,
    213,
    234,
    247,
    254,
    254,
    247,
    234,
    213,
    188,
    157,
    121,
    83,
    42,
];


/* An array mapping physical PWM values (255 == fully on, 0 = off, inbetween determines percentage
 * of duty cycle) to perceived PWM values */
pub const PWMPERC: [u8; 256] = [
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    1,
    1,
    1,
    1,
    1,
    1,
    1,
    1,
    2,
    2,
    2,
    2,
    2,
    2,
    3,
    3,
    3,
    3,
    4,
    4,
    4,
    4,
    5,
    5,
    5,
    5,
    6,
    6,
    6,
    7,
    7,
    7,
    8,
    8,
    8,
    9,
    9,
    9,
    10,
    10,
    11,
    11,
    11,
    12,
    12,
    13,
    13,
    14,
    14,
    15,
    15,
    16,
    16,
    17,
    17,
    18,
    18,
    19,
    19,
    20,
    20,
    21,
    21,
    22,
    23,
    23,
    24,
    24,
    25,
    26,
    26,
    27,
    28,
    28,
    29,
    30,
    30,
    31,
    32,
    32,
    33,
    34,
    35,
    35,
    36,
    37,
    38,
    38,
    39,
    40,
    41,
    42,
    42,
    43,
    44,
    45,
    46,
    47,
    47,
    48,
    49,
    50,
    51,
    52,
    53,
    54,
    55,
    56,
    56,
    57,
    58,
    59,
    60,
    61,
    62,
    63,
    64,
    65,
    66,
    67,
    68,
    69,
    70,
    71,
    73,
    74,
    75,
    76,
    77,
    78,
    79,
    80,
    81,
    82,
    84,
    85,
    86,
    87,
    88,
    89,
    91,
    92,
    93,
    94,
    95,
    97,
    98,
    99,
    100,
    102,
    103,
    104,
    105,
    107,
    108,
    109,
    111,
    112,
    113,
    115,
    116,
    117,
    119,
    120,
    121,
    123,
    124,
    126,
    127,
    128,
    130,
    131,
    133,
    134,
    136,
    137,
    139,
    140,
    142,
    143,
    145,
    146,
    148,
    149,
    151,
    152,
    154,
    155,
    157,
    158,
    160,
    162,
    163,
    165,
    166,
    168,
    170,
    171,
    173,
    175,
    176,
    178,
    180,
    181,
    183,
    185,
    186,
    188,
    190,
    192,
    193,
    195,
    197,
    199,
    200,
    202,
    204,
    206,
    207,
    209,
    211,
    213,
    215,
    217,
    218,
    220,
    222,
    224,
    226,
    228,
    230,
    232,
    233,
    235,
    237,
    239,
    241,
    243,
    245,
    247,
    249,
    251,
    253,
    255,
];


/* An array mapping humanly perceived PWM values (255 == fully on, 0 = off, inbetween determines percentage
 * of duty cycle) to physical PWM values. This is the inverse mappting to PWMPERC and mostly for
 * reference. Since both are calculated from the formula PWMINVPERC[i] = round (255 * sqrt(i / 255))
 * the indices will due to rounding point to the middle value instead of the first */
pub const PWMINVPERC: [u8; 256] = [
    0,
    16,
    23,
    28,
    32,
    36,
    39,
    42,
    45,
    48,
    50,
    53,
    55,
    58,
    60,
    62,
    64,
    66,
    68,
    70,
    71,
    73,
    75,
    77,
    78,
    80,
    81,
    83,
    84,
    86,
    87,
    89,
    90,
    92,
    93,
    94,
    96,
    97,
    98,
    100,
    101,
    102,
    103,
    105,
    106,
    107,
    108,
    109,
    111,
    112,
    113,
    114,
    115,
    116,
    117,
    118,
    119,
    121,
    122,
    123,
    124,
    125,
    126,
    127,
    128,
    129,
    130,
    131,
    132,
    133,
    134,
    135,
    135,
    136,
    137,
    138,
    139,
    140,
    141,
    142,
    143,
    144,
    145,
    145,
    146,
    147,
    148,
    149,
    150,
    151,
    151,
    152,
    153,
    154,
    155,
    156,
    156,
    157,
    158,
    159,
    160,
    160,
    161,
    162,
    163,
    164,
    164,
    165,
    166,
    167,
    167,
    168,
    169,
    170,
    170,
    171,
    172,
    173,
    173,
    174,
    175,
    176,
    176,
    177,
    178,
    179,
    179,
    180,
    181,
    181,
    182,
    183,
    183,
    184,
    185,
    186,
    186,
    187,
    188,
    188,
    189,
    190,
    190,
    191,
    192,
    192,
    193,
    194,
    194,
    195,
    196,
    196,
    197,
    198,
    198,
    199,
    199,
    200,
    201,
    201,
    202,
    203,
    203,
    204,
    204,
    205,
    206,
    206,
    207,
    208,
    208,
    209,
    209,
    210,
    211,
    211,
    212,
    212,
    213,
    214,
    214,
    215,
    215,
    216,
    217,
    217,
    218,
    218,
    219,
    220,
    220,
    221,
    221,
    222,
    222,
    223,
    224,
    224,
    225,
    225,
    226,
    226,
    227,
    228,
    228,
    229,
    229,
    230,
    230,
    231,
    231,
    232,
    233,
    233,
    234,
    234,
    235,
    235,
    236,
    236,
    237,
    237,
    238,
    238,
    239,
    240,
    240,
    241,
    241,
    242,
    242,
    243,
    243,
    244,
    244,
    245,
    245,
    246,
    246,
    247,
    247,
    248,
    248,
    249,
    249,
    250,
    250,
    251,
    251,
    252,
    252,
    253,
    253,
    254,
    254,
    255,
];
