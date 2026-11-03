#[derive(Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}
#[allow(unused)]
impl Color {
    pub const WHITE: Color = Color::from_rgb(255, 255, 255);
    pub const BLACK: Color = Color::from_rgb(0, 0, 0);
    pub const RED: Color = Color::from_rgb(255, 0, 0);
    pub const GREEN: Color = Color::from_rgb(0, 255, 0);
    pub const BLUE: Color = Color::from_rgb(0, 0, 255);
    pub const fn from_rgb(r: u8, g: u8, b: u8) -> Color {
        Color { r, g, b }
    }
    pub fn to_arr(&self, format: &shared::PixelFormat) -> [u8; 4] {
        match format {
            shared::PixelFormat::Rgb => [self.r, self.g, self.b, 0],
            shared::PixelFormat::Bgr => [self.b, self.g, self.r, 0],
        }
    }
}

pub static SCREEN: spin::Mutex<Screen> = spin::Mutex::new(Screen::uninit());

static FONT: &'static [u8; 16 * 128] = include_bytes!("../../font.bin");

pub unsafe fn assume_init_screen(options: shared::FrameBufferOptions) {
    *SCREEN.lock() = unsafe { Screen::from_options(options) };
}

pub struct Screen {
    buffer: &'static mut [u8],
    width: u32,
    height: u32,
    pixels_per_scanline: u32,
    pixel_format: shared::PixelFormat,
}
impl Screen {
    pub const fn uninit() -> Self {
        Self {
            buffer: unsafe {
                // 長さは0である
                // 空のVecのポインタがダングリングであるように、
                // 長さ0のスライスは論理的に全てが領域外でありアクセスは発生しないため安全である
                core::slice::from_raw_parts_mut(core::ptr::dangling_mut(), 0)
            },
            width: 0,
            height: 0,
            pixels_per_scanline: 0,
            pixel_format: shared::PixelFormat::Rgb,
        }
    }
    pub unsafe fn from_options(options: shared::FrameBufferOptions) -> Self {
        Self {
            buffer: unsafe {
                core::slice::from_raw_parts_mut(options.frame_buffer, options.frame_buffer_size)
            },
            width: options.horizontal_resolution,
            height: options.vertical_resolution,
            pixels_per_scanline: options.pixels_per_scanline,
            pixel_format: options.pixel_format,
        }
    }
    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn height(&self) -> u32 {
        self.height
    }
    pub fn draw_pixel(&mut self, x: u32, y: u32, color: Color) {
        if x >= self.width {
            return;
        }
        if y >= self.height {
            return;
        }
        let index = ((self.pixels_per_scanline * y + x) * 4) as usize;
        let color = color.to_arr(&self.pixel_format);
        self.buffer[index] = color[0];
        self.buffer[index + 1] = color[1];
        self.buffer[index + 2] = color[2];
        self.buffer[index + 3] = color[3];
    }
    pub fn draw_rect(&mut self, x: u32, y: u32, width: u32, height: u32, color: Color) {
        for xi in x..(x + width) {
            for yi in y..(y + height) {
                self.draw_pixel(xi, yi, color);
            }
        }
    }
    pub fn draw_ascii(&mut self, x: u32, y: u32, color: Color, c: char) {
        if c.is_ascii() {
            let idx = (c as usize) * 16;
            for (yi, line) in FONT[idx..(idx + 16)].iter().enumerate() {
                for xi in 0..8 {
                    if *line & (0b10000000 >> xi) != 0 {
                        self.draw_pixel(x + xi, y + (yi as u32), color);
                    }
                }
            }
        }
    }
    pub fn str_writer(&mut self, x: u32, y: u32, color: Color) -> ScrWriter<'_> {
        ScrWriter {
            screen: self,
            x,
            xi: x,
            yi: y,
            color,
        }
    }
}

pub struct ScrWriter<'a> {
    screen: &'a mut Screen,
    x: u32,
    xi: u32,
    yi: u32,
    color: Color,
}
impl<'a> core::fmt::Write for ScrWriter<'a> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.chars() {
            self.screen.draw_ascii(self.xi, self.yi, self.color, c);
            if c == '\n' {
                self.xi = self.x;
                self.yi += 16;
            } else {
                self.xi += 8;
            }
        }
        Ok(())
    }
}
