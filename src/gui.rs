use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use std::time::Duration;
use std::sync::{mpsc,Arc, Mutex};

pub const WIDTH: usize = 160;
pub const HEIGHT: usize = 144;
pub const DEPTH: usize = 3;
pub const SIZE: usize = WIDTH * HEIGHT * DEPTH;

enum GBKey {
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
    
        let window = video_subsystem.window("rust-sdl2 demo", 800, 600)
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
                    _ => {}
                }
            }
            // The rest of the game loop goes here...
            self.canvas.present();
            ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
        }

        self.tx.send(Message::WindowClosed).unwrap();
    } 

    pub fn get_texture(&self) -> Arc<Mutex<[u8; SIZE]>> {
        self.target.clone()
    }
}


