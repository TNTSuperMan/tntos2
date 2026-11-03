use core::fmt;

use crate::screen;

const CONSOLE_PADDING: u32 = 8;
const CONSOLE_FORE: screen::Color = screen::Color::BLACK;
const CONSOLE_BACK: screen::Color = screen::Color::WHITE;

static CURSOR_POS: spin::Mutex<(u32, u32)> = spin::Mutex::new((CONSOLE_PADDING, CONSOLE_PADDING));

pub struct ConsoleLock<'lock> {
    cursor: spin::MutexGuard<'lock, (u32, u32)>,
    screen: spin::MutexGuard<'lock, screen::Screen>,
}
impl<'lock> ConsoleLock<'lock> {
    pub fn lock() -> ConsoleLock<'static> {
        ConsoleLock {
            cursor: CURSOR_POS.lock(),
            screen: screen::SCREEN.lock(),
        }
    }
    pub fn next_line(&mut self) {
        self.cursor.0 = CONSOLE_PADDING;
        self.cursor.1 += 16;
        if self.cursor.1 > self.screen.height() - CONSOLE_PADDING - 16 {
            self.cursor.1 = CONSOLE_PADDING;
        }
        let width = self.screen.width();
        self.screen.draw_rect(
            CONSOLE_PADDING,
            self.cursor.1,
            width - CONSOLE_PADDING * 2,
            32,
            CONSOLE_BACK,
        );
    }
}
impl<'lock> fmt::Write for ConsoleLock<'lock> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            self.write_char(c)?;
        }
        Ok(())
    }
    fn write_char(&mut self, c: char) -> fmt::Result {
        if !c.is_whitespace() {
            self.screen.draw_ascii(
                self.cursor.0,
                self.cursor.1,
                CONSOLE_FORE,
                if c.is_ascii() { c } else { '?' },
            );
        }
        if c == '\n' {
            self.next_line();
        } else {
            self.cursor.0 += 8;
            if self.cursor.0 > self.screen.width() - CONSOLE_PADDING - 8 {
                self.next_line();
            }
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($args:tt)*) => {{
        $crate::console::ConsoleLock::lock().write_fmt(format_args!($($args)*)).unwrap();
    }};
}
#[macro_export]
macro_rules! println {
    () => {{
        $crate::console::ConsoleLock::lock().next_line();
    }};
    ($($args:tt)*) => {{
        let mut lock = $crate::console::ConsoleLock::lock();
        lock.write_fmt(format_args!($($args)*)).unwrap();
        lock.next_line();
    }};
}
