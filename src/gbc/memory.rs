use super::bus::Busable;


pub struct Ram {
    bank0: [u8; 0x1000],
    banks: Vec<[u8; 0x1000]>,
    current_bank: usize,
}

impl Ram {
    pub fn new() -> Self{
        Ram{
            bank0: [0; 0x1000],
            banks: vec![[0; 0x1000]; 6],
            current_bank: 0,
        }
    }
}

impl Busable for Ram{
    fn read(&self, addr: u16) -> u8{
        if addr >= 0xc000 && addr < 0xd000 {
            return self.bank0[(addr - 0xc000) as usize];
        }else if addr >= 0xd000 && addr < 0xe000 {
            return self.banks[self.current_bank][(addr - 0xd000) as usize];
        }else {
            panic!("Invalid RAM read at {:x}", addr);
        }
    }
    fn write(&mut self, addr: u16, val: u8){
        if addr >= 0xc000 && addr < 0xd000 {
            self.bank0[(addr - 0xc000) as usize] = val;
        }else if addr >= 0xd000 && addr < 0xe000 {
            self.banks[self.current_bank][(addr - 0xd000) as usize] = val;
        }else {
            panic!("Invalid RAM write at {:x}", addr);
        }
    }

    fn write16(&mut self, addr: u16, val: u16){
        self.write(addr, (val & 0xff) as u8);
        self.write(addr + 1, (val >> 8) as u8);
    }
}