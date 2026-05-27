use crate::gfx::{COLOR_BACKGROUND_OFFSET, ImageAsset};

static BACK_0: &[u8] = include_bytes!("../../resources/back0.png");
static BACK_1: &[u8] = include_bytes!("../../resources/back1.png");
static BACK_2: &[u8] = include_bytes!("../../resources/back2.png");
static BACK_3: &[u8] = include_bytes!("../../resources/back3.png");
static BACK_4: &[u8] = include_bytes!("../../resources/back4.png");

pub struct Background(pub ImageAsset);

impl Background {
    #[inline]
    pub fn back0() -> Self {
        Self::load(BACK_0)
    }

    #[inline]
    pub fn back1() -> Self {
        Self::load(BACK_1)
    }

    #[inline]
    pub fn back2() -> Self {
        Self::load(BACK_2)
    }

    #[inline]
    pub fn back3() -> Self {
        Self::load(BACK_3)
    }

    #[inline]
    pub fn back4() -> Self {
        Self::load(BACK_4)
    }

    fn load(background_png: &[u8]) -> Self {
        let mut img = ImageAsset::load_asset_with_palette(background_png);
        if img.width != 320 || img.height != 200 {
            panic!("Unacceptable background: must be 320x200");
        }
        if img.palette.is_empty() {
            panic!("Error! Background palette is empty!");
        }

        // shift pixel values
        for v in &mut img.pixel_data {
            *v += COLOR_BACKGROUND_OFFSET;
        }

        Background(img)
    }

    /// Draw all of the background onto the VGA display
    pub fn draw_all(&self) {
        unsafe {
            dos_x::vga::draw_buffer(&self.0.pixel_data);
        }
    }

    /// Redraw a portion of the background onto the VGA display
    pub fn draw_rect(&self, x: i32, y: i32, w: u32, h: u32) {
        let origin = (x.max(0) as u32, y.max(0) as u32, w, h);
        let target = (x, y);

        unsafe {
            dos_x::vga::blit_rect(
                &self.0.pixel_data,
                (self.0.width, self.0.height),
                origin,
                target,
            );
        }
    }
}
