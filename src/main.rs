mod gbc;
mod gui;

use gbc::bus::Bus;
use gbc::bus;

use gbc::cpu::Cpu;
use gbc::ppu::{PpuInterrupt};
use gbc::cartridge::load_rom;

use gui::{Message, Gui};

use std::rc::Rc;
use std::cell::RefCell;

use std::thread;
use std::sync::{mpsc, Arc, Mutex};

extern crate num_enum;
extern crate sdl2;
extern crate arrayvec;

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
    let mut cpu = Cpu::new();

    cpu.reset();

    {
        let mut tex = texture.lock().expect("Mutex error");
        for (i, byte) in tex.iter_mut().enumerate() {
            if i % 3 == 1 {
                *byte = 255;
            }
        }
    }
    
    loop {
        cpu.tick(&mut bus);

        match bus.ppu.tick() {
            PpuInterrupt::None => {},
            PpuInterrupt::VBlank => bus.requested_interrupts |= bus::VBLANK,
            PpuInterrupt::Stat => bus.requested_interrupts |= bus::LCD_STAT,
        };

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