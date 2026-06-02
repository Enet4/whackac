//! Module for keyboard input handling
//!
//! In particular, this creates a common abstraction
//! that detect triggers such as keydown and keyup,
//! and lets me know whether a particular key is being held.
//!
//! This could also implement key mappings!

use core::ffi::c_void;

use dos_x::{
    djgpp::{self, dpmi::_go32_dpmi_lock_code, go32::_go32_my_cs, pc::outportb},
    key, println,
};

/// Game key, maps to the key state array
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Key {
    /// Key to select an option or proceed,
    /// usually Enter
    Select = 0,
    /// Go back or exit,
    /// usually Esc
    Back = 1,
    /// Up for player 1
    Up1 = 2,
    /// Down for player 1
    Down1 = 3,
    /// Left for player 1
    Left1 = 4,
    /// Right for player 1
    Right1 = 5,
    /// Whack for player 1,
    /// usually Enter
    Whack1 = 6,
    /// Grab for player 1,
    /// usually right shift
    Grab1 = 7,
    /// Up for player 2
    Up2 = 8,
    /// Down for player 2
    Down2 = 9,
    /// Left for player 2
    Left2 = 10,
    /// Right for player 2
    Right2 = 11,
    /// Whack for player 1,
    /// usually Z
    Whack2 = 12,
    /// Grab for player 1,
    /// usually X
    Grab2 = 13,
    /// Confirm quit, Y
    Confirm = 14,
}

const NUM_KEYS: usize = 15;

/// the primary keyboard key which maps to the given [`Key`]
static mut KEY_MAPPING: [u8; NUM_KEYS] = [
    0x1c, 0x01, 0x48, 0x50, 0x4b, 0x4d, 0x1c, 0x36, 0x11, 0x1f, 0x1e, 0x20, 0x2c, 0x2d, 0x15,
];

// global data

/// the latest key state buffer
static mut KEY_STATE: [bool; NUM_KEYS] = [false; NUM_KEYS];

/// the previous key state buffer
static mut KEY_STATE_PREVIOUS: [bool; NUM_KEYS] = [false; NUM_KEYS];

// functions

/// Update the key state buffers for reading in this cycle.
///
/// Call this first, then use the other functions below.
pub fn update_keys() {
    // roll key state
    unsafe {
        for i in 0..NUM_KEYS {
            let k = KEY_STATE[i];
            KEY_STATE_PREVIOUS[i] = k;
        }
    }

    // update current key state

    for _ in 0..8 {
        let key = key::get_keypress();
        if key == 0 {
            break;
        }

        // take off
        let release = (key & 0x80) != 0;
        let key = key & 0x7F;

        for i in 0..NUM_KEYS {
            unsafe {
                if KEY_MAPPING[i] == key {
                    // update mapping
                    KEY_STATE[i] = !release;
                }
            }
        }
    }
}

/// return whether the key has just been pressed
pub fn is_pressed(key: Key) -> bool {
    unsafe { KEY_STATE[key as usize] && !KEY_STATE_PREVIOUS[key as usize] }
}

/// return whether the key is held down
pub fn is_down(key: Key) -> bool {
    unsafe { KEY_STATE[key as usize] }
}

/// return whether the key has just been released
pub fn is_released(key: Key) -> bool {
    unsafe { !KEY_STATE[key as usize] && KEY_STATE_PREVIOUS[key as usize] }
}

// --- interrupt handling logic

static mut OLD_KEYBOARD_ISR: djgpp::dpmi::_go32_dpmi_seginfo = unsafe { core::mem::zeroed() };
static mut NEW_KEYBOARD_ISR: djgpp::dpmi::_go32_dpmi_seginfo = unsafe { core::mem::zeroed() };
const INTERRUPT_KEYBOARD: core::ffi::c_int = 0x09;

pub fn take_default_keyboard_interrupt() {
    // fetch the address of the old keyboard ISR into old_isr
    unsafe {
        djgpp::dpmi::_go32_dpmi_get_protected_mode_interrupt_vector(
            INTERRUPT_KEYBOARD,
            &raw mut OLD_KEYBOARD_ISR,
        );

        let h: *mut c_void = custom_keyboard_interrupt_callback as *mut _;

        _go32_dpmi_lock_code(h, 128);

        NEW_KEYBOARD_ISR.pm_offset = h.addr() as u32;
        NEW_KEYBOARD_ISR.pm_selector = _go32_my_cs();

        let c = djgpp::dpmi::_go32_dpmi_allocate_iret_wrapper(&raw mut NEW_KEYBOARD_ISR);
        assert_eq!(c, 0);

        let c = djgpp::dpmi::_go32_dpmi_set_protected_mode_interrupt_vector(
            INTERRUPT_KEYBOARD,
            &raw mut NEW_KEYBOARD_ISR,
        );
        assert_eq!(c, 0);
    }
}

pub fn restore_keyboard_interrupt() {
    unsafe {
        println!(
            "Restoring keyboard interrupt handler ({:?})...",
            *&raw const OLD_KEYBOARD_ISR
        );
        let c = djgpp::dpmi::_go32_dpmi_set_protected_mode_interrupt_vector(
            INTERRUPT_KEYBOARD,
            &raw mut OLD_KEYBOARD_ISR,
        );
        assert_eq!(c, 0);
        let c = djgpp::dpmi::_go32_dpmi_free_iret_wrapper(&raw mut NEW_KEYBOARD_ISR);
        assert_eq!(c, 0);
    }
    println!("Done.");
}

#[unsafe(no_mangle)]
#[inline(never)]
unsafe extern "C" fn custom_keyboard_interrupt_callback() {
    // do nothing. We capture the keyboard buffer ourselves
    unsafe {
        outportb(0x20, 0x20);
    }
}
