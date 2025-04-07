mod emulator;

fn main() {
    let mut chip8 = emulator::core::Chip8::new();
    chip8.tick();
    chip8.tick_timers();
}
