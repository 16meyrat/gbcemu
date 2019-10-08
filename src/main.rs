mod gbc;
mod gui;

use gbc::bus::Bus;
use gbc::cpu::Cpu;
use gbc::cartridge::load_rom;

use std::thread;
use std::sync::mpsc;

extern crate num_enum;
extern crate nannou;

fn main() {
    let (rx, tx) = mpsc::channel::<gui::Message>();
    gui::run_gui();
}

fn run_emulator() {
    let mut rom = load_rom("Tetris.GB");
    let mut bus = Bus::new(&mut *rom);
    let mut cpu = Cpu::new(& mut bus);

    cpu.reset();
    
    loop {
        cpu.tick();
    }
}