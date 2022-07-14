pub mod bus;
pub mod cartridge;
pub mod cpu;
pub mod sound;
pub mod input;
pub mod ppu;
pub mod timer;

mod memory;

use anyhow::Result;

use bus::Bus;

use cartridge::{load_rom};
use cpu::Cpu;
use ppu::{PpuInterrupt};

use crate::gui::{self, Message};

pub struct Emu {
    cpu: Cpu,
    bus: Bus,
}

impl Emu {
    pub fn new(rom_name: &str) -> Result<Self> {
        let rom = load_rom(rom_name)?;
        let bus = Bus::new(rom)?;
        let mut cpu = Cpu::new();

        cpu.reset();

        Ok(Self { cpu, bus })
    }

    pub fn get_next_frame(&mut self, events: &[Message], rendering_texture: &mut [u8; gui::SIZE]) {
        let mut frame_done = false;
        loop {
            self.cpu.tick(&mut self.bus);

            match self.bus.ppu.tick() {
                PpuInterrupt::None => {}
                PpuInterrupt::VBlank => {
                    self.bus.requested_interrupts |= bus::VBLANK;
                    frame_done = true;
                }
                PpuInterrupt::Stat => {
                    self.bus.requested_interrupts |= bus::LCD_STAT;
                }
            }

            if self.bus.timer.tick() {
                self.bus.requested_interrupts |= bus::TIMER;
            }
            for ev in events {
                if self.bus.joypad.update(ev) {
                    self.bus.requested_interrupts |= bus::JOYPAD
                };
            }

            if frame_done {
                self.bus.ppu.render(rendering_texture);
                break;
            }
        }
    }
}
