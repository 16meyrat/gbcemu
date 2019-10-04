
use super::bus::Bus;

pub struct Cpu<'a> {
    bus: &'a mut Bus<'a>,
}

impl<'a> Cpu<'a> {
    pub fn new(bus: &'a mut Bus<'a>) -> Self {
        Cpu{
            bus: bus,
        }
    }

    pub fn tick() {
        
    }
}


