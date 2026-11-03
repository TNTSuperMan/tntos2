use core::fmt::Write;

use crate::{
    asm::stop,
    screen::{Color, SCREEN},
};

#[panic_handler]
fn on_panic(info: &core::panic::PanicInfo) -> ! {
    SCREEN.try_lock().map(|mut screen| {
        let width = screen.width();
        let height = screen.height();
        screen.draw_rect(0, 0, width, height, Color::BLUE);

        let _ = screen
            .str_writer(50, 50, Color::WHITE)
            .write_fmt(format_args!(":(\ntntos2 {info}"));
    });

    stop()
}
