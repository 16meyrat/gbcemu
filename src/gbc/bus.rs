
use super::ppu::Ppu;
use super::memory::Ram;
use super::cartridge::Cartridge;

pub struct Bus<'a>{
    pub ppu: Ppu,
    ram: Ram,
    pub cartridge: &'a mut dyn Cartridge,
    pub enabled_interrupts: u8,
    pub requested_interrupts: u8,
}

pub trait Busable {
    fn read(&self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, value: u8);
}

pub const VBLANK: u8 = 0x01;
pub const LCD_STAT: u8 = 0x02;
pub const TIMER: u8 = 0x04;
pub const SERIAL: u8 = 0x08;
pub const JOYPAD: u8 = 0x10;

impl<'a> Busable for Bus<'a> {
    fn read(&self, addr: u16) -> u8{
        match addr {
            x if x < 0x8000 => self.cartridge.read(addr),
            x if x < 0xa000 => self.ppu.read(addr),
            x if x < 0xc000 => self.cartridge.read(addr),
            x if x < 0xe000 => self.ram.read(addr),
            x if x < 0xFE00 => self.ram.read(addr - 0x2000),
            x if x >= 0xff80 && x < 0xfffe => self.ram.read(addr),
            0xffff => self.enabled_interrupts,
            0xff0f => self.requested_interrupts,
            0xff40 => self.ppu.get_lcdc(),
            0xff41 => self.ppu.get_lcds(),
            0xff42 => self.ppu.get_scy(),
            0xff43 => self.ppu.get_scx(),
            0xff44 => self.ppu.get_ly(),
            0xff45 => self.ppu.get_lcy(),
            0xff4a => self.ppu.get_wy(),
            0xff4b => self.ppu.get_wx(),
            0xff47 => self.ppu.get_bgp(),
            0xff48 => self.ppu.get_obp0(),
            0xff49 => self.ppu.get_obp1(),
            0xff01 => 0, // serial
            0xff02 => 0, // serial
            x if x >= 0xff10 && x < 0xff27 => 0, // sound
            x if x >= 0xff30 && x < 0xff40 => 0, // sound

            _ => panic!("Illegal read at {:#x}", addr)
        }
    }

    fn write(&mut self, addr: u16, value: u8){
        match addr {
            x if x < 0x8000 => self.cartridge.write(addr, value),
            x if x < 0xa000 => self.ppu.write(addr, value),
            x if x < 0xc000 => self.cartridge.write(addr, value),
            x if x < 0xe000 => self.ram.write(addr, value),
            x if x < 0xFE00 => self.ram.write(addr - 0x2000, value),
            x if x >= 0xff80 && x < 0xfffe => self.ram.write(addr, value),
            0xffff => self.enabled_interrupts = value,
            0xff0f => self.requested_interrupts = value,
            0xff40 => self.ppu.set_lcdc(value),
            0xff41 => self.ppu.set_lcds(value),
            0xff42 => self.ppu.set_scy(value),
            0xff43 => self.ppu.set_scx(value),
            0xff44 => self.ppu.set_ly(value),
            0xff45 => self.ppu.set_lcy(value),
            0xff4a => self.ppu.set_wy(value),
            0xff4b => self.ppu.set_wx(value),
            0xff47 => self.ppu.set_bgp(value),
            0xff48 => self.ppu.set_obp0(value),
            0xff49 => self.ppu.set_obp1(value),
            0xff01 => {}, // serial
            0xff02 => {}, // serial
            x if x >= 0xff10 && x < 0xff27 => {}, // sound
            x if x >= 0xff30 && x < 0xff40 => {}, // sound
            _ => panic!("Illegal write at {:#x}", addr)
        };
    }
}

impl<'a> Bus<'a> {
    pub fn new(cartridge: &'a mut dyn Cartridge) -> Bus<'a> {
        Bus {
            ppu: Ppu::new(),
            ram: Ram::new(),
            cartridge: cartridge,
            enabled_interrupts: 0xFF,
            requested_interrupts: 0xFF,
        }
    } 

    pub fn write16(&mut self, addr: u16, value: u16){
        self.write(addr, (value & 0xff) as u8);
        self.write(addr + 1, (value >> 8) as u8);
    }
}
