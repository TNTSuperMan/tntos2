use core::fmt::Write;

use crate::{
    asm::stop,
    screen::{Color, SCREEN},
};

#[panic_handler]
fn on_panic(info: &core::panic::PanicInfo) -> ! {
    SCREEN.try_lock().map(|mut lock| {
        lock.as_mut().map(|screen| {
            screen.draw_rect(0, 0, screen.width(), screen.height(), Color::BLUE);

            let _ = screen
                .str_writer(50, 50, Color::WHITE)
                .write_fmt(format_args!(":(\ntntos2 {info}"));
        })
    });

    stop()
}
