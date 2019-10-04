
use super::ppu::Ppu;
use super::memory::Ram;
use super::cartridge::Cartridge;

pub struct Bus<'a>{
    ppu: Ppu,
    ram: Ram,
    cartridge: &'a mut Cartridge,
}

pub trait Busable {
    fn read(&self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, value: u8);
}

impl<'a> Busable for Bus<'a> {
    fn read(&self, addr: u16) -> u8{
        0
    }

    fn write(&mut self, addr: u16, value: u8){

    }
}

impl<'a> Bus<'a> {
    pub fn new(cartridge: &'a mut Cartridge) -> Bus<'a> {
        Bus {
            ppu: Ppu::new(),
            ram: Ram::new(),
            cartridge: cartridge
        }
    } 
}
