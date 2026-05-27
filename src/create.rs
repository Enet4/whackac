//! The create-a-creature mode

use dos_x::{
    key,
    vga::{Palette, clear_screen, draw_rect, vsync},
};

use crate::{
    Assets,
    audio::{play_click_1, play_click_2},
    creature::CreatureParams,
    gfx::{COLOR_BACKGROUND, COLOR_BLACK, draw_arrow_left, draw_arrow_right, set_creature_palette},
};

/// What the game should do as the level ends
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum CreateCreatureOutcome {
    /// Accept the creature and return to the main menu with it!
    SaveCreature,
    /// Exit the game immediately
    Exit,
}

/// the function holding the main game stuff
pub fn create_a_creature_game(
    assets: &Assets,
    creature: &mut CreatureParams,
    palette: &mut Palette,
) -> CreateCreatureOutcome {
    unsafe {
        vsync();
    }

    unsafe {
        clear_screen(253);
    }

    let Assets {
        adlib_player,
        creature_assets,
        small_font,
        ..
    } = assets;

    let creature_x = (320 - 32) / 2;
    let creature_y = (200 - 32) / 2;

    // draw the creature in the center of the screen
    creature_assets.draw_creature(creature, Default::default(), creature_x, creature_y);

    const BASE_Y: i32 = 28;

    let stride = 24;

    // draw the UI
    small_font.draw_text(48, BASE_Y, "Shape", COLOR_BLACK);
    small_font.draw_text(48, BASE_Y + stride, "Color", COLOR_BLACK);
    small_font.draw_text(48, BASE_Y + stride * 2, "Eyes", COLOR_BLACK);
    small_font.draw_text(48, BASE_Y + stride * 3, "Mouth", COLOR_BLACK);
    small_font.draw_text(48, BASE_Y + stride * 4, "Legs", COLOR_BLACK);
    small_font.draw_text(48, BASE_Y + stride * 5, "Arms", COLOR_BLACK);
    small_font.draw_text(48, BASE_Y + stride * 6 + 8, "Done!", COLOR_BLACK);

    // selector for different parameters/actions:
    // 0: param0 (shape)
    // 1: param1 (color)
    // 2: param2 (eye type)
    // 3: param3 (limbs)
    // 4: param4 (mouth)
    // 5: Done!
    let mut selector = 0;

    let mut keystate_up = false;
    let mut keystate_down = false;
    let mut keystate_left = false;
    let mut keystate_right = false;

    // the game loop
    loop {
        unsafe {
            vsync();
        }

        const ARROW_LEFT: u32 = 28;
        const ARROW_RIGHT: u32 = 124;
        // clear regions with selection arrow
        unsafe {
            dos_x::vga::draw_rect(ARROW_LEFT as i32, BASE_Y, 7, 160, COLOR_BACKGROUND);
            dos_x::vga::draw_rect(ARROW_RIGHT as i32, BASE_Y, 7, 160, COLOR_BACKGROUND);
        }
        let selection_y = BASE_Y as u32 + selector as u32 * 24;
        let selection_y = if selector == 6 {
            selection_y + 9
        } else {
            selection_y
        };
        draw_arrow_left(ARROW_LEFT, selection_y, COLOR_BLACK);
        draw_arrow_right(ARROW_RIGHT, selection_y, COLOR_BLACK);

        // detect Left, Right, Up, Down key presses
        // (W, A, S, D also works)
        let mut params_changed = false;
        let key = key::get_keypress();
        match key {
            // up
            0x48 | 0x11 => {
                if !keystate_up {
                    keystate_up = true;
                    if selector > 0 {
                        // move selection up
                        selector -= 1;
                        play_click_1();
                    }
                }
            }
            // up release (0x80 bit set)
            0xc8 | 0x91 => {
                keystate_up = false;
            }
            // down
            0x50 | 0x1f => {
                if !keystate_down {
                    keystate_down = true;
                    if selector < 5 {
                        // move selection down
                        selector += 1;
                        play_click_1();
                    }
                }
            }
            // down release
            0xd0 | 0x9f => {
                keystate_down = false;
            }

            // left
            0x4b | 0x1e => {
                if !keystate_left {
                    keystate_left = true;
                    match selector {
                        0 => {
                            // change shape (rotate backwards)
                            if creature.shape == 0 {
                                creature.shape = (crate::creature::NUM_SHAPES as u8) - 1;
                            } else {
                                creature.shape -= 1;
                            }
                            params_changed = true;
                            play_click_2();
                        }
                        1 => {
                            // change color
                            if creature.color == 0 {
                                creature.color = (crate::creature::NUM_COLORS as u8) - 1;
                            } else {
                                creature.color -= 1;
                            }
                            params_changed = true;
                            play_click_2();
                        }
                        2 => {
                            // change eye type
                            if creature.eyes == 0 {
                                creature.eyes = (crate::creature::NUM_EYES as u8) - 1;
                            } else {
                                creature.eyes -= 1;
                            }
                            params_changed = true;
                            play_click_2();
                        }
                        3 => {
                            // change mouth
                            if creature.mouth == 0 {
                                creature.mouth = (crate::creature::NUM_MOUTHS as u8) - 1;
                            } else {
                                creature.mouth -= 1;
                            }
                            params_changed = true;
                            play_click_2();
                        }
                        4 => {
                            // change both limbs
                            if creature.arms == 0 {
                                creature.legs = (crate::creature::NUM_LIMBS as u8) - 1;
                            } else {
                                creature.arms -= 1;
                            }
                            creature.legs = creature.arms;
                            params_changed = true;
                            play_click_2();
                        }
                        _ => {}
                    }
                }
            }
            // left release
            0xcb | 0x9e => {
                keystate_left = false;
            }

            // right
            0x4d | 0x20 => {
                if !keystate_right {
                    keystate_right = true;
                    match selector {
                        0 => {
                            // change shape
                            creature.shape = (creature.shape + 1) % crate::creature::NUM_SHAPES;
                            params_changed = true;
                            play_click_2();
                        }
                        1 => {
                            // change color
                            creature.color = (creature.color + 1) % crate::creature::NUM_COLORS;
                            params_changed = true;
                            play_click_2();
                        }
                        2 => {
                            // change eye type
                            creature.eyes = (creature.eyes + 1) % crate::creature::NUM_EYES;
                            params_changed = true;
                            play_click_2();
                        }
                        3 => {
                            // change mouth
                            creature.mouth = (creature.mouth + 1) % crate::creature::NUM_MOUTHS;
                            params_changed = true;
                            play_click_2();
                        }
                        4 => {
                            // change both limbs
                            creature.legs = (creature.legs + 1) % crate::creature::NUM_LIMBS;
                            creature.arms = creature.legs;
                            params_changed = true;
                            play_click_2();
                        }
                        _ => {}
                    }
                }
            }
            // right release
            0xcd | 0xa0 => {
                keystate_right = false;
            }

            // enter
            0x1c => {
                if selector == 5 {
                    // done!
                    play_click_2();
                    return CreateCreatureOutcome::SaveCreature;
                }
            }

            // escape to exit immediately
            0x81 => {
                play_click_1();
                return CreateCreatureOutcome::Exit;
            }

            _ => {}
        };

        if params_changed {
            // update palette
            set_creature_palette(palette, creature, Default::default());

            unsafe {
                draw_rect(creature_x, creature_y, 32, 32, COLOR_BACKGROUND);
            }
            // redraw the creature with new parameters
            creature_assets.draw_creature(creature, Default::default(), creature_x, creature_y);
        }

        adlib_player.process();
    }
}
