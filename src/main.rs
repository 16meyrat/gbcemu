mod gbc;
mod gui;

use gbc::bus::Bus;
use gbc::cpu::Cpu;
use gbc::cartridge::load_rom;

use gui::{Message, Gui};

use std::thread;
use std::sync::{mpsc, Arc, Mutex};

extern crate num_enum;
extern crate sdl2;

fn main() {
    let (tx, rx) = mpsc::channel::<gui::Message>();
    let mut gui = Gui::new(tx);
    let texture = gui.get_texture();
    let emu_thread = thread::spawn(move || {
        run_emulator(rx, texture);
    });
    gui.run();
    emu_thread.join().unwrap();
}

fn run_emulator(rx: mpsc::Receiver::<gui::Message>, texture: Arc<Mutex<[u8; gui::SIZE]>>) {
    let mut rom = load_rom("Tetris.GB");
    let mut bus = Bus::new(&mut *rom);
    let mut cpu = Cpu::new(& mut bus);

    cpu.reset();

    {
        let mut tex = texture.lock().expect("Mutex error");
        for (i, byte) in tex.iter_mut().enumerate() {
            if i % 3 == 0 {
                *byte = 255;
            }
        }
    }
    
    loop {
        cpu.tick();

        match rx.try_recv() {
            Ok(event) => {
                match event {
                    Message::WindowClosed => break,
                    _ => {}
                }
            }
            Err(mpsc::TryRecvError::Disconnected) => break,
            Err(mpsc::TryRecvError::Empty) => {}
        }
    }
}