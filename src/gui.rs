use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use std::time::Duration;
use std::sync::{mpsc,Arc, Mutex};

use crate::gbc::Emu;

pub const WIDTH: usize = 160;
pub const HEIGHT: usize = 144;
pub const DEPTH: usize = 3;
pub const SIZE: usize = WIDTH * HEIGHT * DEPTH;

#[derive(Debug, Clone, Copy)]
pub enum GBKey {
    Up, Down, Left,Right, A, B, Start, Select
}

pub enum Message {
    WindowClosed,
    KeyUp(GBKey),
    KeyDown(GBKey)
}

pub struct Gui{
    context: sdl2::Sdl,
    canvas: sdl2::render::Canvas<sdl2::video::Window>,
    emu: Emu,
}

impl Gui{
    pub fn new(emu: Emu) -> Self {
        let sdl_context = sdl2::init().unwrap();
        let video_subsystem = sdl_context.video().unwrap();
    
        let window = video_subsystem.window("yaGBemu", WIDTH as u32 * 4, HEIGHT as u32 * 4)
            .position_centered()
            .build()
            .unwrap();
    
        let mut canvas = window.into_canvas().build().unwrap();
    
        canvas.clear();
        canvas.present();
        
        Gui {
            context: sdl_context,
            canvas,
            emu
        }
    }
    pub fn run(&mut self){
        let mut event_pump = self.context.event_pump().unwrap();

        let texture_creator = self.canvas.texture_creator();
        let mut texture = texture_creator.create_texture_static(
            PixelFormatEnum::RGB24,
            WIDTH as u32,
            HEIGHT as u32
        ).expect("Could not allocate texture");

        let mut render_target = Box::new([0u8;SIZE]);

        'running: loop {
            let mut events = vec![];
           
            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit {..} |
                    Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                        break 'running
                    },
                    Event::KeyDown {keycode: Some(Keycode::Return), ..} => {
                        events.push(Message::KeyDown(GBKey::Start));
                    },
                    Event::KeyUp {keycode: Some(Keycode::Return), ..} => {
                        events.push(Message::KeyUp(GBKey::Start));
                    },
                    Event::KeyDown {keycode: Some(Keycode::Backspace), ..} => {
                        events.push(Message::KeyDown(GBKey::Select));
                    },
                    Event::KeyUp {keycode: Some(Keycode::Backspace), ..} => {
                        events.push(Message::KeyUp(GBKey::Select));
                    },
                    Event::KeyDown {keycode: Some(Keycode::Q), ..} => {
                        events.push(Message::KeyDown(GBKey::B));
                    },
                    Event::KeyUp {keycode: Some(Keycode::Q), ..} => {
                        events.push(Message::KeyUp(GBKey::B));
                    },
                    Event::KeyDown {keycode: Some(Keycode::S), ..} => {
                        events.push(Message::KeyDown(GBKey::A));
                    },
                    Event::KeyUp {keycode: Some(Keycode::S), ..} => {
                        events.push(Message::KeyUp(GBKey::A));
                    },
                    Event::KeyDown {keycode: Some(Keycode::Left), ..} => {
                        events.push(Message::KeyDown(GBKey::Left));
                    },
                    Event::KeyUp {keycode: Some(Keycode::Left), ..} => {
                        events.push(Message::KeyUp(GBKey::Left));
                    }
                    Event::KeyDown {keycode: Some(Keycode::Right), ..} => {
                        events.push(Message::KeyDown(GBKey::Right));
                    },
                    Event::KeyUp {keycode: Some(Keycode::Right), ..} => {
                        events.push(Message::KeyUp(GBKey::Right));
                    },
                    Event::KeyDown {keycode: Some(Keycode::Up), ..} => {
                        events.push(Message::KeyDown(GBKey::Up));
                    },
                    Event::KeyUp {keycode: Some(Keycode::Up), ..} => {
                        events.push(Message::KeyUp(GBKey::Up));
                    },
                    Event::KeyDown {keycode: Some(Keycode::Down), ..} => {
                        events.push(Message::KeyDown(GBKey::Down));
                    },
                    Event::KeyUp {keycode: Some(Keycode::Down), ..} => {
                        events.push(Message::KeyUp(GBKey::Down));
                    }

                    _ => {}
                }
            }
            // The rest of the game loop goes here...
            self.emu.get_next_frame(&events, &mut render_target);
            texture.update(None, &render_target[..], WIDTH * DEPTH).expect("Could not update texture");
            self.canvas.copy(&texture, None, None).expect("Could not render texture");
            self.canvas.present();
            ::std::thread::sleep(Duration::from_micros(1_000_000u64 / 60));
        }
    }
}


