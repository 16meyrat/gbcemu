mod gbc;

use gbc::bus::Bus;
use gbc::cpu::Cpu;
use gbc::cartridge::Cartridge;

fn main() {
    println!("Hello, world!");
    runEmulator();
}

fn runEmulator() {
    let mut cartridge = Cartridge::new("Tetris.GB");
    let mut bus = Bus::new(& mut cartridge);
    let mut cpu = Cpu::new(& mut bus);
}