use crate::{
    gameplay::CreatureIndex,
    gfx::{CreatureSprite, background::Background, set_background_palette},
    input::{self, Key},
};
use alloc::format;
use dos_x::vga::vsync;
use tinyrand::RandRange;

use crate::{
    Assets,
    audio::{play_click_1, play_click_2},
    creature::CreatureParams,
    gameplay::RoundOptions,
    gfx::{
        COLOR_BLACK, COLOR_WHITE, HoleSprite, draw_arrow_left, draw_arrow_right,
        set_creature_palette,
    },
};

#[derive(Debug, Clone)]
pub enum MenuOutcome {
    /// Prepare a new game
    NewGame(RoundOptions),
    /// Instructions page
    HowToPlay,
    /// Enter Create-a-Creature mode (WIP)
    Create,
    /// Exit the game
    Exit,
}

/// Show and operate the main menu
pub fn menu(assets: &mut Assets, rng: &mut impl RandRange<u16>) -> MenuOutcome {
    // simple menu screen with 2 choices
    let mut choice: u16 = 0;

    unsafe {
        vsync();
    }
    // apply the right background
    assets.background = Background::back1();
    assets.background.draw_all();
    set_background_palette(&mut assets.palette, &assets.background);
    assets.palette.set();

    assets.big_font.draw_text(96, 17, "Whack a", COLOR_WHITE);
    assets.big_font.draw_text(97, 18, "Whack a", COLOR_BLACK);

    let mut animation = AnimationStatus::new(&mut *assets, &mut *rng);

    assets
        .small_font
        .draw_text(230, 188, "E_net4, 2026", COLOR_BLACK);

    let menu_options = ["New Game", "How to Play", "Exit"];
    let menu_option_count: u32 = 3;

    // draw main menu section options
    let base_y = 125;
    for (i, opt) in menu_options
        .iter()
        .take(menu_option_count as usize)
        .enumerate()
    {
        let x = 160 - opt.len() * 16 / 2;
        assets
            .big_font
            .draw_text(x as i32, base_y as i32 + i as i32 * 18, opt, COLOR_BLACK);
    }

    loop {
        unsafe {
            vsync();
        }

        animation.step(&mut *assets, &mut *rng);

        const ARROW_LEFT: u32 = 60;
        const ARROW_RIGHT: u32 = 266;
        // clear regions with selection arrow
        assets
            .background
            .draw_rect(ARROW_LEFT as i32, 127, 8, 16 * menu_option_count);
        assets
            .background
            .draw_rect(ARROW_RIGHT as i32, 127, 8, 16 * menu_option_count);

        let selection_y = 128 + choice as u32 * 18;
        draw_arrow_right(ARROW_LEFT, selection_y, COLOR_BLACK);
        draw_arrow_left(ARROW_RIGHT, selection_y, COLOR_BLACK);

        // check key presses

        // check up and down
        if input::is_pressed(Key::Up1) {
            // up arrow
            // change choice
            choice = choice.saturating_sub(1);
            play_click_1();
        } else if input::is_pressed(Key::Down1) {
            // change choice
            choice = (choice + 1).min(menu_option_count as u16 - 1);
            play_click_1();
        }

        // Enter
        if input::is_select_pressed() {
            play_click_2();
            match choice {
                0 => {
                    // wipe section with options
                    assets.background.draw_rect(30, 120, 220, 60);

                    match submenu_new_game(&mut *assets, &mut animation, &mut *rng) {
                        NewGameOutcome::Back => {
                            // rewrite the main menu options
                            assets.background.draw_rect(24, base_y, 180, 64);
                            for (i, opt) in menu_options
                                .iter()
                                .take(menu_option_count as usize)
                                .enumerate()
                            {
                                let x = 160 - opt.len() * 16 / 2;
                                assets.big_font.draw_text(
                                    x as i32,
                                    base_y as i32 + i as i32 * 18,
                                    opt,
                                    COLOR_BLACK,
                                );
                            }
                            continue;
                        }
                        NewGameOutcome::Easy => {
                            let round = RoundOptions::new_game_easy(rng);
                            submenu_present(&mut *assets, &round);
                            return MenuOutcome::NewGame(round);
                        }
                        NewGameOutcome::Medium => {
                            let round = RoundOptions::new_game_medium(rng);
                            submenu_present(&mut *assets, &round);
                            return MenuOutcome::NewGame(round);
                        }
                        NewGameOutcome::Hard => {
                            let round = RoundOptions::new_game_hard(rng);
                            submenu_present(&mut *assets, &round);
                            return MenuOutcome::NewGame(round);
                        }
                        NewGameOutcome::Custom => {
                            // TODO
                            continue;
                        }
                    }
                }
                1 => return MenuOutcome::HowToPlay,
                2 => return MenuOutcome::Exit,
                _ => unreachable!(),
            };
        } else if input::is_pressed(Key::Back) {
            // escape key
            play_click_2();
            return MenuOutcome::Exit;
        }
        input::flip_keystate();

        assets.adlib_player.process();
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum NewGameOutcome {
    Easy,
    Medium,
    Hard,
    Custom,
    Back,
}

/// frame logic for the New Game submenu
fn submenu_new_game(
    assets: &mut Assets,
    animation: &mut AnimationStatus,
    rng: &mut impl RandRange<u16>,
) -> NewGameOutcome {
    // sub menu screen with 3 choices, 1 per difficulty
    let mut choice: u16 = 0;

    let menu_options = ["Easy", "Medium", "Hard"];
    let menu_option_count: u32 = 3;

    // draw section options
    let base_y = 125;
    for (i, opt) in menu_options
        .iter()
        .take(menu_option_count as usize)
        .enumerate()
    {
        let x = 160 - opt.len() * 16 / 2;
        assets
            .big_font
            .draw_text(x as i32, base_y as i32 + i as i32 * 18, opt, COLOR_BLACK);
    }

    input::flip_keystate();

    loop {
        unsafe {
            vsync();
        }

        const ARROW_LEFT: u32 = 60;
        const ARROW_RIGHT: u32 = 266;
        // clear regions with selection arrow
        assets
            .background
            .draw_rect(ARROW_LEFT as i32, 127, 8, 16 * menu_option_count);
        assets
            .background
            .draw_rect(ARROW_RIGHT as i32, 127, 8, 16 * menu_option_count);

        let selection_y = 128 + choice as u32 * 18;
        draw_arrow_right(ARROW_LEFT, selection_y, COLOR_BLACK);
        draw_arrow_left(ARROW_RIGHT, selection_y, COLOR_BLACK);

        // handle menu animations
        animation.step(&mut *assets, &mut *rng);

        // handle keys

        // check up and down
        if input::is_pressed(Key::Up1) {
            // up arrow
            // change choice
            choice = choice.saturating_sub(1);
            play_click_1();
        } else if input::is_pressed(Key::Down1) {
            // change choice
            choice = (choice + 1).min(menu_option_count as u16 - 1);
            play_click_1();
        }

        if input::is_select_pressed() {
            play_click_2();
            match choice {
                0 => return NewGameOutcome::Easy,
                1 => return NewGameOutcome::Medium,
                2 => return NewGameOutcome::Hard,
                _ => unreachable!(),
            };
        } else if input::is_pressed(Key::Back) {
            // escape key
            play_click_2();
            return NewGameOutcome::Back;
        }
        input::flip_keystate();

        assets.adlib_player.process();
    }
}

// --- in-menu animation logic ---

struct AnimationStatus {
    creature_frame: u32,
    creature: CreatureParams,
    creature_sprite: CreatureSprite,
    active_hole: HoleSprite,
    hole_2: HoleSprite,
    hole_3: HoleSprite,
}

impl AnimationStatus {
    const HOLE_1_X: i32 = 108;
    const HOLE_2_X: i32 = 150;
    const HOLE_3_X: i32 = 192;
    const HOLES_Y: i32 = 100;

    pub fn new(assets: &mut Assets, rng: &mut impl RandRange<u16>) -> Self {
        assets
            .adlib_player
            .set_music(crate::audio::Playing::MainMenu);
        // initialize a random creature for animation purposes
        let creature = CreatureParams::new_random(rng);
        set_creature_palette(&mut assets.palette, &creature, Default::default());
        assets.palette.set();

        crate::print_name(&creature, &assets.big_font, 46);

        // draw 3 holes
        let hole_y = Self::HOLES_Y;
        let active_hole = HoleSprite::new_at(Self::HOLE_2_X, hole_y);
        let hole_2 = HoleSprite::new_at(Self::HOLE_1_X, hole_y);
        let hole_3 = HoleSprite::new_at(Self::HOLE_3_X, hole_y);
        active_hole.draw();
        hole_2.draw();
        hole_3.draw();

        let creature_sprite = CreatureSprite::new(
            &creature,
            Default::default(),
            &assets.creature_assets,
            &active_hole,
        );

        AnimationStatus {
            creature_frame: 0,
            creature,
            creature_sprite,
            active_hole,
            hole_2,
            hole_3,
        }
    }

    pub fn step(&mut self, assets: &mut Assets, rng: &mut impl RandRange<u16>) {
        self.active_hole.draw();
        self.hole_2.draw();
        self.hole_3.draw();
        // animate creature leaving and entering hole
        match self.creature_frame {
            // emerging
            0..64 => {
                self.creature_sprite.update_up(1);
                self.creature_sprite.draw(&self.active_hole);
            }
            64..200 => {
                self.creature_sprite.draw(&self.active_hole);
            }
            // leaving
            200..284 => {
                self.creature_sprite.update_down(1);
                self.creature_sprite.draw(&self.active_hole);
            }
            // rotate creature
            // so that a new one appears
            451 => {
                self.creature = CreatureParams::new_random(rng);
                set_creature_palette(&mut assets.palette, &self.creature, Default::default());
                assets.palette.set();
                // select a new hole for the creature to appear in
                let new_hole = rng.next_range(0..3);
                match new_hole {
                    0 => {
                        self.active_hole = HoleSprite::new_at(Self::HOLE_1_X, Self::HOLES_Y);
                    }
                    1 => {
                        self.active_hole = HoleSprite::new_at(Self::HOLE_2_X, Self::HOLES_Y);
                    }
                    2 => {
                        self.active_hole = HoleSprite::new_at(Self::HOLE_3_X, Self::HOLES_Y);
                    }
                    _ => unreachable!(),
                }
                self.creature_sprite = CreatureSprite::new(
                    &self.creature,
                    Default::default(),
                    &assets.creature_assets,
                    &self.active_hole,
                );

                // rewrite creature name
                assets.background.draw_rect(20, 46, 280, 16);
                crate::print_name(&self.creature, &assets.big_font, 46);

                self.creature_frame = 0;
                // do not do the increment step below
                return;
            }
            // nothing
            _ => { /* no-op */ }
        }
        self.creature_frame += 1;
    }
}

/// Menu logic for what happens after a difficulty is selected.
///
/// The creatures to whack and grab are presented.
fn submenu_present(assets: &mut Assets, round: &RoundOptions) {
    // TODO change background or something
    assets.background.draw_all();

    let creature1 = &round.whack;
    let creature2 = &round.grab;
    set_creature_palette(&mut assets.palette, &creature1, CreatureIndex::Whack);
    set_creature_palette(&mut assets.palette, &creature2, CreatureIndex::Grab);

    assets.palette.set();

    let text1 = format!("Whack {creature1}!");
    assets.big_font.draw_text(39, 50, &text1, COLOR_WHITE);
    assets.big_font.draw_text(40, 51, &text1, COLOR_BLACK);

    assets
        .creature_assets
        .draw_creature(&creature1, CreatureIndex::Whack, 144, 68);

    let text2 = format!("Grab {creature2}!");
    assets.big_font.draw_text(39, 115, &text2, COLOR_WHITE);
    assets.big_font.draw_text(40, 116, &text2, COLOR_BLACK);
    assets
        .creature_assets
        .draw_creature(&creature2, CreatureIndex::Grab, 144, 128);

    input::flip_keystate();

    loop {
        unsafe {
            vsync();
        }

        assets.adlib_player.process();

        if input::is_select_pressed() {
            return;
        }

        input::flip_keystate();
    }
}
