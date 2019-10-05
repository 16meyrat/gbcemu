mod gbc;

use gbc::bus::Bus;
use gbc::cpu::Cpu;
use gbc::cartridge::load_rom;

extern crate num_enum;

fn main() {
    run_emulator();
}

fn run_emulator() {
    let mut rom = load_rom("Tetris.GB");
    let mut bus = Bus::new(&mut *rom);
    let mut cpu = Cpu::new(& mut bus);

    loop {
        cpu.tick();
    }
}