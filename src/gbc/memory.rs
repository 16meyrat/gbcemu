use super::bus::Busable;


pub struct Ram {
    bank0: [u8; 0x1000],
    banks: Vec<[u8; 0x1000]>,
    high_ram: [u8; 0x7f],
    current_bank: usize,
}

impl Ram {
    pub fn new() -> Self{
        Ram{
            bank0: [0; 0x1000],
            banks: vec![[0; 0x1000]; 6],
            high_ram: [0; 0x7f],
            current_bank: 0,
        }
    }
}

impl Busable for Ram{
    fn read(&self, addr: u16) -> u8{
        if addr >= 0xc000 && addr < 0xd000 {
            return self.bank0[(addr - 0xc000) as usize];
        }else if addr >= 0xff80 && addr < 0xffff {
            return self.high_ram[(addr - 0xff80) as usize];
        }else if addr >= 0xd000 && addr < 0xe000 {
            return self.banks[self.current_bank][(addr - 0xd000) as usize];
        }else {
            panic!("Invalid RAM read at {:x}", addr);
        }
    }
    fn write(&mut self, addr: u16, val: u8){
        if addr >= 0xc000 && addr < 0xd000 {
            self.bank0[(addr - 0xc000) as usize] = val;
        }else if addr >= 0xff80 && addr < 0xffff {
            self.high_ram[(addr - 0xff80) as usize] = val;
        }else if addr >= 0xd000 && addr < 0xe000 {
            self.banks[self.current_bank][(addr - 0xd000) as usize] = val;
        }else {
            panic!("Invalid RAM write at {:x}", addr);
        }
    }
}