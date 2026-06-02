use core::cell::Cell;

use dos_x::{
    adlib,
    djgpp::{
        dos::delay,
        pc::{inportb, outportb},
    },
};
use opbinary::vgm::OplCommand;

static mut NO_SOUND: bool = false;
static mut NO_MUSIC: bool = false;

static MUSIC_VGM_MENU: &[u8] = include_bytes!("../resources/createac.vgm");
static MUSIC_VGM_GAME: &[u8] = include_bytes!("../resources/whackac.vgm");
static MUSIC_VGM_OVER: &[u8] = include_bytes!("../resources/gameover.vgm");

// Hz
const PIT_FREQUENCY: u32 = 0x1234DD;

/// disable sound effects
pub fn sound_off() {
    unsafe {
        NO_SOUND = true;
    }
}

/// disable music
pub fn music_off() {
    unsafe {
        NO_MUSIC = true;
    }
}

/// Play a very short click sound
pub fn play_click_1() {
    play_click_impl(1800, 2);
}

/// Play a click sound
pub fn play_click_2() {
    play_click_impl(1500, 4);
}

#[inline]
fn play_click_impl(countdown: u16, duration_ms: u32) {
    if unsafe { NO_SOUND } {
        return;
    }

    // use PC speaker
    unsafe {
        pc_speaker_on();

        play_note(countdown);
        delay(duration_ms);

        // turn off
        pc_speaker_off();
    }
}

#[inline]
unsafe fn play_note(countdown: u16) {
    unsafe {
        outportb(0x42, (countdown & 0xff) as u8);
        outportb(0x42, (countdown >> 8) as u8);
    }
}

#[inline]
unsafe fn pc_speaker_on() {
    unsafe {
        let inb = inportb(0x61);
        outportb(0x61, inb | 3); // enable speaker
        outportb(0x43, 0xb6); // set PIT
    }
}

#[inline(always)]
unsafe fn pc_speaker_off() {
    unsafe {
        let inb = inportb(0x61);
        outportb(0x61, inb & 0xfc);
    }
}

#[inline]
pub fn adlib_notes_off() {
    unsafe {
        for reg in 0xB0..0xB8 {
            adlib::write_command(reg, 0);
        }
        for reg in 0xC0..0xC8 {
            adlib::write_command(reg, 0);
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PlaybackState {
    Playing,
    Stopped,
}

/// The loaded OPL sequences of all music in the game.
struct BgmSet {
    /// Main menu
    menu: opbinary::vgm::OplVgm,
    /// In-game
    game: opbinary::vgm::OplVgm,
    /// Game over
    over: opbinary::vgm::OplVgm,
}

/// Selector for the music to play
#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub enum Playing {
    #[default]
    None,
    MainMenu,
    InGame,
    GameOver,
}

/// An async processor of Adlib music playback.
pub struct AdlibPlayer {
    vgm: Option<BgmSet>,
    cmd_index: core::cell::Cell<usize>,
    timer: core::cell::Cell<u32>,
    repeat: bool,
    playing: Playing,
}

fn samples_to_us(samples: u32) -> u32 {
    // VGM standard sample rate is 44100 Hz
    (samples * 10_000) / 441
}

impl AdlibPlayer {
    pub fn load() -> Self {
        if unsafe { NO_MUSIC } {
            return AdlibPlayer {
                vgm: None,
                cmd_index: Cell::new(0),
                timer: Cell::new(0),
                repeat: false,
                playing: Playing::None,
            };
        }

        let vgm_menu = opbinary::vgm::Vgm::from_bytes(MUSIC_VGM_MENU)
            .expect("Could not read embedded VGM data for main menu music");
        let vgm_game = opbinary::vgm::Vgm::from_bytes(MUSIC_VGM_GAME)
            .expect("Could not read embedded VGM data for in-game music");
        let vgm_over = opbinary::vgm::Vgm::from_bytes(MUSIC_VGM_OVER)
            .expect("Could not read embedded VGM data for game over music");

        AdlibPlayer {
            vgm: Some(BgmSet {
                menu: vgm_menu.into_opl_vgm(),
                game: vgm_game.into_opl_vgm(),
                over: vgm_over.into_opl_vgm(),
            }),
            cmd_index: Cell::new(0),
            timer: Cell::new(0),
            repeat: true,
            playing: Playing::MainMenu,
        }
    }

    /// Select the music to play,
    /// resetting if it is different.
    pub fn set_music(&mut self, playing: Playing) {
        if self.playing == playing || self.vgm.is_none() {
            return;
        }
        self.playing = playing;
        if playing == Playing::None {
            // stop playing
        }
        self.cmd_index = Cell::new(0);
        self.timer = Cell::new(0);
        // do not loop game over music
        self.repeat = playing != Playing::GameOver;
    }

    /// Process OPL commands to perform at this time
    pub fn process(&self) -> PlaybackState {
        // !!! this is currently assuming that this function is called at a steady rate,
        // synchronized from somewhere else.
        // Prefer using the PIT timer or something.
        self.poll_with_time(14_400)
    }

    pub fn poll_with_time(&self, delta_microseconds: u32) -> PlaybackState {
        let vgm = match (&self.vgm, self.playing) {
            (None, _) | (_, Playing::None) => return PlaybackState::Stopped,
            (Some(vgm), Playing::MainMenu) => &vgm.menu,
            (Some(vgm), Playing::InGame) => &vgm.game,
            (Some(vgm), Playing::GameOver) => &vgm.over,
        };

        let timer = self.timer.get().saturating_sub(delta_microseconds);
        self.timer.set(timer);
        let mut cmd_index = self.cmd_index.get();

        while self.timer.get() == 0 && cmd_index < vgm.opl_commands.len() {
            let cmd = &vgm.opl_commands[cmd_index];
            match cmd {
                OplCommand::Opl3 {
                    port: 0,
                    address,
                    data,
                } => unsafe {
                    adlib::write_command_l(*address, *data);
                },
                OplCommand::Opl3 {
                    port: 1,
                    address,
                    data,
                } => unsafe {
                    adlib::write_command_r(*address, *data);
                },
                OplCommand::Opl2 { address, data }
                | OplCommand::Opl3 {
                    port: _,
                    address,
                    data,
                } => unsafe {
                    adlib::write_command(*address, *data);
                },
                OplCommand::Wait { samples } => {
                    self.timer.set(samples_to_us(*samples as u32));
                }
                OplCommand::SmallWait { n } => {
                    self.timer.set(samples_to_us(*n as u32 + 1));
                }
                OplCommand::Wait735 => {
                    self.timer.set(samples_to_us(735));
                }
                OplCommand::Wait882 => {
                    self.timer.set(samples_to_us(882));
                }
            }
            cmd_index += 1;
            self.cmd_index.set(cmd_index);
        }
        let cmd_index = self.cmd_index.get();
        if cmd_index >= vgm.opl_commands.len() && self.repeat {
            // loop
            self.cmd_index.set(cmd_index - vgm.opl_commands.len());
        }
        PlaybackState::Playing
    }
}

/// Initialize the Adlib music player,
/// loading the game music if music is enabled.
///
/// If music is disabled, the returned dummy player does nothing.
pub fn load_player() -> AdlibPlayer {
    // load OPL data of music
    let mut player = AdlibPlayer::load();

    // patch the music a bit
    if let Some(BgmSet { menu, game, .. }) = &mut player.vgm {
        // add a small waiting time at the end to avoid abrupt cut-off in the loop
        menu.opl_commands.push(OplCommand::Wait { samples: 2210 });
        game.opl_commands.push(OplCommand::Wait { samples: 4380 });
    }

    player
}
