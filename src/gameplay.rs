//! Some data structures for the gameplay logic
//!

use alloc::collections::vec_deque::VecDeque;
use tinyrand::RandRange;

use crate::{creature::CreatureParams, gfx::HoleSprite};

/// Wrapper for an index representing a hole,
/// a number from 0 to 8 in row-first order.
#[derive(Debug, Copy, Clone, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct HoleIndex(u8);

impl HoleIndex {
    /// Get the X and Y integer coordinates of this hole,
    /// in hole slots of a 3x3 grid.
    ///
    /// # Examples
    ///
    /// ```
    /// assert_eq!(HoleIndex(0).coords(), (0, 0));
    /// assert_eq!(HoleIndex(1).coords(), (1, 0));
    /// assert_eq!(HoleIndex(4).coords(), (1, 1));
    /// assert_eq!(HoleIndex(8).coords(), (2, 2));
    /// ```
    pub fn coords(&self) -> (u8, u8) {
        // it's probably cheaper to expand them all
        // rather than doing a division by 3
        match self.0 {
            0 => (0, 0),
            1 => (1, 0),
            2 => (2, 0),
            3 => (0, 1),
            4 => (1, 1),
            5 => (2, 1),
            6 => (0, 2),
            7 => (1, 2),
            8 => (2, 2),
            _ => unreachable!("hole index out of bounds"),
        }
    }

    /// Create an iterator through all 9 holes.
    pub fn all() -> impl core::iter::Iterator<Item = HoleIndex> {
        (0..9).map(HoleIndex)
    }

    pub fn to_usize(self) -> usize {
        self.0 as usize
    }
}

/// A pre-programmed event happening during the round,
/// typically the appearance of a creature.
#[derive(Debug, Copy, Clone, PartialEq)]
#[repr(u8)]
pub enum EventKind {
    /// Make a creature appear from a hole
    ///
    /// (which hole is only decided during the game)
    Appear(CreatureIndex),
}

/// A pre-programmed event happening during the round,
/// typically the appearance of a creature.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Event {
    /// The instant in number of ticks
    /// that this event is triggered
    pub tick: u16,
    /// Make a creature appear from a specific hole
    pub kind: EventKind,
}

/// The state of a game round
#[derive(Debug, Clone)]
pub struct RoundState {
    /// how many ticks have passed
    pub ticks: u32,
    pub events: VecDeque<Event>,
    pub options: RoundOptions,
}

impl RoundState {
    pub const ROUND_LENGTH: u16 = RoundOptions::ROUND_LENGTH;

    pub fn new(options: RoundOptions, rng: &mut impl RandRange<u16>) -> Self {
        RoundState {
            ticks: 0,
            events: Self::build_events(&options, rng),
            options,
        }
    }

    /// Fetch an event if it occurs at the given tick instant
    /// (or should have occurred before)
    pub fn pop_event(&mut self, current_tick: u16) -> Option<Event> {
        let next_event_tick = self.events.front().map(|ev| ev.tick)?;
        if next_event_tick <= current_tick {
            self.events.pop_front()
        } else {
            None
        }
    }

    /// pre-populate what will happen in a round
    fn build_events(options: &RoundOptions, rng: &mut impl RandRange<u16>) -> VecDeque<Event> {
        let mut events = VecDeque::with_capacity(
            options.num_creatures as usize + options.num_creatures_distraction as usize,
        );

        // spawn creatures to whack and to grab
        for _ in 0..options.num_creatures {
            let tick = rng.next_range(0..RoundState::ROUND_LENGTH - 60);
            // 1d2
            let n = rng.next_range(0..2);
            let creature = if n > 0 {
                CreatureIndex::Whack
            } else {
                CreatureIndex::Grab
            };
            events.push_back(Event {
                tick,
                kind: EventKind::Appear(creature),
            });
        }

        // spawn creatures to distract
        // first identify how many distinct distracting creatures we have
        let distinct = match (
            &options.distraction3,
            &options.distraction2,
            &options.distraction1,
        ) {
            (Some(_), _, _) => 3,
            (None, Some(_), _) => 2,
            (None, None, Some(_)) => 1,
            (None, None, None) => 0,
        };
        if distinct > 0 {
            for _ in 0..options.num_creatures_distraction {
                let tick = rng.next_range(0..RoundState::ROUND_LENGTH);
                // roll for distinct creatures
                let c = rng.next_range(0..distinct);
                let creature = match c {
                    0 => CreatureIndex::Other1,
                    1 => CreatureIndex::Other2,
                    2 => CreatureIndex::Other3,
                    _ => unreachable!(),
                };
                events.push_back(Event {
                    tick,
                    kind: EventKind::Appear(creature),
                });
            }
        }

        // sort the events
        events.make_contiguous().sort_by_key(|event| event.tick);

        events
    }
}

/// A compact representation for
/// the creature that may appear from a hole.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum CreatureIndex {
    /// The creature to whack.
    ///
    /// Can also be used for the default (only) creature on screen.
    #[default]
    Whack = 0,
    /// The creature to grab
    Grab = 1,
    /// Another creature to serve as distraction (#1)
    Other1 = 2,
    /// Another creature to serve as distraction (#2)
    Other2 = 3,
    /// Another creature to serve as distraction (#3)
    Other3 = 4,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Difficulty {
    Easy,
    Normal,
    Hard,
}

/// The characteristics of a game round
#[derive(Debug, Clone)]
pub struct RoundOptions {
    /// difficulty
    pub difficulty: Difficulty,
    /// whack this creature
    pub whack: CreatureParams,
    /// save/grab this creature
    pub grab: CreatureParams,

    /// the creature that might appear just for distraction
    pub distraction1: Option<CreatureParams>,
    /// the creature that might appear just for distraction
    pub distraction2: Option<CreatureParams>,
    /// the creature that might appear just for distraction
    pub distraction3: Option<CreatureParams>,

    /// measure for the usual amount of time that a creature is on screen
    /// before it hides, in game ticks
    pub avg_idle_time: u16,

    /// total number of creatures to present in a round
    pub num_creatures: u16,

    /// total number of distracting creatures to present in a round
    pub num_creatures_distraction: u16,
}

impl RoundOptions {
    /// The total length of a round in ticks
    const ROUND_LENGTH: u16 = 150 * 8;

    /// Create a new game round on Easy difficulty
    pub fn new_game_easy(rng: &mut impl RandRange<u16>) -> Self {
        let whack = CreatureParams::new_random(rng);

        // obtain a sufficiently different creature for grabbing
        let mut grab = CreatureParams::new_random(rng);
        for _ in 0..16 {
            if grab.count_matching_properties(&whack) <= 2 {
                break;
            }
            grab = CreatureParams::new_random(rng);
        }

        // easy game has no distractions, and creatures are slow
        RoundOptions {
            difficulty: Difficulty::Easy,
            whack,
            grab,
            distraction1: None,
            distraction2: None,
            distraction3: None,
            avg_idle_time: 100,
            num_creatures: 45,
            num_creatures_distraction: 0,
        }
    }

    /// Create a new game round on Medium difficulty
    pub fn new_game_medium(rng: &mut impl RandRange<u16>) -> Self {
        let whack = CreatureParams::new_random(rng);

        // obtain a sufficiently different creature for grabbing
        let mut grab = CreatureParams::new_random(rng);
        for _ in 0..16 {
            if grab.count_matching_properties(&whack) <= 3 {
                break;
            }
            grab = CreatureParams::new_random(rng);
        }

        let distraction = loop {
            let c = CreatureParams::new_random(rng);
            if c != whack && c != grab {
                break c;
            }
        };

        // medium game has 1 distraction, and creatures are faster
        RoundOptions {
            difficulty: Difficulty::Normal,
            whack,
            grab,
            distraction1: Some(distraction),
            distraction2: None,
            distraction3: None,
            avg_idle_time: 78,
            num_creatures: 80,
            num_creatures_distraction: 3,
        }
    }

    /// Create a new game round on Hard difficulty
    pub fn new_game_hard(rng: &mut impl RandRange<u16>) -> Self {
        let whack = CreatureParams::new_random(rng);

        // copy the other creature and change it a bit
        let mut grab = whack;
        grab.mutate_one(rng);

        let distraction1 = loop {
            let c = CreatureParams::new_random(rng);
            if c != whack && c != grab {
                break c;
            }
        };
        let distraction2 = loop {
            let c = CreatureParams::new_random(rng);
            if c != whack && c != grab && c != distraction1 {
                break c;
            }
        };
        let distraction3 = loop {
            let c = CreatureParams::new_random(rng);
            if c != whack && c != grab && c != distraction1 && c != distraction2 {
                break c;
            }
        };

        // hard game has 3 distractions, and creatures are even faster
        RoundOptions {
            difficulty: Difficulty::Hard,
            whack,
            grab,
            distraction1: Some(distraction1),
            distraction2: Some(distraction2),
            distraction3: Some(distraction3),
            avg_idle_time: 64,
            num_creatures: 110,
            num_creatures_distraction: 10,
        }
    }

    pub fn creature_of(&self, index: CreatureIndex) -> Option<&CreatureParams> {
        match index {
            CreatureIndex::Whack => Some(&self.whack),
            CreatureIndex::Grab => Some(&self.grab),
            CreatureIndex::Other1 => self.distraction1.as_ref(),
            CreatureIndex::Other2 => self.distraction2.as_ref(),
            CreatureIndex::Other3 => self.distraction3.as_ref(),
        }
    }
}

/// Represents the 3x3 grid of holes that the player can whack/grab
pub struct Table {
    /// the table's base position on the screen in pixels
    base_pos: (u16, u16),

    holes: [HoleStatus; 9],
}

impl Table {
    /// number of pixels to the right until you find the next hole
    pub const HOLE_STRIDE_X: u16 = 40;
    /// number of pixels down until you find the next hole
    pub const HOLE_STRIDE_Y: u16 = 32;

    pub fn new(base_pos: (u16, u16)) -> Self {
        Table {
            base_pos,
            holes: Default::default(),
        }
    }

    /// get a hole's position in pixels
    pub fn hole_pos(&self, hole: HoleIndex) -> (u16, u16) {
        let (hole_x, hole_y) = hole.coords();
        (
            self.base_pos.0 + hole_x as u16 * Table::HOLE_STRIDE_X,
            self.base_pos.1 + hole_y as u16 * Table::HOLE_STRIDE_Y,
        )
    }

    /// get mutable access to each hole slot
    #[inline]
    pub fn holes_mut(&mut self) -> &mut [HoleStatus] {
        &mut self.holes
    }

    #[inline]
    pub fn holes(&self) -> &[HoleStatus] {
        &self.holes
    }

    /// Roll for an empty hole in the table
    pub fn pick_empty(&self, rng: &mut impl RandRange<u16>) -> Option<HoleIndex> {
        // return `None` if all are occupied
        if self.holes.iter().all(|h| !h.is_empty()) {
            return None;
        }

        // try a limited number of times
        // so the performance does not become too unpredictable
        for _ in 0..16 {
            let i = rng.next_range(0..9);
            if self.holes[i as usize].is_empty() {
                return Some(HoleIndex(i as u8));
            }
        }
        None
    }

    /// create new holes according to this table
    pub fn generate_hole_sprites(&self) -> [HoleSprite; 9] {
        let mut holes: [core::mem::MaybeUninit<HoleSprite>; 9] =
            [core::mem::MaybeUninit::uninit(); 9];

        for i in 0..9 {
            let (x, y) = self.hole_pos(HoleIndex(i));
            holes[i as usize].write(HoleSprite::new_at(x as i32, y as i32));
        }

        // experimental API usage.
        // for something more stable, use `transmute`
        unsafe { core::mem::MaybeUninit::array_assume_init(holes) }
    }

    /// put a creature in a hole,
    /// making it appear in future updates
    pub fn put(&mut self, hole: HoleIndex, creature: CreatureIndex) {
        self.holes[hole.0 as usize] = HoleStatus::Appearing { creature, frame: 0 };
    }

    /// get info about the hole positioned where the glove is
    /// (in coordinates (-1, -1) to (1, 1) )
    pub fn hole_at_mut(&mut self, glove_x: i8, glove_y: i8) -> &mut HoleStatus {
        let glove_index = (3 * (glove_y + 1) + glove_x + 1) as usize;
        &mut self.holes[glove_index]
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub enum HoleStatus {
    /// Nothing is on the hole
    #[default]
    Empty,

    /// The creature is emerging from the hole.
    /// Can be whacked and grabbed at this point.
    Appearing { creature: CreatureIndex, frame: u16 },

    /// The creature is on plain screen
    Shown { creature: CreatureIndex, frame: u16 },

    /// The creature is going away.
    /// Can still be whacked at this point, but not grabbed.
    Hiding { creature: CreatureIndex, frame: u16 },
}

impl HoleStatus {
    pub fn is_empty(&self) -> bool {
        matches!(self, HoleStatus::Empty)
    }
}
