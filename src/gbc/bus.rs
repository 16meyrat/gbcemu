
use super::ppu::Ppu;
use super::memory::Ram;
use super::cartridge::Cartridge;

pub struct Bus<'a>{
    ppu: Ppu,
    ram: Ram,
    pub cartridge: &'a mut dyn Cartridge,
}

pub trait Busable {
    fn read(&self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, value: u8);
    fn write16(&mut self, addr: u16, value: u16);
}

impl<'a> Busable for Bus<'a> {
    fn read(&self, addr: u16) -> u8{
        match addr {
            x if x < 0x800 => self.cartridge.read(addr),
            x if x < 0xa00 => self.ppu.read(addr),
            x if x < 0xc00 => self.cartridge.read(addr),
            x if x < 0xe00 => self.ram.read(addr),
            x if x < 0xFE00 => self.ram.read(addr - 0x2000),
            _ => 0
        }
    }

    fn write(&mut self, addr: u16, value: u8){
        match addr {
            x if x < 0x800 => self.cartridge.write(addr, value),
            x if x < 0xa00 => self.ppu.write(addr, value),
            x if x < 0xc00 => self.cartridge.write(addr, value),
            x if x < 0xe00 => self.ram.write(addr, value),
            x if x < 0xFE00 => self.ram.write(addr - 0x2000, value),
            _ => {}
        };
    }
    fn write16(&mut self, addr: u16, value: u16){
        match addr {
            x if x < 0x800 => self.cartridge.write16(addr, value),
            x if x < 0xa00 => self.ppu.write16(addr, value),
            x if x < 0xc00 => self.cartridge.write16(addr, value),
            x if x < 0xe00 => self.ram.write16(addr, value),
            x if x < 0xFE00 => self.ram.write16(addr - 0x2000, value),
            _ => {}
        };
    }
}

impl<'a> Bus<'a> {
    pub fn new(cartridge: &'a mut dyn Cartridge) -> Bus<'a> {
        Bus {
            ppu: Ppu::new(),
            ram: Ram::new(),
            cartridge: cartridge
        }
    } 
}
