mod gbc;
mod gui;

use gbc::bus::Bus;
use gbc::bus;

use gbc::cpu::Cpu;
use gbc::ppu::{PpuInterrupt};
use gbc::cartridge::load_rom;

use gui::{Message, Gui};

use std::thread;
use std::sync::{mpsc, Arc, Mutex};
use std::env;

extern crate num_enum;
extern crate sdl2;
extern crate arrayvec;

fn main() {

    if env::args().count() != 2 {
        eprintln!("Please enter the path to ROM.GB");
        return;
    }

    let rom_name = env::args().nth(1).unwrap();

    let (tx, rx) = mpsc::channel::<gui::Message>();
    let mut gui = Gui::new(tx);
    let texture = gui.get_texture();
    let emu_thread = thread::spawn(move || {
        run_emulator(rom_name, rx, texture);
    });
    gui.run();
    emu_thread.join().unwrap();
}

fn run_emulator(rom_name : String, rx: mpsc::Receiver::<gui::Message>, texture: Arc<Mutex<[u8; gui::SIZE]>>) {
    let mut rom = load_rom(&rom_name);
    let mut bus = Bus::new(&mut *rom, texture);
    let mut cpu = Cpu::new();

    cpu.reset();
    
    loop {
        cpu.tick(&mut bus);

        match bus.ppu.tick() {
            PpuInterrupt::None => {},
            PpuInterrupt::VBlank => {
                bus.requested_interrupts |= bus::VBLANK;
            }
            PpuInterrupt::Stat => {
                bus.requested_interrupts |= bus::LCD_STAT;
            }
        }

        if bus.timer.tick() {
            bus.requested_interrupts |= bus::TIMER;
        }

        match rx.try_recv() {
            Ok(event) => {
                match event {
                    Message::WindowClosed => break,
                    x => {
                        if bus.joypad.update(x) {
                            bus.requested_interrupts |= bus::JOYPAD
                        }
                    },
                }
            }
            Err(mpsc::TryRecvError::Disconnected) => break,
            Err(mpsc::TryRecvError::Empty) => {}
        }
    }
}