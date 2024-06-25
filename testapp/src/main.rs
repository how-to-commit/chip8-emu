use sdl2::{pixels::Color, rect::Rect};

use chip8_engine::emulator;

const SCALE: u32 = 10;
const WINDOW_WIDTH: u32 = emulator::SCREEN_WIDTH as u32 * SCALE;
const WINDOW_HEIGHT: u32 = emulator::SCREEN_HEIGHT as u32 * SCALE;

fn main() {
    // init sdl
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("Chip-8", WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .opengl()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();
    canvas.clear();
    canvas.present();

    let mut event_pump = sdl_context.event_pump().unwrap();

    // get rom
    let args: Vec<String> = std::env::args().collect();

    let rom = std::fs::read(&args[1]).unwrap();

    let mut machine = emulator::Chip8::new();
    machine.load_rom(&rom);

    'running: loop {
        for e in event_pump.poll_iter() {
            match e {
                sdl2::event::Event::Quit { .. } => {
                    break 'running;
                }
                _ => (),
            }
        }

        machine.run_cycle();

        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();

        canvas.set_draw_color(Color::RGB(255, 255, 255));

        for (i, px) in machine.get_screen().iter().enumerate() {
            if *px {
                let x = (i % emulator::SCREEN_WIDTH) as u32;
                let y = (i / emulator::SCREEN_WIDTH) as u32;

                let rect = Rect::new((SCALE * x) as i32, (SCALE * y) as i32, SCALE, SCALE);
                canvas.fill_rect(rect).unwrap();
            }
        }
        canvas.present();
    }
}
