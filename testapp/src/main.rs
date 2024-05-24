use sdl2::{pixels::Color, rect::Rect};

const SCALE: u32 = 10;
const WINDOW_WIDTH: u32 = chip8_engine::SCREEN_WIDTH as u32 * SCALE;
const WINDOW_HEIGHT: u32 = chip8_engine::SCREEN_HEIGHT as u32 * SCALE;

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

    let mut machine = chip8_engine::Machine::new();
    let rom: &[u8] = &[
        0x00, 0xe0, 0xa2, 0x2a, 0x60, 0x0c, 0x61, 0x08, 0xd0, 0x1f, 0x70, 0x09, 0xa2, 0x39, 0xd0,
        0x1f, 0xa2, 0x48, 0x70, 0x08, 0xd0, 0x1f, 0x70, 0x04, 0xa2, 0x57, 0xd0, 0x1f, 0x70, 0x08,
        0xa2, 0x66, 0xd0, 0x1f, 0x70, 0x08, 0xa2, 0x75, 0xd0, 0x1f, 0x12, 0x28, 0xff, 0x00, 0xff,
        0x00, 0x3c, 0x00, 0x3c, 0x00, 0x3c, 0x00, 0x3c, 0x00, 0xff, 0x00, 0xff, 0xff, 0x00, 0xff,
        0x00, 0x38, 0x00, 0x3f, 0x00, 0x3f, 0x00, 0x38, 0x00, 0xff, 0x00, 0xff, 0x80, 0x00, 0xe0,
        0x00, 0xe0, 0x00, 0x80, 0x00, 0x80, 0x00, 0xe0, 0x00, 0xe0, 0x00, 0x80, 0xf8, 0x00, 0xfc,
        0x00, 0x3e, 0x00, 0x3f, 0x00, 0x3b, 0x00, 0x39, 0x00, 0xf8, 0x00, 0xf8, 0x03, 0x00, 0x07,
        0x00, 0x0f, 0x00, 0xbf, 0x00, 0xfb, 0x00, 0xf3, 0x00, 0xe3, 0x00, 0x43, 0xe0, 0x00, 0xe0,
        0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0xe0, 0x00, 0xe0,
    ];
    machine.load_rom(rom);

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
                let x = (i % chip8_engine::SCREEN_WIDTH) as u32;
                let y = (i / chip8_engine::SCREEN_WIDTH) as u32;

                let rect = Rect::new((SCALE * x) as i32, (SCALE * y) as i32, SCALE, SCALE);
                canvas.fill_rect(rect).unwrap();
            }
        }
        canvas.present();
    }
}