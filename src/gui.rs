use sdl2::GameControllerSubsystem;
use sdl2::controller::Button;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use std::time::{Duration, Instant};
use std::env;
use anyhow::Result;

use crate::gbc::Emu;

pub const WIDTH: usize = 160;
pub const HEIGHT: usize = 144;
pub const DEPTH: usize = 3;
pub const SIZE: usize = WIDTH * HEIGHT * DEPTH;

#[derive(Debug, Clone, Copy)]
pub enum GBKey {
    Up,
    Down,
    Left,
    Right,
    A,
    B,
    Start,
    Select,
}

pub enum Message {
    KeyUp(GBKey),
    KeyDown(GBKey),
}

pub struct Gui {
    context: sdl2::Sdl,
    gamepad_subsystem: GameControllerSubsystem,
    canvas: sdl2::render::Canvas<sdl2::video::Window>,
    emu: Emu,

    last_time: Instant,
    last_sleep: Duration,
}

impl Gui {
    pub fn new(emu: Emu) -> Result<Self> {
        let sdl_context = sdl2::init().map_err(|e|anyhow::anyhow!(e))?;
        let video_subsystem = sdl_context.video().map_err(|e|anyhow::anyhow!(e))?;
        let gamepad_subsystem = sdl_context.game_controller().map_err(|e|anyhow::anyhow!(e))?;
        if let Ok(p) = env::var("SDL_JOYSTICK_MAPPINGS") {
            gamepad_subsystem.load_mappings(&p)?;
            println!("Loaded mapping from {p}");
        }
        let window = video_subsystem
            .window("yaGBemu", WIDTH as u32 * 4, HEIGHT as u32 * 4)
            .position_centered()
            .build()?;

        let mut canvas = window.into_canvas().build()?;

        canvas.clear();
        canvas.present();

        Ok(Gui {
            context: sdl_context,
            gamepad_subsystem,
            canvas,
            emu,

            last_time: Instant::now(),
            last_sleep: Duration::from_millis(0),
        })
    }
    pub fn run(&mut self) {
        let mut event_pump = self.context.event_pump().unwrap();

        let mut gamepads = vec![];
        for which in 0..self.gamepad_subsystem.num_joysticks().unwrap() {
            if self.gamepad_subsystem.is_game_controller(which) {
                println!("Added gamepad {}", self.gamepad_subsystem.name_for_index(which).unwrap());
                gamepads.push(self.gamepad_subsystem.open(which).unwrap());
            }
        }

        let texture_creator = self.canvas.texture_creator();
        let mut texture = texture_creator
            .create_texture_static(PixelFormatEnum::RGB24, WIDTH as u32, HEIGHT as u32)
            .expect("Could not allocate texture");

        let mut render_target = Box::new([0u8; SIZE]);

        'running: loop {
            let mut events = vec![];

            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit { .. }
                    | Event::KeyDown {
                        keycode: Some(Keycode::Escape),
                        ..
                    } => break 'running,
                    Event::KeyDown {
                        keycode: Some(Keycode::Return),
                        ..
                    } => {
                        events.push(Message::KeyDown(GBKey::Start));
                    }
                    Event::KeyUp {
                        keycode: Some(Keycode::Return),
                        ..
                    } => {
                        events.push(Message::KeyUp(GBKey::Start));
                    }
                    Event::KeyDown {
                        keycode: Some(Keycode::Backspace),
                        ..
                    } => {
                        events.push(Message::KeyDown(GBKey::Select));
                    }
                    Event::KeyUp {
                        keycode: Some(Keycode::Backspace),
                        ..
                    } => {
                        events.push(Message::KeyUp(GBKey::Select));
                    }
                    Event::KeyDown {
                        keycode: Some(Keycode::Q),
                        ..
                    } => {
                        events.push(Message::KeyDown(GBKey::B));
                    }
                    Event::KeyUp {
                        keycode: Some(Keycode::Q),
                        ..
                    } => {
                        events.push(Message::KeyUp(GBKey::B));
                    }
                    Event::KeyDown {
                        keycode: Some(Keycode::S),
                        ..
                    } => {
                        events.push(Message::KeyDown(GBKey::A));
                    }
                    Event::KeyUp {
                        keycode: Some(Keycode::S),
                        ..
                    } => {
                        events.push(Message::KeyUp(GBKey::A));
                    }
                    Event::KeyDown {
                        keycode: Some(Keycode::Left),
                        ..
                    } => {
                        events.push(Message::KeyDown(GBKey::Left));
                    }
                    Event::KeyUp {
                        keycode: Some(Keycode::Left),
                        ..
                    } => {
                        events.push(Message::KeyUp(GBKey::Left));
                    }
                    Event::KeyDown {
                        keycode: Some(Keycode::Right),
                        ..
                    } => {
                        events.push(Message::KeyDown(GBKey::Right));
                    }
                    Event::KeyUp {
                        keycode: Some(Keycode::Right),
                        ..
                    } => {
                        events.push(Message::KeyUp(GBKey::Right));
                    }
                    Event::KeyDown {
                        keycode: Some(Keycode::Up),
                        ..
                    } => {
                        events.push(Message::KeyDown(GBKey::Up));
                    }
                    Event::KeyUp {
                        keycode: Some(Keycode::Up),
                        ..
                    } => {
                        events.push(Message::KeyUp(GBKey::Up));
                    }
                    Event::KeyDown {
                        keycode: Some(Keycode::Down),
                        ..
                    } => {
                        events.push(Message::KeyDown(GBKey::Down));
                    }
                    Event::KeyUp {
                        keycode: Some(Keycode::Down),
                        ..
                    } => {
                        events.push(Message::KeyUp(GBKey::Down));
                    }
                    Event::ControllerButtonDown { button, .. } => {
                        events.push(
                            if let Some(gb_key) = controller_to_gb_key(&button) {
                                Message::KeyDown(gb_key)
                            } else {
                                continue;
                            }
                        )
                    }
                    Event::ControllerButtonUp { button, .. } => {
                        events.push(
                            if let Some(gb_key) = controller_to_gb_key(&button) {
                                Message::KeyUp(gb_key)
                            } else {
                                continue;
                            }
                        )
                    }
                    Event::ControllerDeviceAdded { which,.. } => {
                        println!("Added gamepad {}", self.gamepad_subsystem.name_for_index(which).unwrap());
                        gamepads.push(self.gamepad_subsystem.open(which).unwrap());
                    }
                    Event::ControllerDeviceRemoved { which,.. } => {
                        if let Some(g) = gamepads.iter().find(|g|g.instance_id() == which) {
                            println!("Disconnected gamepad {}", g.name());
                            gamepads.retain(|c|c.instance_id() != which);
                        }
                    }
                    _ => {}
                }
            }
            // The rest of the game loop goes here...
            self.emu.get_next_frame(&events, &mut render_target);
            texture
                .update(None, &render_target[..], WIDTH * DEPTH)
                .expect("Could not update texture");
            self.canvas
                .copy(&texture, None, None)
                .expect("Could not render texture");
            self.canvas.present();

            let last_time = self.last_time;
            self.last_time = Instant::now();
            let elapsed = self.last_time.duration_since(last_time).as_micros();
            let sleep = self.last_sleep.as_micros() as i128 + (16_666 - elapsed as i128);
            //println!("Fps: {}", 1e6 / elapsed as f64);
            if sleep > 0 {
                self.last_sleep = Duration::from_micros(sleep as u64);
                std::thread::sleep(self.last_sleep);
            } else {
                self.last_sleep = Duration::from_micros(0);
                std::thread::yield_now();
            }
        }
    }
}

fn controller_to_gb_key(sdl_key: &Button) -> Option<GBKey> {
    match sdl_key {
        Button::A => Some(GBKey::A),
        Button::B => Some(GBKey::B),
        Button::Back => Some(GBKey::Select),
        Button::Start => Some(GBKey::Start),
        Button::DPadUp => Some(GBKey::Up),
        Button::DPadDown => Some(GBKey::Down),
        Button::DPadLeft => Some(GBKey::Left),
        Button::DPadRight => Some(GBKey::Right),
        _ => None
    }
}