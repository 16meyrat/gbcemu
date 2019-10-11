use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use std::time::Duration;
use std::sync::{mpsc,Arc, Mutex};

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
    target: Arc<Mutex<[u8; SIZE]>>,
    tx: mpsc::Sender<Message>,
}

impl Gui{
    pub fn new(tx: mpsc::Sender<Message>) -> Self {
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
            target: Arc::new(Mutex::new([0; SIZE])),
            tx
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

        'running: loop {
            {
                let target = self.target.lock().expect("Mutex error");
                texture.update(None, &target[..], WIDTH * DEPTH).expect("Could not update texture");
            }
            self.canvas.copy(&texture, None, None).expect("Could not render texture");
            
            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit {..} |
                    Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                        break 'running
                    },
                    Event::KeyDown {keycode: Some(Keycode::Return), ..} => {
                        self.tx.send(Message::KeyDown(GBKey::Start)).unwrap();
                    },
                    Event::KeyUp {keycode: Some(Keycode::Return), ..} => {
                        self.tx.send(Message::KeyUp(GBKey::Start)).unwrap();
                    },
                    Event::KeyDown {keycode: Some(Keycode::Backspace), ..} => {
                        self.tx.send(Message::KeyDown(GBKey::Select)).unwrap();
                    },
                    Event::KeyUp {keycode: Some(Keycode::Backspace), ..} => {
                        self.tx.send(Message::KeyUp(GBKey::Select)).unwrap();
                    },
                    Event::KeyDown {keycode: Some(Keycode::Q), ..} => {
                        self.tx.send(Message::KeyDown(GBKey::B)).unwrap();
                    },
                    Event::KeyUp {keycode: Some(Keycode::Q), ..} => {
                        self.tx.send(Message::KeyUp(GBKey::B)).unwrap();
                    },
                    Event::KeyDown {keycode: Some(Keycode::S), ..} => {
                        self.tx.send(Message::KeyDown(GBKey::A)).unwrap();
                    },
                    Event::KeyUp {keycode: Some(Keycode::S), ..} => {
                        self.tx.send(Message::KeyUp(GBKey::A)).unwrap();
                    },
                    Event::KeyDown {keycode: Some(Keycode::Left), ..} => {
                        self.tx.send(Message::KeyDown(GBKey::Left)).unwrap();
                    },
                    Event::KeyUp {keycode: Some(Keycode::Left), ..} => {
                        self.tx.send(Message::KeyUp(GBKey::Left)).unwrap();
                    }
                    Event::KeyDown {keycode: Some(Keycode::Right), ..} => {
                        self.tx.send(Message::KeyDown(GBKey::Right)).unwrap();
                    },
                    Event::KeyUp {keycode: Some(Keycode::Right), ..} => {
                        self.tx.send(Message::KeyUp(GBKey::Right)).unwrap();
                    },
                    Event::KeyDown {keycode: Some(Keycode::Up), ..} => {
                        self.tx.send(Message::KeyDown(GBKey::Up)).unwrap();
                    },
                    Event::KeyUp {keycode: Some(Keycode::Up), ..} => {
                        self.tx.send(Message::KeyUp(GBKey::Up)).unwrap();
                    },
                    Event::KeyDown {keycode: Some(Keycode::Down), ..} => {
                        self.tx.send(Message::KeyDown(GBKey::Down)).unwrap();
                    },
                    Event::KeyUp {keycode: Some(Keycode::Down), ..} => {
                        self.tx.send(Message::KeyUp(GBKey::Down)).unwrap();
                    }

                    _ => {}
                }
            }
            // The rest of the game loop goes here...
            self.canvas.present();
            ::std::thread::sleep(Duration::from_micros(1_000_000u64 / 60));
        }

        self.tx.send(Message::WindowClosed).unwrap();
    } 

    pub fn get_texture(&self) -> Arc<Mutex<[u8; SIZE]>> {
        self.target.clone()
    }
}


