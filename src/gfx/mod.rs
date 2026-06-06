use alloc::format;
use alloc::vec::Vec;
use dos_x::vga::{Palette, read_video_buffer_rect, vsync};
use minipng::{BitDepth, ImageData};

use crate::creature::CreatureParams;
use crate::gameplay::CreatureIndex;
use crate::gfx::background::Background;

pub mod background;
pub mod fonts;

pub const COLOR_HIGHLIGHT: u8 = 252;
pub const COLOR_BACKGROUND: u8 = 253;
pub const COLOR_WHITE: u8 = 254;
pub const COLOR_BEIGE: u8 = 9;
pub const COLOR_BLACK: u8 = 1;

/// Defines the start of color indices used for background images
pub const COLOR_BACKGROUND_OFFSET: u8 = 64;
/// The maximum number of colors admitted for a background
pub const BACKGROUND_MAX_COLORS: u8 = 160;

// embed images into the binary
static CREATURE_SHAPES: &[u8] = include_bytes!("../../resources/creature-shapes.png");
static CREATURE_EYES: &[u8] = include_bytes!("../../resources/creature-eyes.png");
static CREATURE_MOUTHS: &[u8] = include_bytes!("../../resources/creature-mouths.png");
static CREATURE_LEGS: &[u8] = include_bytes!("../../resources/creature-legs.png");
static CREATURE_ARMS: &[u8] = include_bytes!("../../resources/creature-arms.png");

static GLOVE_PNG: &[u8] = include_bytes!("../../resources/glove.png");

#[derive(Debug)]
pub struct CreatureAssets {
    pub shapes_image: ImageAsset,
    pub eyes_image: ImageAsset,
    pub mouths_image: ImageAsset,
    pub legs_image: ImageAsset,
    pub arms_image: ImageAsset,
}

/// Representation of a video buffer
pub trait Buffer {
    /// read-only access to the buffer
    fn data(&self) -> &[u8];
    /// read-write access to the buffer
    fn data_mut(&mut self) -> &mut [u8];

    /// get the width of the buffer
    fn width(&self) -> u16;

    /// get the height of the buffer
    fn height(&self) -> u16;

    /// slice a rectangle portion of the buffer
    fn rect_mut(&mut self, pos: (u16, u16), dim: (u16, u16)) -> &mut [u8] {
        let buf_w = self.width() as usize;
        let buf_h = self.height() as usize;
        let offset = pos.1 as usize * buf_w + pos.0 as usize;
        // clamp width
        let w = (dim.0 as usize).min(buf_w - pos.0 as usize);
        let h = (dim.1 as usize).min(buf_h - pos.1 as usize);

        &mut self.data_mut()[offset..offset + w * h]
    }

    /// get a slice of a particular row
    fn row_mut(&mut self, y: u16) -> &mut [u8] {
        let w = self.width() as usize;
        let offset = y as usize * w;
        &mut self.data_mut()[offset..offset + w]
    }

    /// paint a horizontal line in a solid color
    fn hline(&mut self, (pos_x, pos_y): (u16, u16), length: u16, c: u8) {
        if pos_y >= self.height() || pos_x >= self.width() {
            return;
        }
        let row = self.row_mut(pos_y);
        let x = pos_x as usize;
        row[x..x + length as usize].fill(c);
    }

    /// paint a vertical line in a solid color
    fn vline(&mut self, (pos_x, pos_y): (u16, u16), length: u16, c: u8) {
        let width = self.width() as usize;
        let mut offset = pos_y as usize * width + pos_x as usize;
        let buf = self.data_mut();
        for _ in 0..length {
            buf[offset] = c;
            offset += width;
        }
    }
}

/// owned image asset (always 8-bit indexed)
pub struct ImageAsset {
    pub width: u32,
    pub height: u32,
    pub pixel_data: Vec<u8>,
    pub bit_depth: BitDepth,
    pub palette: Vec<u8>,
}

impl Buffer for ImageAsset {
    fn data(&self) -> &[u8] {
        &self.pixel_data
    }

    fn data_mut(&mut self) -> &mut [u8] {
        &mut self.pixel_data
    }

    fn width(&self) -> u16 {
        self.width as u16
    }

    fn height(&self) -> u16 {
        self.height as u16
    }
}

impl ImageAsset {
    /// Load a PNG file for an image asset,
    /// without loading the palette
    pub fn load_asset(png_data: &[u8]) -> Self {
        // (first load header to know how much space to reserve)
        let h = minipng::decode_png_header(png_data).expect("Failed to read png header");
        let bytes_needed = h.required_bytes();
        let mut img_buffer: Vec<u8> = Vec::with_capacity(bytes_needed);
        img_buffer.resize(bytes_needed, 0);
        match minipng::decode_png(png_data, &mut img_buffer[..]) {
            Ok(image) => {
                if image.color_type() != minipng::ColorType::Indexed {
                    panic!("Image must be indexed");
                }
                let width = image.width();
                let height = image.height();
                let bit_depth = image.bit_depth();

                ImageAsset {
                    width,
                    height,
                    bit_depth,
                    pixel_data: img_buffer,
                    palette: Vec::new(),
                }
            }
            Err(e) => {
                panic!("Could not decode PNG file: {}", e);
            }
        }
    }

    /// Load a PNG file for an image asset,
    /// including the palette
    pub fn load_asset_with_palette(png_data: &[u8]) -> Self {
        // (first load header to know how much space to reserve)
        let h = minipng::decode_png_header(png_data).expect("Failed to read png header");
        let bytes_needed = h.required_bytes();
        let mut img_buffer: Vec<u8> = Vec::with_capacity(bytes_needed);
        img_buffer.resize(bytes_needed, 0);
        match minipng::decode_png(png_data, &mut img_buffer[..]) {
            Ok(image) => {
                if image.color_type() != minipng::ColorType::Indexed {
                    panic!("Image must be indexed");
                }
                let palette = Self::palette_from_imagedata(&image);

                let width = image.width();
                let height = image.height();
                let bit_depth = image.bit_depth();

                ImageAsset {
                    width,
                    height,
                    bit_depth,
                    pixel_data: img_buffer,
                    palette,
                }
            }
            Err(e) => {
                panic!("Could not decode PNG file: {}", e);
            }
        }
    }

    fn palette_from_imagedata(image: &ImageData) -> Vec<u8> {
        let mut palette = Vec::new();
        let num_colors = (1 as u16) << (image.bit_depth() as u16);
        for i in 0..num_colors as u16 {
            let [r, g, b, _a] = image.palette(i as u8);
            palette.push(r >> 2);
            palette.push(g >> 2);
            palette.push(b >> 2);
        }
        palette
    }

    pub fn load_glove() -> ImageAsset {
        Self::load_asset(GLOVE_PNG)
    }
}

impl CreatureAssets {
    /// Load all creature assets.
    pub fn load() -> CreatureAssets {
        // load creature shapes
        let shapes_image = ImageAsset::load_asset(CREATURE_SHAPES);

        // load creature eyes
        let eyes_image = ImageAsset::load_asset(CREATURE_EYES);

        // load creature mouths
        let mouths_image = ImageAsset::load_asset(CREATURE_MOUTHS);

        // load creature legs
        let legs_image = ImageAsset::load_asset(CREATURE_LEGS);

        // load creature arms
        let arms_image = ImageAsset::load_asset(CREATURE_ARMS);

        CreatureAssets {
            shapes_image,
            eyes_image,
            mouths_image,
            legs_image,
            arms_image,
        }
    }

    /// Render the creature into a 32x32 pixel data buffer.
    ///
    /// The index is important because it defines
    /// the offset for the creature-specific colors on the palette.
    pub fn render_creature(
        &self,
        params: &CreatureParams,
        which: CreatureIndex,
        buffer: &mut [u8; 32 * 32],
    ) {
        // render creature legs first
        let legs_index = params.legs as u32;
        let legs_x = legs_index * 32;
        for j in 9..32 {
            for i in 0..32 {
                let src_offset = (j * self.legs_image.width + legs_x + i) as usize;
                let leg_pixel = self.legs_image.pixel_data[src_offset];
                if leg_pixel != 0 {
                    let dst_offset = (j * 32 + i) as usize;
                    buffer[dst_offset] = leg_pixel;
                }
            }
        }

        // render creature shape to buffer
        let shape_index = params.shape as u32;
        let shape_x = shape_index * 32;

        for row in 1..31 {
            for col in 1..31 {
                let src_offset = (row * self.shapes_image.width + shape_x + col) as usize;
                let shape_pixel = self.shapes_image.pixel_data[src_offset];
                if shape_pixel != 0 {
                    let dst_offset = (row * 32 + col) as usize;
                    buffer[dst_offset] = shape_pixel;
                }
            }
        }

        // render arms
        let arms_index = params.arms as u32;
        let arms_x = arms_index * 32;
        for j in 2..32 {
            for i in 0..32 {
                let src_offset = (j * self.arms_image.width + arms_x + i) as usize;
                let arm_pixel = self.arms_image.pixel_data[src_offset];
                if arm_pixel != 0 {
                    let dst_offset = (j * 32 + i) as usize;
                    buffer[dst_offset] = arm_pixel;
                }
            }
        }

        // render mouth
        let mouth_index = params.mouth as u32;
        let mouth_x = mouth_index * 32;

        for j in 5..28 {
            for i in 2..30 {
                let src_offset = (j * self.mouths_image.width + mouth_x + i) as usize;
                let mouth_pixel = self.mouths_image.pixel_data[src_offset];
                if mouth_pixel != 0 {
                    let dst_offset = (j * 32 + i) as usize;
                    buffer[dst_offset] = mouth_pixel;
                }
            }
        }

        // draw eyes on top
        let eyes_index = params.eyes as u32;
        let eyes_x = eyes_index * 32;

        // we use a tiny trick here, since we do not expect eye pixels around the boundaries
        for j in 2..25 {
            for i in 3..29 {
                let src_offset = (j * self.eyes_image.width + eyes_x + i) as usize;
                let eye_pixel = self.eyes_image.pixel_data[src_offset];
                if eye_pixel != 0 {
                    let dst_offset = (j * 32 + i) as usize;
                    buffer[dst_offset] = eye_pixel;
                }
            }
        }

        // adjust creature-specific colors
        let creature_index = which as u8;
        // edge case: default creature index
        if creature_index == 0 {
            return;
        }
        for v in buffer {
            if (16..20).contains(v) {
                // 4 creature-specific colors
                *v += creature_index * 4;
            }
        }
    }

    /// Draw the creature to the screen at the given pixel coordinates.
    pub fn draw_creature(&self, params: &CreatureParams, which: CreatureIndex, x: i32, y: i32) {
        let mut buffer = [0; 32 * 32];

        unsafe {
            read_video_buffer_rect(&mut buffer, (x, y), (32, 32));
        }

        self.render_creature(params, which, &mut buffer);

        unsafe {
            dos_x::vga::blit_rect(&buffer, (32, 32), (0, 0, 32, 32), (x, y));
        }
    }
}

impl core::fmt::Debug for ImageAsset {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ImageData")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("bit_depth", &self.bit_depth)
            .field("palette_length", &format!("[u8; {}", self.palette.len()))
            .finish()
    }
}

/// Fade out the given palette into all blackness.
pub fn fade_out(palette: &mut Palette) {
    for _ in 0..32 {
        fade_out_step(palette);
    }
}

/// Perform a single step to fade out the given palette
/// into all blackness.
///
/// This is useful in order to keep the game loop running
/// while fading out.
pub fn fade_out_step(palette: &mut Palette) {
    unsafe {
        for p in palette.0.iter_mut().take(768) {
            *p = p.saturating_sub(2);
        }
        vsync();
        palette.set();
    }
}

/// Fade in the given pallette
/// until it reaches the values in the other palette.
///
/// Ensure to reset the main palette first.
pub fn fade_in(palette: &mut Palette, to_match: &Palette) {
    for _ in 0..32 {
        fade_in_step(palette, to_match);
    }
}

/// Perform a single step to fade-in the given pallette
/// until it reaches the values in the other palette.
///
/// This is useful in order to keep the game loop running
/// while fading in.
pub fn fade_in_step(palette: &mut Palette, to_match: &Palette) {
    unsafe {
        for (p, v) in palette
            .0
            .iter_mut()
            .take(768)
            .zip(to_match.0.iter().copied())
        {
            if *p >= v {
                *p = v;
            } else {
                *p = (*p + 2).min(v);
            }
        }
        vsync();
        palette.set();
    }
}

pub fn draw_arrow_right(x: u32, y: u32, color: u8) {
    unsafe {
        dos_x::vga::put_pixel(x + 1, y, color);
        dos_x::vga::put_pixel(x + 2, y + 1, color);
        dos_x::vga::put_pixel(x + 3, y + 2, color);
        dos_x::vga::draw_hline(x as i32, y as i32 + 3, 5, color);
        dos_x::vga::put_pixel(x + 3, y + 4, color);
        dos_x::vga::put_pixel(x + 2, y + 5, color);
        dos_x::vga::put_pixel(x + 1, y + 6, color);
    }
}

pub fn draw_arrow_left(x: u32, y: u32, color: u8) {
    unsafe {
        dos_x::vga::put_pixel(x + 3, y, color);
        dos_x::vga::put_pixel(x + 2, y + 1, color);
        dos_x::vga::put_pixel(x + 1, y + 2, color);
        dos_x::vga::draw_hline(x as i32, y as i32 + 3, 5, color);
        dos_x::vga::put_pixel(x + 1, y + 4, color);
        dos_x::vga::put_pixel(x + 2, y + 5, color);
        dos_x::vga::put_pixel(x + 3, y + 6, color);
    }
}

/// A sprite for a creature in a hole
#[derive(Debug, Copy, Clone)]
pub struct HoleSprite {
    /// center position of the hole on the X coordinate
    x: i32,
    /// wide-most position of the hole on the Y coordinate
    /// (for convenience)
    y: i32,
    /// pixel data buffer (32 x 42)
    pub buffer: [u8; 32 * 42],
}

impl HoleSprite {
    pub fn new_at(x: i32, y: i32) -> Self {
        let mut o = HoleSprite {
            x,
            y,
            buffer: [0u8; 32 * 42],
        };

        o.update_buffer();
        o
    }

    pub fn update_buffer(&mut self) {
        Self::read_background(&mut self.buffer, self.x, self.y);
        Self::render_hole(&mut self.buffer);
    }

    // use x and y to fetch the corresponding pixel data from the screen
    fn read_background(buffer: &mut [u8], x: i32, y: i32) {
        unsafe {
            dos_x::vga::read_video_buffer_rect(buffer, (x - 16, y - 37), (32, 42));
        }
    }

    fn render_hole(buffer: &mut [u8]) {
        // we'll draw the hole around here
        let y_hole = 34;

        for i in 0..7 {
            // draw two horizontal lines with the same size per step
            let w = match i {
                0 => 4,
                1 => 7,
                2 => 10,
                3 => 12,
                4 => 14,
                5 => 15,
                6.. => 16,
                _ => unreachable!("draw_hole i out of bounds"),
            };
            let o_start = ((y_hole + 7 - i as usize) * 32) + (16 - w);
            buffer[o_start..o_start + w * 2].fill(COLOR_BLACK);
            let o_start = ((y_hole - 6 + i as usize) * 32) + (16 - w);
            buffer[o_start..o_start + w * 2].fill(COLOR_BLACK);
        }
    }

    /// Draws the hole without a creature.
    ///
    /// See [CreatureSprite::draw] to draw a creature and its hole.
    pub fn draw(&self) {
        unsafe {
            dos_x::vga::blit_rect(
                &self.buffer,
                (32, 42),
                (0, 0, 32, 42),
                (self.x - 16, self.y - 37),
            );
        }
    }

    /// Draws the hole to a full 320x200 video buffer
    pub fn render_to(&self, vga_buffer: &mut [u8]) {
        // row by row
        for row in 0..42 {
            let base_offset_in = row as usize * 32;
            let base_offset_out = row as usize * 320 + self.x as usize;

            vga_buffer[base_offset_out..base_offset_out + 32]
                .copy_from_slice(&self.buffer[base_offset_in..base_offset_in + 32]);
        }
    }

    /// half of the width of the hole at a given coordinate
    pub fn half_width_at(i: i16) -> u8 {
        match i {
            0 => 4,
            1 => 7,
            2 => 10,
            3 => 12,
            4 => 14,
            5 => 15,
            6.. => 16,
            _ => unreachable!("draw_hole i out of bounds"),
        }
    }
}

/// A sprite for a creature which can be partially drawn
/// (from being in a hole).
#[derive(Debug, Copy, Clone)]
pub struct CreatureSprite {
    /// a buffer with the creature rendered in it,
    /// only changes when the creature is replaced
    creature_buffer: [u8; 32 * 32],
    /// a buffer with the creature partially concealed,
    /// changes when `outside` changes
    partial_buffer: [u8; 32 * 32],
    /// a buffer for what is _really_ drawn to the screen
    out_buffer: [u8; 32 * 42],
    /// the X position of the hole
    /// per the corresponding [`HoleSprite`]
    x: i32,
    /// the Y position of the hole
    /// per the corresponding [`HoleSprite`]
    y: i32,
    /// how many pixels the creature is outside of the hole.
    /// 0 for invisible inside the hole,
    /// 32 for fully visible
    outside: i16,
}

impl CreatureSprite {
    pub fn new_unset(hole: &HoleSprite) -> Self {
        CreatureSprite {
            creature_buffer: [0; 32 * 32],
            partial_buffer: [0; 32 * 32],
            out_buffer: [0; 32 * 42],
            x: hole.x,
            y: hole.y,
            outside: 0,
        }
    }

    pub fn new(
        creature: &CreatureParams,
        which: CreatureIndex,
        creature_assets: &CreatureAssets,
        hole: &HoleSprite,
    ) -> Self {
        let mut sprite = CreatureSprite {
            creature_buffer: [0; 32 * 32],
            partial_buffer: [0; 32 * 32],
            out_buffer: [0; 32 * 42],
            x: hole.x,
            y: hole.y,
            outside: 0,
        };
        sprite.set_creature(creature, which, creature_assets);
        sprite
    }

    pub fn set_creature(
        &mut self,
        creature: &CreatureParams,
        which: CreatureIndex,
        creature_assets: &CreatureAssets,
    ) {
        self.creature_buffer[..].fill(0);
        creature_assets.render_creature(creature, which, &mut self.creature_buffer);
    }

    #[inline]
    pub fn update_up(&mut self, delta: i16) {
        self.update(self.outside + delta)
    }

    #[inline]
    pub fn update_down(&mut self, delta: i16) {
        self.update(self.outside - delta)
    }

    pub fn update(&mut self, new_outside: i16) {
        // clamp it so we have no surprises
        let new_outside = new_outside.clamp(0, 32);

        if self.outside == new_outside {
            return;
        }

        if self.outside < new_outside {
            // getting out of the hole

            for i in 0..7 {
                let row = new_outside - i - 1;
                if row < 0 {
                    // no more rows to consider
                    break;
                }
                debug_assert!(row < 32, "row should not have reached 32, but was {row}");

                let row_index = 32 * row as usize;

                // fill row according to i
                let hw = HoleSprite::half_width_at(i) as usize;
                let range = row_index + 16 - hw..row_index + 16 + hw;
                self.partial_buffer[range.clone()].copy_from_slice(&self.creature_buffer[range]);
            }
            // nothing else is needed,
            // it should work as long as the difference isn't too significant
        } else {
            // getting back into the hole
            for i in 0..7 {
                let row = new_outside - i;
                if row < 0 {
                    break;
                }
                debug_assert!(row < 32, "row should not have reached 32, but was {row}");
                let row_index = 32 * row as usize;
                // TODO improve this so that it looks more realistic
                self.partial_buffer[row_index..row_index + 32].fill(0);
            }
        }

        self.outside = new_outside;
    }

    /// Render the effective creature onto its final buffer,
    /// without touching transparent pixels.
    ///
    /// The buffer should have at least 42 pixels in height;
    fn update_buffer(&mut self) {
        // position depends on how "outside" the creature is
        for i in 0..self.outside {
            // rendering top to bottom,
            // so that we can easily show it peeping from the hole;
            // the offset depends on how much of it is outside
            let out_row = 38 - self.outside + i;
            let out_row_index = out_row as usize * 32;
            let row_index = i as usize * 32;
            for (&p, o) in self.partial_buffer[row_index..row_index + 32]
                .iter()
                .zip(&mut self.out_buffer[out_row_index..out_row_index + 32])
            {
                if p != 0 {
                    *o = p;
                }
            }
        }
    }

    /// Draw the creature in its hole
    /// at the right position
    pub fn draw(&mut self, hole: &HoleSprite) {
        // copy hole buffer onto output buffer
        self.out_buffer.copy_from_slice(&hole.buffer);
        self.update_buffer();

        // then blit to screen
        unsafe {
            dos_x::vga::blit_rect(
                &self.out_buffer,
                (32, 42),
                (0, 0, 32, 42),
                (self.x - 16, self.y - 37),
            );
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
#[repr(u8)]
pub enum GloveFrame {
    Idle = 0,
    Whack = 1,
    Grab = 2,
}

/// Directly draw a glove (16x16) at the requested position on the screen
pub fn draw_glove(glove: &ImageAsset, x: i32, y: i32, frame: GloveFrame) {
    let mut buf = [0_u8; 16 * 16];

    // use x and y to fetch data from screen
    unsafe {
        dos_x::vga::read_video_buffer_rect(&mut buf, (x, y), (16, 16));
    }

    // pick the right frame from the image asset
    let offset = frame as usize * 16 * 16;
    let pixel_data = &glove.pixel_data[offset..offset + 16 * 16];

    for (o, &v) in buf.iter_mut().zip(pixel_data) {
        if v != 0 {
            *o = v;
        }
    }

    // blit back to screen
    unsafe {
        dos_x::vga::blit_rect(&buf, (16, 16), (0, 0, 16, 16), (x, y));
    }
}

/// Initialize the color palette with the hardcoded stuff.
///
/// Creature and background palettes must be set afterwards.
/// Remember also to call `.set()`
pub fn init_palette(palette: &mut Palette) {
    // initialize with zeros
    palette.0.fill(0);

    // set up palette:
    // 0: reserved for transparency
    // 1: black
    // 2: white
    // 3: light grey
    // 4: grey
    // 5: dark grey
    // 6: red
    // 7: darker red
    // 8: brown
    // 9: beige
    // 10..=15: reserved
    // 16: creature #1 light
    // 17: creature #1 regular
    // 18: creature #1 dark
    // 19: creature #1 darker
    // 20..=23: creature #2 (light, regular, dark, darker)
    // 24..=27: creature #3 (light, regular, dark, darker)
    // 28..=31: creature #4 (light, regular, dark, darker)
    // 32..=34: creature #5 (light, regular, dark, darker)
    // 35..=63: reserved
    // 64..=223: background image palette (160 colors)
    // 224..=251: reserved
    // 252: highlight color (orange-ish)
    // 253: background color
    // 254: white
    // 255: black

    // (setting color 0 to magenta for testing purposes
    // as it is never really used anyway)
    palette.0[0] = 0x30;
    palette.0[1] = 0x3f;
    palette.0[2] = 0x00;

    // black is already black

    // white
    palette.0[6] = 0x3c;
    palette.0[7] = 0x3c;
    palette.0[8] = 0x3c;

    // light grey
    palette.0[9] = 0x32;
    palette.0[10] = 0x32;
    palette.0[11] = 0x32;

    // dark grey
    palette.0[12] = 0x0f;
    palette.0[13] = 0x0f;
    palette.0[14] = 0x0f;

    // grey
    palette.0[15] = 0x1f;
    palette.0[16] = 0x1f;
    palette.0[17] = 0x1f;

    // red
    palette.0[18] = 0x3c;
    palette.0[19] = 0x03;
    palette.0[20] = 0x03;

    // darker red
    palette.0[21] = 0x1f;
    palette.0[22] = 0x01;
    palette.0[23] = 0x01;

    // brown
    palette.0[24] = 0x1f;
    palette.0[25] = 0x0e;
    palette.0[26] = 0x00;

    // a light beige for the stats board
    palette.0[27] = 0x39;
    palette.0[28] = 0x38;
    palette.0[39] = 0x24;

    // range 40..47 currently unused

    // range 48.. reserved for creatures and background (20 + 160 colors)

    // highlight color (orange)
    palette.0[252 * 3] = 63;
    palette.0[252 * 3 + 1] = 36;
    palette.0[252 * 3 + 2] = 0;

    // background color (soft magenta)
    palette.0[253 * 3] = 48;
    palette.0[253 * 3 + 1] = 38;
    palette.0[253 * 3 + 2] = 63;

    // ensure that the second last color (#254) is always white.
    palette.0[762] = 63;
    palette.0[763] = 63;
    palette.0[764] = 63;
    // the last color (#255) is always black.
}

/// Update the creature-specific part of the palette.
///
/// The creature index defines which one to update
/// (since there can be more than one different creature on screen).
pub fn set_creature_palette(
    palette: &mut Palette,
    creature: &CreatureParams,
    which: CreatureIndex,
) {
    // 4 colors, 3 samples each
    const CREATURE_COLOR_SAMPLES: usize = 4 * 3;
    let offset = 16 * 3 + CREATURE_COLOR_SAMPLES * which as usize;
    let body_colors: [u8; CREATURE_COLOR_SAMPLES] = creature.body_colors();
    palette.0[offset..offset + CREATURE_COLOR_SAMPLES].copy_from_slice(&body_colors);
}

pub fn set_background_palette(palette: &mut Palette, background: &Background) {
    let offset = COLOR_BACKGROUND_OFFSET as usize * 3;
    let num_samples = BACKGROUND_MAX_COLORS as usize * 3;
    palette.0[offset..offset + num_samples].copy_from_slice(&background.0.palette[0..num_samples]);
}

pub struct StatsBoard {
    buffer: [u8; Self::WIDTH as usize * Self::HEIGHT as usize],
}

impl StatsBoard {
    pub const WIDTH: u16 = 200;
    pub const HEIGHT: u16 = 112;

    pub const MARGIN_X: u16 = (320 - Self::WIDTH) / 2;
    pub const MARGIN_Y: u16 = (200 - Self::HEIGHT) / 2;

    pub fn new() -> Self {
        StatsBoard {
            buffer: [COLOR_BEIGE; Self::WIDTH as usize * Self::HEIGHT as usize],
        }
    }

    pub fn init(&mut self) {
        // draw a rectangle with an outline
        self.buffer.fill(COLOR_BEIGE);
        let c1 = COLOR_BLACK;
        let c2 = COLOR_WHITE;

        self.hline((0, 0), Self::WIDTH, c1);
        self.vline((0, 1), Self::HEIGHT - 2, c1);
        self.vline((Self::WIDTH - 1, 1), Self::HEIGHT - 2, c1);
        self.hline((0, Self::HEIGHT - 1), Self::WIDTH, c1);
        self.hline((1, 0), Self::WIDTH - 2, c2);
        self.vline((1, 1), Self::HEIGHT - 2, c2);
    }

    /// Draw at the center of the screen
    pub fn draw(&self) {
        unsafe {
            dos_x::vga::blit_rect(
                &self.buffer,
                (Self::WIDTH as u32, Self::HEIGHT as u32),
                (0, 0, Self::WIDTH as u32, Self::HEIGHT as u32),
                (Self::MARGIN_X as i32, Self::MARGIN_Y as i32),
            );
        }
    }
}

impl Buffer for StatsBoard {
    fn data(&self) -> &[u8] {
        &self.buffer
    }

    fn data_mut(&mut self) -> &mut [u8] {
        &mut self.buffer
    }

    fn width(&self) -> u16 {
        Self::WIDTH
    }

    fn height(&self) -> u16 {
        Self::HEIGHT
    }
}
