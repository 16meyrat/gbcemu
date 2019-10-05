use super::bus::Busable;


pub struct Ppu {
    
}

impl Ppu {
    pub fn new() -> Self {
        Ppu{}
    }
}

impl Busable for Ppu {
    fn read(&self, addr: u16) -> u8{
        0
    }
    fn write(&mut self, addr: u16, val: u8){

    }

    fn write16(&mut self, addr: u16, val: u16){

    }
}
