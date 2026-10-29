use core::cell::OnceCell;

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
    pub fn to_u32(&self, format: &shared::PixelFormat) -> u32 {
        match format {
            shared::PixelFormat::Rgb => u32::from_le_bytes([self.r, self.g, self.b, 0]),
            shared::PixelFormat::Bgr => u32::from_le_bytes([self.b, self.g, self.r, 0]),
        }
    }
}

pub static SCREEN: Screen = Screen::new();

static FONT: &'static [u8; 16 * 128] = include_bytes!("../../font.bin");

#[derive(Clone)]
pub struct Screen {
    options: OnceCell<shared::FrameBufferOptions>,
}
unsafe impl Sync for Screen {
    // 基本的にunsafeの根源はFrameBufferOptions.frame_buffer (*mut u8)である
}
impl AsRef<shared::FrameBufferOptions> for Screen {
    fn as_ref(&self) -> &shared::FrameBufferOptions {
        // 最初の最初にちゃんと初期化しておけば基本的に安全
        unsafe { self.options.get().unwrap_unchecked() }
    }
}
impl Screen {
    pub const fn new() -> Self {
        Self {
            options: OnceCell::new(),
        }
    }
    pub fn init(&self, options: shared::FrameBufferOptions) {
        self.options
            .set(options)
            .map_err(|_| ())
            .expect("attempt double init screen");
    }
    pub fn width(&self) -> u32 {
        self.as_ref().horizontal_resolution
    }
    pub fn height(&self) -> u32 {
        self.as_ref().vertical_resolution
    }
    pub fn pixels_per_scanline(&self) -> u32 {
        self.as_ref().pixels_per_scanline
    }
    pub fn format(&self) -> shared::PixelFormat {
        self.as_ref().pixel_format
    }
    pub fn draw_pixel(&self, x: u32, y: u32, color: Color) {
        if x >= self.width() {
            return;
        }
        if y >= self.width() {
            return;
        }
        let index = ((self.pixels_per_scanline() * y + x) * 4) as usize;
        let color = color.to_u32(&self.format());
        unsafe {
            core::ptr::write_unaligned(self.as_ref().frame_buffer.add(index) as *mut u32, color)
        };
    }
    pub fn draw_rect(&self, x: u32, y: u32, width: u32, height: u32, color: Color) {
        for xi in x..(x + width) {
            for yi in y..(y + height) {
                self.draw_pixel(xi, yi, color);
            }
        }
    }
    pub fn draw_ascii(&self, x: u32, y: u32, color: Color, c: char) {
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
    pub fn str_writer(&self, x: u32, y: u32, color: Color) -> ScrWriter<'_> {
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
    screen: &'a Screen,
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
