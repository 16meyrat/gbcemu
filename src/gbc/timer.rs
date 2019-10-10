use num_enum::IntoPrimitive;

pub struct Timer{
    enabled: bool,
    counter: u32,
    tima: u8,
    tma: u8,
    tac: TAC,
}

impl Timer {
    pub fn new() -> Self {
        Timer {
            enabled: false,
            counter: 0,
            tima: 0,
            tma: 0,
            tac: TAC::Clock0,
        }
    }

    // return whether timer interrupt needs to happen
    pub fn tick(&mut self) -> bool {
        if self.counter > 0 {
            self.counter -= 1;
            return false;
        }

        self.counter = self.tac as u32;

        if self.tima == 0xff {
            self.tima = self.tma;
            return true;
        } else {
            self.tima += 1;
            return false;
        }
    }

    pub fn set_tma(&mut self, tma: u8) {
        self.tma = tma;
    }

    pub fn get_tma(&self) -> u8 {
        self.tma
    }

    pub fn set_tima(&mut self, tima: u8) {
        self.tima = tima;
    }

    pub fn get_tima(&self) -> u8 {
        self.tima
    }

    pub fn set_tac(&mut self, tac: u8) {
        self.enabled = tac & 4 != 0;
        self.tac = match tac & 3 {
            0 => TAC::Clock0,
            1 => TAC::Clock1,
            2 => TAC::Clock2,
            3 => TAC::Clock3,
            _ => panic!("Invalid divider")
        }
    }

    pub fn get_tac(&self) -> u8 {
        (self.enabled as u8) << 2 | match self.tac {
            TAC::Clock0 => 0,
            TAC::Clock1 => 1,
            TAC::Clock2 => 2,
            TAC::Clock3 => 3,
        }
    }
}

#[derive(Clone, Copy, IntoPrimitive)]
#[repr(u16)]
enum TAC {
    Clock0 = 0x1024,
    Clock1 = 0x16,
    Clock2 = 0x64,
    Clock3 = 0x256,
}