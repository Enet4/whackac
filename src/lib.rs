#![feature(maybe_uninit_array_assume_init)]
#![no_std]
#![no_main]
extern crate alloc;

mod audio;
mod create;
mod creature;
mod gameplay;
mod gfx;
mod howto;
mod ingame;
mod input;
mod menu;

use alloc::format;
use audio::sound_off;
use dos_x::adlib::detect_adlib;
use dos_x::djgpp::dos::delay;
use dos_x::djgpp::dpmi::{__dpmi_int, __dpmi_regs};
use dos_x::vga::Palette;

use core::panic::PanicInfo;
use dos_x::vga::vsync;
use dos_x::{djgpp::stdlib::exit, println};
use tinyrand::{RandRange, Seeded};

use crate::audio::{adlib_notes_off, enable_pc_speaker, load_player, music_off};
use crate::create::{CreateCreatureOutcome, create_a_creature_game};
use crate::creature::CreatureParams;
use crate::gameplay::RoundOptions;
use crate::gfx::background::Background;
use crate::gfx::fonts::BitmapFont;
use crate::gfx::{
    COLOR_HIGHLIGHT, COLOR_WHITE, CreatureAssets, fade_out, init_palette, set_background_palette,
    set_creature_palette,
};
use crate::howto::how_to_play;
use crate::ingame::game_round;
use crate::input::{restore_keyboard_interrupt, take_default_keyboard_interrupt};
use crate::menu::MenuOutcome;

/// 16x16 floppy disk icon, raw 8-bit indexed data
/// (already assumes game palette for B&W)
static FLOPPY_DATA: &[u8] = include_bytes!("../resources/floppy_16px.data");

/// Holder for all assets in the game,
/// so that they are readily available.
pub struct Assets {
    pub palette: Palette,
    pub creature_assets: CreatureAssets,
    pub background: Background,
    pub small_font: BitmapFont,
    pub big_font: BitmapFont,
    pub adlib_player: audio::AdlibPlayer,
}

#[derive(Debug, Clone)]
enum GameState {
    MainMenu,
    HowToPlay,
    InGame(RoundOptions),
    CreateACreature,
    PresentingCreature(CreatureParams),
}

#[unsafe(no_mangle)]
fn dos_main() {
    // process inputs
    for arg in dos_x::argv() {
        unsafe {
            let arg = core::ffi::CStr::from_ptr(*arg);
            if arg.to_bytes() == b"nosound" {
                sound_off();
                music_off();
            }
        }
    }

    // seed the RNG

    let time = dos_x::clock::get_system_clock_ticks();
    let seed = 0xc5a0_63ab_2366_2d31 ^ ((time as u64) << 32 | (time as u64));

    let rng = tinyrand::Xorshift::seed(seed);
    run(rng);
}

fn run(mut rng: impl RandRange<u16>) {
    println!("Whack-a-Creature by E_net4 (2026)");

    unsafe {
        delay(400);
    }

    // disable the mouse
    unsafe {
        let mut regs: __dpmi_regs = core::mem::zeroed();
        regs.h.ah = 2;
        __dpmi_int(0x33, &mut regs);
    }

    take_default_keyboard_interrupt();

    dos_x::vga::set_video_mode_13h();
    unsafe {
        // clear screen (background color)
        dos_x::vga::draw_rect(0, 0, 320, 200, 253);
    }

    // grab palette and apply it to VGA display
    let mut palette = Palette::new([0u8; 768]);
    init_palette(&mut palette);
    // sync palette
    palette.set();

    unsafe {
        vsync();

        // clear screen (background color)
        dos_x::vga::draw_rect(0, 0, 320, 200, 253);

        // draw floppy disk onto the screen
        // (suggesting that the game is loading)
        dos_x::vga::blit_rect(FLOPPY_DATA, (16, 16), (0, 0, 16, 16), (152, 92));

        delay(100);
    }

    match detect_adlib() {
        0 => {
            println!("No Adlib sound card detected, music disabled");
            music_off();
            // but we can enable PC speaker sound
            enable_pc_speaker();
        }
        _ => {
            println!("Adlib sound card detected");
        }
    };
    let adlib_player = load_player();

    // create a random creature in preparation for the menu animations
    let mut creature = CreatureParams::new_random(&mut rng);
    set_creature_palette(&mut palette, &creature, Default::default());

    // load creature assets
    let creature_assets = CreatureAssets::load();

    let big_font = BitmapFont::big();
    let small_font = BitmapFont::small();

    // start with background 1
    let background = Background::back1();
    set_background_palette(&mut palette, &background);
    palette.set();

    let mut assets = Assets {
        palette,
        creature_assets,
        background,
        small_font,
        big_font,
        adlib_player,
    };

    let mut state = GameState::MainMenu;
    loop {
        match &state {
            GameState::MainMenu => {
                let outcome = menu::menu(&mut assets, &mut rng);
                match outcome {
                    MenuOutcome::NewGame(options) => {
                        // silence all music channels
                        adlib_notes_off();

                        fade_out(&mut assets.palette);
                        state = GameState::InGame(options);
                    }
                    MenuOutcome::Create => {
                        state = GameState::CreateACreature;
                    }
                    MenuOutcome::HowToPlay => {
                        state = GameState::HowToPlay;
                    }
                    MenuOutcome::Exit => {
                        break;
                    }
                }
            }
            GameState::HowToPlay => {
                how_to_play(&mut assets, &mut rng);
                state = GameState::MainMenu;
                input::flip_keystate();
            }
            GameState::InGame(round) => {
                let outcome = game_round(&mut assets, &mut rng, round.clone());
                match outcome {
                    ingame::InGameOutcome::Exit => {
                        state = GameState::MainMenu;
                    }
                    ingame::InGameOutcome::GameOver(_stats) => {
                        state = GameState::MainMenu;
                    }
                }
            }
            GameState::CreateACreature => {
                let outcome = create_a_creature_game(&assets, &mut creature, &mut palette);
                match outcome {
                    CreateCreatureOutcome::Exit => break,
                    CreateCreatureOutcome::SaveCreature => {
                        state = GameState::PresentingCreature(creature.clone());
                    }
                }
            }
            GameState::PresentingCreature(creature) => {
                present_creature(&assets, &creature);
                // and return to main menu
                state = GameState::MainMenu;
            }
        }
    }

    // silence all music channels
    adlib_notes_off();

    fade_out(&mut assets.palette);

    // set back to text mode
    unsafe {
        dos_x::vga::set_video_mode(0x02);
    }

    // reset the default keyboard interrupt handler
    restore_keyboard_interrupt();

    println!("Thank you for playing!");
}

fn present_creature(assets: &Assets, creature: &CreatureParams) {
    let Assets {
        adlib_player,
        creature_assets,
        big_font,
        small_font,
        ..
    } = assets;

    let mut can_proceed = 128;
    unsafe {
        // clear screen (background color)
        dos_x::vga::draw_rect(0, 0, 320, 200, 253);
    }

    small_font.draw_text(86, 20, "You have created", gfx::COLOR_BLACK);

    // print creature name
    print_name(creature, big_font, 50);

    let mut keystate_enter = false;

    const JUMP_SPEED: i32 = 14;
    let mut var_y = 0;
    let mut speed_y = -JUMP_SPEED;
    let mut num_jumps: u16 = 0;

    // pre-render creature
    let mut creature_render = [gfx::COLOR_BACKGROUND; 32 * 32];
    creature_assets.render_creature(creature, Default::default(), &mut creature_render);
    let creature_render = &creature_render[..];

    loop {
        unsafe {
            vsync();
        }

        unsafe {
            if num_jumps < 24 {
                // clear screen in creature's place
                dos_x::vga::draw_rect(144, 71, 32, 60, 253);
            } else {
                // after some time, more creatures will appear,
                // so clear more
                dos_x::vga::draw_rect(100, 71, 114, 60, 253);
            }
        }

        // draw creature in center of screen
        unsafe {
            dos_x::vga::blit_rect(creature_render, (32, 32), (0, 0, 32, 32), (144, 89 + var_y));
        }

        if num_jumps >= 24 {
            // draw more creatures
            unsafe {
                dos_x::vga::blit_rect(creature_render, (32, 32), (0, 0, 32, 32), (100, 89 + var_y));
                dos_x::vga::blit_rect(creature_render, (32, 32), (0, 0, 32, 32), (188, 89 + var_y));
            }
        }

        speed_y += 1;
        var_y += speed_y / 5;
        if var_y > 10 {
            var_y = 10;
            speed_y = -JUMP_SPEED;
            num_jumps = (num_jumps + 1) & 0x3F;
        }

        adlib_player.process();

        if can_proceed > 0 {
            can_proceed -= 1;
            continue;
        }

        small_font.draw_text(60, 165, "Press ENTER to continue", gfx::COLOR_BLACK);

        // check for ENTER key
        let key = dos_x::key::get_keypress();
        if key == 0x1c {
            // key pressed
            keystate_enter = true;
        } else if key == 0x9c && keystate_enter {
            // key released
            break;
        }
    }
}

/// print the creature's name at the center of the screen
/// (with an exclamation point)
pub(crate) fn print_name(creature: &CreatureParams, big_font: &BitmapFont, y: i32) {
    let text = format!("{creature}!");

    // centered
    let x = (320 - (text.len() as i32 * 17)) / 2;
    big_font.draw_text(x - 1, y, &text, COLOR_WHITE);
    big_font.draw_text(x, y + 1, text, COLOR_HIGHLIGHT);
}

#[panic_handler]
fn handle_panic(info: &PanicInfo) -> ! {
    unsafe {
        // reset video mode
        dos_x::vga::set_video_mode(0x02);
        println!("Program aborted: {}", info);
        println!("This is likely a bug! Please reach out:");
        println!("    https://github.com/Enet4/whackac/issues/new");

        // try to recover keyboard interrupt
        restore_keyboard_interrupt();

        // exit using libc
        exit(-1);
        core::hint::unreachable_unchecked()
    }
}
