use alloc::vec::Vec;
use alloc::{format, vec};
use dos_x::vga::{self, Palette};
use minipng::BitDepth;

use crate::creature::CreatureParams;

// embed bitmap font images into the binary
static BIGFONT_PNG: &[u8] = include_bytes!("../../resources/nullptr.png");
static SMALLFONT_PNG: &[u8] = include_bytes!("../../resources/charmap-cellphone_white.png");

pub struct BitmapFont {
    /// the pixel data (8-bit colormap for convenience)
    pub pixeldata: Vec<u8>,
    /// number of columns in the pixel data
    pub img_width: u16,
    /// the width and height of the character in pixels
    pub char_size: (u8, u8),
    /// x and y pixels to shift to reach a new character
    pub char_stride: (u8, u8),
    /// number of characters in each row
    pub chars_per_row: u8,
}

impl BitmapFont {
    /// the horizontal spacing added between characters when drawing text
    const H_SPACING: u8 = 1;

    pub fn big() -> Self {
        // 16x16 grid of 14x14 characters
        let h =
            minipng::decode_png_header(BIGFONT_PNG).expect("Failed to load big font PNG header");
        let mut pixeldata = vec![0; h.required_bytes()];
        let _image = minipng::decode_png(BIGFONT_PNG, &mut pixeldata[..])
            .expect("Failed to load big font PNG");

        BitmapFont {
            pixeldata,
            img_width: h.width() as u16,
            char_size: (14, 14),
            char_stride: (16, 16),
            chars_per_row: 8,
        }
    }

    pub fn small() -> Self {
        // 7x9 grid of 5x7 characters
        let h = minipng::decode_png_header(SMALLFONT_PNG)
            .expect("Failed to load small font PNG header");
        let mut pixeldata = vec![0; h.required_bytes()];
        let _image = minipng::decode_png(SMALLFONT_PNG, &mut pixeldata[..])
            .expect("Failed to load small font PNG");

        BitmapFont {
            pixeldata,
            img_width: h.width() as u16,
            char_size: (5, 7),
            char_stride: (7, 9),
            chars_per_row: 18,
        }
    }

    /// map a single text character
    /// to its pixel coordinates in the font sheet
    fn map_char_to_position(&self, mut char: u8) -> (u32, u32) {
        if !(b' '..=b'~').contains(&char) {
            char = b'?';
        }

        let i = (char - b' ') as u32;
        let (i, j) = (i % self.chars_per_row as u32, i / self.chars_per_row as u32);

        (i * self.char_stride.0 as u32, j * self.char_stride.1 as u32)
    }

    pub fn draw_text(&self, x: i32, y: i32, text: impl AsRef<str>, color: u8) {
        let img_width = self.img_width as u32;
        let cw = self.char_size.0 as u32;
        let ch = self.char_size.1 as u32;
        unsafe {
            let text = text.as_ref();
            for (i, c) in text.bytes().enumerate() {
                let (char_x, char_y) = self.map_char_to_position(c);
                let mut char_buffer = [0u8; 14 * 14];
                let target = (
                    x + i as i32 * (self.char_size.0 as i32 + Self::H_SPACING as i32),
                    y,
                );
                // fill buffer from current video buffer
                vga::read_video_buffer_rect(&mut char_buffer, target, (cw, ch));

                // copy character pixel data into temporary buffer
                // if non-zero
                for row in 0..ch {
                    let src_offset = ((char_y + row) * img_width + char_x) as usize;
                    let dst_offset = (row * cw) as usize;
                    for col in 0..cw {
                        let pixel = self.pixeldata[src_offset + col as usize];
                        if pixel == 0 {
                            continue;
                        }
                        char_buffer[dst_offset + col as usize] = color;
                    }
                }
                // blit character
                dos_x::vga::blit_rect(&char_buffer, (cw, ch), (0, 0, cw, ch), target);
            }
        }
    }
}
