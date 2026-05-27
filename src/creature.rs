//! Creature creation logic

use tinyrand::RandRange;

pub const NUM_SHAPES: u8 = 14;
pub const NUM_COLORS: u8 = 8;
pub const NUM_MOUTHS: u8 = 8;
pub const NUM_EYES: u8 = 10;
pub const NUM_LIMBS: u8 = 7;

#[derive(Debug, Default, Copy, Clone, Eq, Hash, PartialEq)]
pub struct CreatureParams {
    /// parameter 1: shape
    pub shape: u8,
    /// parameter 2: color
    pub color: u8,
    /// parameter 3: eyes
    pub eyes: u8,
    /// parameter 4: mouth
    pub mouth: u8,
    /// parameter 5: legs
    pub legs: u8,
    /// parameter 6: arms
    pub arms: u8,
}

impl CreatureParams {
    pub fn new_random(rng: &mut impl RandRange<u16>) -> Self {
        // in this game we pick the same number for legs and arms,
        // to make it less confusing
        let limbs = rng.next_range(0..NUM_LIMBS as u16) as u8;
        CreatureParams {
            shape: rng.next_range(0..NUM_SHAPES as u16) as u8,
            color: rng.next_range(0..NUM_COLORS as u16) as u8,
            eyes: rng.next_range(0..NUM_EYES as u16) as u8,
            mouth: rng.next_range(0..NUM_MOUTHS as u16) as u8,
            legs: limbs,
            arms: limbs,
        }
    }

    /// Change exactly one of the characteristics of the creature
    pub fn mutate_one(&mut self, rng: &mut impl RandRange<u16>) {
        let characteristic = rng.next_range(0..5);
        match characteristic {
            // shape
            0 => {
                // roll for `#shapes - 1`,
                // then skip so we exclude the one currently selected
                //        v
                // | | | |x|o| |
                self.shape = match rng.next_range(0..NUM_SHAPES as u16 - 1) as u8 {
                    v if v < self.shape => v,
                    // skip the preexisting value
                    v => v + 1,
                };
                debug_assert!(self.shape < NUM_SHAPES);
            }
            // color
            1 => {
                self.color = match rng.next_range(0..NUM_COLORS as u16 - 1) as u8 {
                    v if v < self.color => v,
                    // skip the preexisting value
                    v => v + 1,
                };
                debug_assert!(self.color < NUM_COLORS);
            }
            // eyes
            2 => {
                self.eyes = match rng.next_range(0..NUM_EYES as u16 - 1) as u8 {
                    v if v < self.eyes => v,
                    // skip the preexisting value
                    v => v + 1,
                };
                debug_assert!(self.eyes < NUM_EYES);
            }
            // limbs
            3 => {
                self.arms = match rng.next_range(0..NUM_LIMBS as u16 - 1) as u8 {
                    v if v < self.arms => v,
                    // skip the preexisting value
                    v => v + 1,
                };
                debug_assert!(self.arms < NUM_EYES);
                // apply also to the legs
                self.legs = self.arms;
            }
            _ => unreachable!(),
        }
    }

    /// maps param2 to the main RGB color (in 0..64 range)
    pub fn body_color(&self) -> [u8; 3] {
        match self.color {
            // white
            0 => [0x3a, 0x3a, 0x3a],
            // red
            1 => [0x3c, 0x14, 0x14],
            // yellow
            2 => [0x3c, 0x3c, 0x14],
            // green
            3 => [0x14, 0x3c, 0x14],
            // cyan
            4 => [0x14, 0x3c, 0x3c],
            // blue
            5 => [0x16, 0x16, 0x3c],
            // magenta
            6 => [0x3c, 0x14, 0x3c],
            // brown
            7 => [0x30, 0x20, 0x14],
            // fallback to grey
            _ => [0x1f, 0x1f, 0x1f],
        }
    }

    /// create the palette slice for the creature's body colors
    pub fn body_colors(&self) -> [u8; 12] {
        let base_color = self.body_color();
        [
            // light shade
            (base_color[0] + 24).min(63),
            (base_color[1] + 24).min(63),
            (base_color[2] + 24).min(63),
            // base color
            base_color[0],
            base_color[1],
            base_color[2],
            // darker shade
            base_color[0] / 2,
            base_color[1] / 2,
            base_color[2] / 2,
            // darker shade
            base_color[0] / 4,
            base_color[1] / 4,
            base_color[2] / 4,
        ]
    }

    /// Count how many characteristics are equal
    pub fn count_matching_properties(&self, other: &CreatureParams) -> u8 {
        (self.color == other.color) as u8
            + (self.eyes == other.eyes) as u8
            + (self.shape == other.shape) as u8
            // in this game we treat arms and legs as a single characteristic
            + (self.arms == other.arms) as u8
    }
}

/// The Display impl prints the creature's generated name
impl core::fmt::Display for CreatureParams {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // portion defined by the creature's color
        let display2 = match self.color {
            0 => "a",
            1 => "e",
            2 => "ey",
            3 => "i",
            4 => "or",
            5 => "o",
            6 => "ar",
            7 => "ur",
            _ => "Unknown",
        };

        // part 1
        match self.shape {
            0 => write!(f, "Fl{display2}")?,
            1 => write!(f, "D{display2}n")?,
            2 => write!(f, "Bl{display2}")?,
            3 => write!(f, "Em{display2}")?,
            4 => write!(f, "N{display2}n")?,
            5 => write!(f, "Sn{display2}")?,
            6 => write!(f, "Yl{display2}m")?,
            7 => write!(f, "H{display2}")?,
            8 => write!(f, "J{display2}m")?,
            9 => write!(f, "Al{display2}")?,
            10 => write!(f, "V{display2}n")?,
            11 => write!(f, "T{display2}")?,
            12 => write!(f, "B{display2}")?,
            13 => write!(f, "K{display2}")?,
            NUM_SHAPES.. => unreachable!(),
        };

        // defined by creature's limbs
        let display5 = match self.legs {
            0 => "n",
            1 => "t",
            2 => "b",
            3 => "f",
            4 => "r",
            5 => "wh",
            6 => "d",
            _ => "",
        };
        f.write_str(display5)?;

        // defined by creature's eyes
        let display4 = match self.eyes {
            0 => "i",
            1 => "o",
            2 => "ow",
            3 => "e",
            4 => "a",
            5 => "ya",
            6 => "yo",
            7 => "u",
            8 => "oo",
            9 => "ey",
            NUM_EYES.. => unreachable!(),
        };
        f.write_str(display4)?;

        // defined by creature's mouth
        let display3 = match self.mouth {
            0 => "n",
            1 => "ty",
            2 => "d",
            3 => "r",
            4 => "z",
            5 => "b",
            6 => "m",
            7 => "x",
            NUM_MOUTHS.. => unreachable!(),
        };
        f.write_str(display3)
    }
}
