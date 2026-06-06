//! How to play page

use alloc::format;
use dos_x::vga::vsync;
use tinyrand::RandRange;

use crate::{
    Assets,
    creature::CreatureParams,
    gameplay::CreatureIndex,
    gfx::{
        GloveFrame, ImageAsset, background::Background, draw_arrow_right, draw_glove,
        set_background_palette, set_creature_palette,
    },
    input::{self, Key},
};

pub fn how_to_play(assets: &mut Assets, rng: &mut impl RandRange<u16>) {
    // load glove
    let glove = ImageAsset::load_glove();

    // set background to back0, draw it
    assets.background = Background::back0();
    set_background_palette(&mut assets.palette, &assets.background);
    assets.palette.set();
    assets.background.draw_all();

    // write "How to Play" text

    assets.big_font.draw_text(55, 10, "How To Play", 1);

    assets
        .small_font
        .draw_text(36, 40, "Position your hand with your directional keys", 1);

    assets.small_font.draw_text(36, 50, "Press Z to whack", 1);

    assets.small_font.draw_text(36, 60, "Press X to grab", 1);

    let mut creature1 = CreatureParams::default();
    set_creature_palette(&mut assets.palette, &creature1, CreatureIndex::Whack);
    let mut creature2 = CreatureParams::default();
    set_creature_palette(&mut assets.palette, &creature2, CreatureIndex::Grab);

    let mut ticks_to_proceed: u16 = 80;
    let mut k = 0;
    loop {
        unsafe {
            vsync();
        }
        if k == 0 {
            // randomize creature
            creature1 = CreatureParams::new_random(rng);
            set_creature_palette(&mut assets.palette, &creature1, CreatureIndex::Whack);
            creature2 = CreatureParams::new_random(rng);
            set_creature_palette(&mut assets.palette, &creature2, CreatureIndex::Grab);
            assets.palette.set();

            // update instructions
            assets.background.draw_rect(20, 70, 280, 120);

            assets
                .small_font
                .draw_text(36, 94, &format!("If you're told to whack {creature1}"), 1);
            assets
                .small_font
                .draw_text(50, 106, &format!("then only whack {creature1}!"), 1);
            assets
                .small_font
                .draw_text(50, 118, &format!("(do not whack {creature2})"), 1);

            assets
                .small_font
                .draw_text(36, 140, &format!("If you're told to grab {creature2}"), 1);
            assets
                .small_font
                .draw_text(50, 152, &format!("then only grab {creature2}!"), 1);
            assets
                .small_font
                .draw_text(50, 164, &format!("(do not grab {creature1})"), 1);

            draw_arrow_right(246, 96, 1);
            assets
                .creature_assets
                .draw_creature(&creature1, CreatureIndex::Whack, 258, 81);
            draw_glove(&glove, 266, 70, GloveFrame::Whack);

            draw_arrow_right(246, 140, 1);
            assets
                .creature_assets
                .draw_creature(&creature2, CreatureIndex::Grab, 258, 129);
            draw_glove(&glove, 266, 124, GloveFrame::Grab);
            k = 820;
        } else {
            k -= 1;
        }

        // process audio
        assets.adlib_player.process();

        // only allow user interactions after a few ticks
        ticks_to_proceed = ticks_to_proceed.saturating_sub(1);
        if ticks_to_proceed > 0 {
            continue;
        }

        // handle keys
        if input::is_select_pressed() || input::is_pressed(Key::Back) {
            return;
        }
        input::flip_keystate();
    }
}
