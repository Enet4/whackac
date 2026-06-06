//! Module for the in-game activity

use alloc::{
    format,
    string::{String, ToString as _},
};
use core::fmt::Write;
use dos_x::vga::{self, Palette, vsync};
use tinyrand::RandRange;

use crate::{
    Assets,
    audio::{play_click_1, play_click_2},
    gameplay::{CreatureIndex, Difficulty, HoleIndex, HoleStatus, RoundOptions, RoundState, Table},
    gfx::{
        COLOR_BEIGE, COLOR_BLACK, COLOR_HIGHLIGHT, COLOR_WHITE, CreatureSprite, GloveFrame,
        HoleSprite, ImageAsset, background::Background, draw_glove, fade_in, init_palette,
        set_background_palette, set_creature_palette,
    },
    input::{self, Key},
};

#[derive(Debug)]
pub enum InGameOutcome {
    /// Just return to the main menu
    Exit,
    /// The game is over, stats returned
    GameOver(Stats),
}

/// a composition of the complete stats of a game
#[derive(Debug, Default)]
pub struct Stats {
    /// total score
    score: u16,
    whacked_1: u16,
    whacked_2: u16,
    whacked_miss: u16,
    whacked_distract: u16,
    grabbed_1: u16,
    grabbed_2: u16,
    grabbed_miss: u16,
    grabbed_distract: u16,
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum GamePhase {
    Running,
    Paused,
    Over,
    Stats,
}
impl GamePhase {
    pub fn toggle_pause(&mut self) {
        match self {
            Self::Running => *self = Self::Paused,
            Self::Paused => *self = Self::Running,
            _ => {}
        }
    }
}

pub fn game_round(
    assets: &mut Assets,
    rng: &mut impl RandRange<u16>,
    round: RoundOptions,
) -> InGameOutcome {
    let mut round = RoundState::new(round, &mut *rng);

    // load all the necessary assets
    let glove = ImageAsset::load_glove();

    assets.adlib_player.set_music(crate::audio::Playing::InGame);
    assets.background = match round.options.difficulty {
        Difficulty::Easy => Background::back2(),
        Difficulty::Normal => Background::back3(),
        Difficulty::Hard => Background::back4(),
    };
    // initialize palette
    let mut target_palette = Palette::new([0; 768]);
    init_palette(&mut target_palette);
    set_background_palette(&mut target_palette, &assets.background);
    set_creature_palette(
        &mut target_palette,
        &round.options.whack,
        CreatureIndex::Whack,
    );
    set_creature_palette(
        &mut target_palette,
        &round.options.grab,
        CreatureIndex::Grab,
    );
    if let Some(distraction1) = &round.options.distraction1 {
        set_creature_palette(&mut target_palette, distraction1, CreatureIndex::Other1);
    }
    if let Some(distraction2) = &round.options.distraction2 {
        set_creature_palette(&mut target_palette, distraction2, CreatureIndex::Other2);
    }
    if let Some(distraction3) = &round.options.distraction3 {
        set_creature_palette(&mut target_palette, distraction3, CreatureIndex::Other3);
    }

    assets.palette.set();
    let mut table = Table::new((114, 86));

    // x, y, w, h of the live portion
    let live_dimensions = (92, 52, 128, 110);

    // draw scenery and fade in
    assets.background.draw_all();

    let mut hole_sprites = [HoleSprite::new_at(0, 0); 9];

    // draw the holes,
    // then copy video buffer to the background
    // so we don't need to keep redrawing them
    for i in HoleIndex::all() {
        let pos = table.hole_pos(i);
        // draw
        let sprite = HoleSprite::new_at(pos.0 as i32, pos.1 as i32);
        hole_sprites[i.to_usize()] = sprite;
        sprite.draw();
    }

    // prepare the creature sprite buffers
    let mut creature_sprites = unsafe { [core::mem::zeroed(); 9] };
    for i in 0..9 {
        creature_sprites[i] = CreatureSprite::new_unset(&hole_sprites[i])
    }

    unsafe {
        vga::read_video_buffer(&mut assets.background.0.pixel_data);
    }
    // fade into the game
    fade_in(&mut assets.palette, &target_palette);

    // roll keypresses so that they are disregarded in the first loop
    input::flip_keystate();

    // update ticks
    let mut ticks = 0;

    // sub-ticks (4ths of a tick)
    let mut subticks: u8 = 0;

    let mut phase = GamePhase::Running;

    // score
    let mut stats = Stats::default();
    let mut score_text = String::with_capacity(16);
    write!(score_text, "Score: {}", stats.score).unwrap_or_else(|_| {
        score_text = "Score: ?".to_string();
    });

    assets
        .small_font
        .draw_text(12, 10, &score_text, COLOR_BLACK);

    let mut update_score = |score: &mut u16, delta: i16| {
        *score = (*score as i32 + delta as i32).max(0) as u16;
        score_text.truncate("Score: ".len());
        let _ = write!(score_text, "{}", *score);
        assets.background.draw_rect(10, 9, 68, 8);
        assets
            .small_font
            .draw_text(12, 10, &score_text, COLOR_BLACK);
    };

    // the player cursor
    let (mut pos_1_x, mut pos_1_y) = (0, 0);
    // state of whacking (with frame)
    let mut whacking: Option<i8> = None;
    // state of grabbing (with frame)
    let mut grabbing: Option<i8> = None;

    loop {
        unsafe {
            vsync();
        }

        if phase != GamePhase::Stats {
            // fresh-draw the lively part of the game
            assets.background.draw_rect(
                live_dimensions.0,
                live_dimensions.1,
                live_dimensions.2,
                live_dimensions.3,
            );
        }

        if phase == GamePhase::Running {
            // draw creatures
            for ((hole, hole_sprite), creature_sprite) in table
                .holes_mut()
                .into_iter()
                .zip(&hole_sprites)
                .zip(&mut creature_sprites)
            {
                match hole {
                    crate::gameplay::HoleStatus::Empty => {
                        // hole_sprite.draw();
                    }
                    crate::gameplay::HoleStatus::Shown { creature, frame } => {
                        // draw the creature on the hole
                        if *frame == 0 {
                            creature_sprite.update(32);
                        }
                        creature_sprite.draw(&hole_sprite);
                        *frame += 1;

                        // make it hide after a while
                        if subticks == 0 {
                            let range = 0..frame.saturating_add(24);
                            let n = rng.next_range(range);
                            if n > round.options.avg_idle_time {
                                *hole = HoleStatus::Hiding {
                                    creature: *creature,
                                    frame: 0,
                                };
                            }
                        }
                    }
                    crate::gameplay::HoleStatus::Appearing { creature, frame } => {
                        // draw the creature hiding into the hole
                        if *frame == 0 {
                            let c = round
                                .options
                                .creature_of(*creature)
                                .expect("missing parameters for creature");
                            creature_sprite.set_creature(c, *creature, &assets.creature_assets);
                        }
                        creature_sprite.update(*frame as i16 * 2);
                        creature_sprite.draw(&hole_sprite);
                        *frame += 1;
                        if *frame == 16 {
                            *hole = crate::gameplay::HoleStatus::Shown {
                                creature: *creature,
                                frame: *frame,
                            };
                        }
                    }
                    crate::gameplay::HoleStatus::Hiding { creature, frame } => {
                        if *frame == 0 {
                            // draw the creature hiding into the hole
                            let c = round
                                .options
                                .creature_of(*creature)
                                .expect("missing parameters for creature");
                            creature_sprite.set_creature(c, *creature, &assets.creature_assets);
                        }
                        creature_sprite.update(32 - *frame as i16 * 2);
                        creature_sprite.draw(&hole_sprite);
                        *frame += 1;
                        if *frame == 16 {
                            *hole = crate::gameplay::HoleStatus::Empty;
                        }
                    }
                }
            }

            // draw glove
            match pos_1_y {
                -1 => {
                    let x = 112 + (pos_1_x + 1) as i32 * Table::HOLE_STRIDE_X as i32;
                    const Y: i32 = 84 + 0 * Table::HOLE_STRIDE_Y as i32 - 6;
                    let (x, y, frame) = match (whacking, grabbing) {
                        (None, None) => (x, Y, GloveFrame::Idle),
                        (Some(step), None) => {
                            (x - 2, Y - 10 + step.min(6) as i32 * 2, GloveFrame::Whack)
                        }
                        (None, Some(step)) => {
                            (x - 2, Y - 6 - step.min(6) as i32 * 2, GloveFrame::Grab)
                        }
                        (Some(_), Some(_)) => {
                            unreachable!("Whacking _and_ grabbing!? In _this_ economy??")
                        }
                    };
                    draw_glove(&glove, x, y, frame);
                }
                0 => {
                    let x = 112 + (pos_1_x + 1) as i32 * Table::HOLE_STRIDE_X as i32;
                    const Y: i32 = 84 + Table::HOLE_STRIDE_Y as i32 - 6;
                    let (x, y, frame) = match (whacking, grabbing) {
                        (None, None) => (x, Y, GloveFrame::Idle),
                        (Some(step), None) => {
                            (x - 2, Y - 10 + step.min(7) as i32 * 2, GloveFrame::Whack)
                        }
                        (None, Some(step)) => {
                            (x - 2, Y - 6 - step.min(7) as i32 * 2, GloveFrame::Grab)
                        }
                        (Some(_), Some(_)) => {
                            unreachable!("Whacking _and_ grabbing!? In _this_ economy??")
                        }
                    };
                    draw_glove(&glove, x, y, frame);
                }
                1 => {
                    let x = 112 + (pos_1_x + 1) as i32 * Table::HOLE_STRIDE_X as i32;
                    const Y: i32 = 84 + 2 * Table::HOLE_STRIDE_Y as i32 - 6;
                    let (x, y, frame) = match (whacking, grabbing) {
                        (None, None) => (x, Y, GloveFrame::Idle),
                        (Some(step), None) => {
                            (x - 2, Y - 10 + step.min(7) as i32 * 2, GloveFrame::Whack)
                        }
                        (None, Some(step)) => {
                            (x - 2, Y - 6 - step.min(7) as i32 * 2, GloveFrame::Grab)
                        }
                        (Some(_), Some(_)) => {
                            unreachable!("Whacking _and_ grabbing!? In _this_ economy??")
                        }
                    };
                    draw_glove(&glove, x, y, frame);
                }
                _ => { /* no-op */ }
            }

            // game loop, do stuff as it goes

            while let Some(ev) = round.pop_event(ticks) {
                match ev.kind {
                    crate::gameplay::EventKind::Appear(creature) => {
                        let Some(hole) = table.pick_empty(&mut *rng) else {
                            // skip, could not spawn the creature :(
                            continue;
                        };

                        // update the table accordingly
                        table.put(hole, creature);
                    }
                }
            }

            // process ongoing whacks and grabs

            if let Some(f) = &mut whacking {
                *f += 1;
                if *f == 5 {
                    // process the hit
                    let hole = table.hole_at_mut(pos_1_x, pos_1_y);
                    match hole {
                        HoleStatus::Appearing { creature, .. }
                        | HoleStatus::Hiding { creature, .. }
                        | HoleStatus::Shown { creature, .. } => {
                            if *creature == CreatureIndex::Whack {
                                // right, score!
                                update_score(&mut stats.score, 2);
                                stats.whacked_1 += 1;
                                play_click_2(); // TODO play proper sound

                                *hole = HoleStatus::Empty;
                            } else {
                                // wrong creature, penalty!
                                update_score(&mut stats.score, -1);
                                if *creature == CreatureIndex::Grab {
                                    stats.whacked_2 += 1;
                                } else {
                                    stats.whacked_distract += 1;
                                }
                                play_click_1(); // TODO play proper sound

                                *hole = HoleStatus::Empty;
                            }
                        }
                        HoleStatus::Empty => {
                            // derp
                            stats.whacked_miss += 1;
                        }
                    }
                }
                // reset after a few more frames for recoil
                if *f >= 10 {
                    whacking = None;
                }
            } else if let Some(f) = &mut grabbing {
                *f += 1;
                if *f == 6 {
                    // process the grab
                    let hole = table.hole_at_mut(pos_1_x, pos_1_y);
                    match hole {
                        HoleStatus::Appearing { creature, .. }
                        | HoleStatus::Hiding { creature, .. }
                        | HoleStatus::Shown { creature, .. } => {
                            if *creature == CreatureIndex::Grab {
                                // right, score!
                                update_score(&mut stats.score, 2);
                                stats.grabbed_2 += 1;
                                play_click_2(); // TODO play proper sound

                                *hole = HoleStatus::Empty;
                            } else {
                                // wrong creature, penalty!
                                update_score(&mut stats.score, -1);
                                if *creature == CreatureIndex::Grab {
                                    stats.grabbed_1 += 1;
                                } else {
                                    stats.grabbed_distract += 1;
                                }
                                play_click_1(); // TODO play proper sound

                                *hole = HoleStatus::Empty;
                            }
                        }
                        HoleStatus::Empty => {
                            // derp
                            stats.grabbed_miss += 1;
                        }
                    }
                }
                // reset after a few more frames for recoil
                if *f >= 12 {
                    grabbing = None;
                }
            }
        } // end if running

        if phase != GamePhase::Paused {
            subticks += 1;
            if subticks >= 4 {
                ticks += 1;
                subticks = 0;
            }
        } else {
            // paused
            assets.big_font.draw_text(101, 85, "PAUSED", COLOR_WHITE);
            assets.big_font.draw_text(102, 86, "PAUSED", COLOR_BLACK);
            assets
                .small_font
                .draw_text(94, 154, "Press 'Y' to give up", COLOR_BLACK);
        }

        if ticks == RoundState::ROUND_LENGTH {
            // game over!
            phase = GamePhase::Over;
            assets
                .adlib_player
                .set_music(crate::audio::Playing::GameOver);
        } else if (RoundState::ROUND_LENGTH..RoundState::ROUND_LENGTH + 80).contains(&ticks) {
            // TIME UP!
            assets.big_font.draw_text(81, 91, "Time's Up!", COLOR_BLACK);
            assets
                .big_font
                .draw_text(82, 92, "Time's Up!", COLOR_HIGHLIGHT);
        } else if ticks == RoundState::ROUND_LENGTH + 80 {
            assets.background.draw_all();
            // show the stats a small bit after the round ends
            draw_stats(&assets, &stats, &round.options, ticks);
            phase = GamePhase::Stats;
        } else if ticks > RoundState::ROUND_LENGTH + 80 {
            // keep drawing the stats
            draw_stats(&assets, &stats, &round.options, ticks);
        }

        // process music
        assets.adlib_player.process();

        // handle input

        if input::is_pressed(Key::Back) {
            // pause/resume
            phase.toggle_pause();
        }
        if phase == GamePhase::Paused && input::is_pressed(Key::Confirm) {
            return InGameOutcome::Exit;
        }
        if phase == GamePhase::Stats
            && (input::is_pressed(Key::Confirm) || input::is_select_pressed())
        {
            return InGameOutcome::GameOver(stats);
        }

        let up_1 = input::is_down(Key::Up1) || input::is_down(Key::Up2);
        let down_1 = input::is_down(Key::Down1) || input::is_down(Key::Down2);
        let left_1 = input::is_down(Key::Left1) || input::is_down(Key::Left2);
        let right_1 = input::is_down(Key::Right1) || input::is_down(Key::Right2);

        if phase == GamePhase::Running {
            if whacking.is_none() && grabbing.is_none() {
                // get position of the player's cursor
                pos_1_y = down_1 as i8 - up_1 as i8;
                pos_1_x = right_1 as i8 - left_1 as i8;
                if input::is_pressed(Key::Whack1) || input::is_pressed(Key::Whack2) {
                    // start the whack
                    whacking = Some(0);
                } else if input::is_pressed(Key::Grab1) || input::is_pressed(Key::Grab2) {
                    // start the grab
                    grabbing = Some(0);
                }
            }
        }
        input::flip_keystate();
    }
}

fn draw_stats(assets: &Assets, stats: &Stats, round_options: &RoundOptions, ticks: u16) {
    const MARGIN_X: i32 = 60;
    const MARGIN_Y: i32 = 44;
    const CORNER_1_X: i32 = MARGIN_X;
    const CORNER_1_Y: i32 = MARGIN_Y;
    const CORNER_2_X: i32 = 320 - MARGIN_X;
    const CORNER_2_Y: i32 = 200 - MARGIN_Y;
    const W: u32 = 320 - MARGIN_X as u32 - MARGIN_X as u32;
    const H: u32 = 200 - MARGIN_Y as u32 - MARGIN_Y as u32;
    let c1 = COLOR_BLACK;
    let c2 = COLOR_BEIGE;
    // draw a rectangle with an outline
    unsafe {
        dos_x::vga::draw_hline(CORNER_1_X, CORNER_1_Y, W, c1);
        dos_x::vga::draw_vline(CORNER_1_X, CORNER_1_Y + 1, H, c1);
        dos_x::vga::draw_vline(CORNER_2_X, CORNER_1_Y, H, c1);
        dos_x::vga::draw_hline(CORNER_1_X + 1, CORNER_1_Y + 1, W - 1, COLOR_WHITE);
        dos_x::vga::draw_vline(CORNER_1_X + 1, CORNER_1_Y + 1, H - 1, COLOR_WHITE);
        dos_x::vga::draw_rect(CORNER_1_X + 2, CORNER_1_Y + 2, W - 2, H - 2, c2);
        dos_x::vga::draw_hline(CORNER_1_X, CORNER_2_Y, W + 1, c1);
    }

    // draw the stats
    assets.big_font.draw_text(74, 50, "Your Stats", COLOR_BLACK);

    assets.small_font.draw_text(
        68,
        80,
        &format!("Whacked {}: {}", round_options.whack, stats.whacked_1),
        COLOR_BLACK,
    );
    assets.small_font.draw_text(
        68,
        94,
        &format!("Grabbed {}: {}", round_options.grab, stats.grabbed_2),
        COLOR_BLACK,
    );

    let mistakes =
        stats.whacked_2 + stats.grabbed_1 + stats.whacked_distract + stats.grabbed_distract;
    assets
        .small_font
        .draw_text(68, 108, &format!("Mistakes: {}", mistakes), COLOR_BLACK);

    assets.small_font.draw_text(
        68,
        136,
        &format!("Total score: {}", stats.score),
        COLOR_BLACK,
    );

    let weak_score = round_options.num_creatures / 2;
    let good_score = round_options.num_creatures * 16 / 11;

    let msg = if stats.score == 0 && mistakes > 0 {
        Some("Did you understand your assignment?")
    } else if stats.score < weak_score {
        match round_options.difficulty {
            Difficulty::Easy => Some("Oof! What was that about?"),
            Difficulty::Normal => Some("They really got you, huh..."),
            Difficulty::Hard => Some("Tough luck!"),
        }
    } else if stats.score >= good_score {
        match round_options.difficulty {
            Difficulty::Easy => Some("OK then! Increase the difficulty!"),
            Difficulty::Normal => Some("Good job! :)"),
            Difficulty::Hard => Some("You're a legend!"),
        }
    } else {
        None
    };

    if let Some(msg) = msg {
        let center_x = 160 - msg.len() as i32 * 3;
        let c = if (ticks & 4) == 0 {
            COLOR_HIGHLIGHT
        } else {
            COLOR_BLACK
        };
        assets.small_font.draw_text(center_x, 180, &msg, c);
    }
}
