
use super::bus::Busable;
use super::bus::Bus;

pub struct Cpu {

}

impl Cpu {
    pub fn new(bus: ) {
        return {
            
        };
    }
}

impl Busable for Cpu {
    fn read(&self, addr: u16) -> u8{
        0
    }

    fn write(&mut self, addr: u16, value: u8){
        
    }
}

