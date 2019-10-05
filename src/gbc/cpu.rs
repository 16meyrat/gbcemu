
use super::bus::Bus;
use super::bus::Busable;

pub struct Cpu<'a> {
    bus: &'a mut Bus<'a>,

    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    pc: u16,
    sp: u16,
    
    zerof: u8,
    add_subf: u8,
    half_carryf: u8,
    carryf: u8,

    wait: u8,
}

const ZERO : u8 = 7;
const ADDSUB : u8 = 6;
const HALFCARRY : u8 = 6;
const CARRY : u8 = 4;

impl<'a> Cpu<'a> {
    pub fn new(bus: &'a mut Bus<'a>) -> Self {
        Cpu{
            bus: bus,

            a: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            pc: 0,
            sp: 0,
            
            zerof: 0,
            add_subf: 0,
            half_carryf: 0,
            carryf: 0,

            wait: 0,
        }
    }

    pub fn reset(&mut self) {
        self.a = 0;
        self.b = 0;
        self.c = 0;
        self.d = 0;
        self.e = 0;
        self.h = 0;
        self.l = 0;
        self.pc = 0;
        
        self.zerof = 0;
        self.add_subf = 0;
        self.half_carryf = 0;
        self.carryf = 0;
    }

    fn flags(&self) -> u8 {
        self.zerof << ZERO | self.add_subf << ADDSUB | self.half_carryf << HALFCARRY | self.carryf << CARRY
    }

    fn set_flags(& mut self, f: u8) {
        self.zerof = f & 1 << ZERO;
        self.add_subf = f & 1 << ADDSUB;
        self.half_carryf = f & 1 << HALFCARRY;
        self.carryf = f & 1 << CARRY;
    }

    pub fn tick(&mut self) {
        macro_rules! disasm {
            ($($arg:tt)+) => (
                if cfg!(debug_assertions) {
                    print!("0x{:<8x}: ", self.pc);
                    println!($($arg)+);
                }
            )
        }
        let op = self.bus.cartridge.read(self.pc);
        match op {
            0x0 => {
                self.wait = 4;
                disasm!("NOP");
            }
            0x1 => {
                self.c = self.bus.read(self.pc);
                self.pc += 1;
                self.b = self.bus.read(self.pc);
                self.pc += 1;
                self.wait = 12;
                disasm!("LD BC, 0x{:x}{:x}", self.b, self.c);
            }
            0x11 => {
                self.e = self.bus.read(self.pc);
                self.pc += 1;
                self.d = self.bus.read(self.pc);
                self.pc += 1;
                self.wait = 12;
                disasm!("LD DE, 0x{:x}{:x}", self.d, self.e);
            }
            0x21 => {
                self.l = self.bus.read(self.pc);
                self.pc += 1;
                self.h = self.bus.read(self.pc);
                self.pc += 1;
                self.wait = 12;
                disasm!("LD HL, 0x{:x}{:x}", self.h, self.l);
            }
            0x31 => {
                self.sp = self.bus.read(self.pc) as u16 | (self.bus.read(self.pc+1) as u16) << 8;
                self.pc += 2;
                self.wait = 12;
                disasm!("LD HL, 0x{:x}", self.sp);
            }
            0x40 => {
                self.b = self.b;
                self.wait = 4;
                disasm!("LD B, B");
            }
            0x50 => {
                self.d = self.b;
                self.wait = 4;
                disasm!("LD D, B");
            }
            0x60 => {
                self.h = self.b;
                self.wait = 4;
                disasm!("LD H, B");
            }
            0x41 => {
                self.b = self.c;
                self.wait = 4;
                disasm!("LD B, C");
            }
            0x51 => {
                self.d = self.c;
                self.wait = 4;
                disasm!("LD D, C");
            }
            0x61 => {
                self.h = self.c;
                self.wait = 4;
                disasm!("LD H, C");
            }
            0x42 => {
                self.b = self.d;
                self.wait = 4;
                disasm!("LD B, D");
            }
            0x52 => {
                self.d = self.d;
                self.wait = 4;
                disasm!("LD D, D");
            }
            0x62 => {
                self.h = self.d;
                self.wait = 4;
                disasm!("LD H, D");
            }
            0x43 => {
                self.b = self.e;
                self.wait = 4;
                disasm!("LD B, E");
            }
            0x53 => {
                self.d = self.e;
                self.wait = 4;
                disasm!("LD D, E");
            }
            0x63 => {
                self.h = self.e;
                self.wait = 4;
                disasm!("LD H, E");
            }
            0x44 => {
                self.b = self.h;
                self.wait = 4;
                disasm!("LD B, H");
            }
            0x54 => {
                self.d = self.h;
                self.wait = 4;
                disasm!("LD D, H");
            }
            0x64 => {
                self.h = self.h;
                self.wait = 4;
                disasm!("LD H, H");
            }
            0x45 => {
                self.b = self.l;
                self.wait = 4;
                disasm!("LD B, L");
            }
            0x55 => {
                self.d = self.l;
                self.wait = 4;
                disasm!("LD D, L");
            }
            0x65 => {
                self.h = self.l;
                self.wait = 4;
                disasm!("LD H, L");
            }
            0x46 => {
                self.b = self.bus.read((self.h as u16) << 8 | self.l as u16);
                self.wait = 8;
                disasm!("LD B, (HL)");
            }
            0x56 => {
                self.d = self.bus.read((self.h as u16) << 8 | self.l as u16);
                self.wait = 8;
                disasm!("LD D, (HL)");
            }
            0x66 => {
                self.h = self.bus.read((self.h as u16) << 8 | self.l as u16);
                self.wait = 8;
                disasm!("LD H, (HL)");
            }
            0x47 => {
                self.b = self.a;
                self.wait = 4;
                disasm!("LD B, A");
            }
            0x57 => {
                self.d = self.a;
                self.wait = 4;
                disasm!("LD D, A");
            }
            0x67 => {
                self.h = self.a;
                self.wait = 4;
                disasm!("LD H, A");
            }
            0x48 => {
                self.c = self.b;
                self.wait = 4;
                disasm!("LD C, B");
            }
            0x58 => {
                self.e = self.b;
                self.wait = 4;
                disasm!("LD E, B");
            }
            0x68 => {
                self.l = self.b;
                self.wait = 4;
                disasm!("LD L, B");
            }
            0x78 => {
                self.a = self.b;
                self.wait = 4;
                disasm!("LD A, B");
            }
            0x49 => {
                self.c = self.c;
                self.wait = 4;
                disasm!("LD C, C");
            }
            0x59 => {
                self.e = self.c;
                self.wait = 4;
                disasm!("LD E, C");
            }
            0x69 => {
                self.l = self.c;
                self.wait = 4;
                disasm!("LD L, C");
            }
            0x79 => {
                self.a = self.c;
                self.wait = 4;
                disasm!("LD A, C");
            }
            0x4a => {
                self.c = self.d;
                self.wait = 4;
                disasm!("LD C, D");
            }
            0x5a => {
                self.e = self.d;
                self.wait = 4;
                disasm!("LD E, D");
            }
            0x6a => {
                self.l = self.d;
                self.wait = 4;
                disasm!("LD L, D");
            }
            0x7a => {
                self.a = self.d;
                self.wait = 4;
                disasm!("LD A, D");
            }
            0x4b => {
                self.c = self.e;
                self.wait = 4;
                disasm!("LD C, E");
            }
            0x5b => {
                self.e = self.e;
                self.wait = 4;
                disasm!("LD E, E");
            }
            0x6b => {
                self.l = self.e;
                self.wait = 4;
                disasm!("LD L, E");
            }
            0x7b => {
                self.a = self.e;
                self.wait = 4;
                disasm!("LD A, E");
            }
            0x4c => {
                self.c = self.h;
                self.wait = 4;
                disasm!("LD C, H");
            }
            0x5c => {
                self.e = self.h;
                self.wait = 4;
                disasm!("LD E, H");
            }
            0x6c => {
                self.l = self.h;
                self.wait = 4;
                disasm!("LD L, H");
            }
            0x7c => {
                self.a = self.h;
                self.wait = 4;
                disasm!("LD A, H");
            }
            0x4d => {
                self.c = self.l;
                self.wait = 4;
                disasm!("LD C, L");
            }
            0x5d => {
                self.e = self.l;
                self.wait = 4;
                disasm!("LD E, L");
            }
            0x6d => {
                self.l = self.l;
                self.wait = 4;
                disasm!("LD L, L");
            }
            0x7d => {
                self.a = self.l;
                self.wait = 4;
                disasm!("LD A, L");
            }
            0x4e => {
                self.c = self.bus.read((self.h as u16) << 8 | self.l as u16);
                self.wait = 8;
                disasm!("LD C, (HL)");
            }
            0x5e => {
                self.e = self.bus.read((self.h as u16) << 8 | self.l as u16);
                self.wait = 8;
                disasm!("LD E, (HL)");
            }
            0x6e => {
                self.l = self.bus.read((self.h as u16) << 8 | self.l as u16);
                self.wait = 8;
                disasm!("LD L, (HL)");
            }
            0x7e => {
                self.a = self.bus.read((self.h as u16) << 8 | self.l as u16);         
                self.wait = 8;
                disasm!("LD A, (HL)");
            }
            0x4f => {
                self.c = self.a;
                self.wait = 4;
                disasm!("LD C, A");
            }
            0x5f => {
                self.e = self.a;
                self.wait = 4;
                disasm!("LD E, A");
            }
            0x6f => {
                self.l = self.a;
                self.wait = 4;
                disasm!("LD L, A");
            }
            0x7f => {
                self.a = self.a;
                self.wait = 4;
                disasm!("LD A, A");
            }
            0x70 => {
                self.bus.write((self.h as u16) << 8 | self.l as u16, self.b);
                self.wait = 8;
                disasm!("LD (HL), B");
            }
            0x72 => {
                self.bus.write((self.h as u16) << 8 | self.l as u16, self.d);
                self.wait = 8;
                disasm!("LD (HL), D");
            }
            0x73 => {
                self.bus.write((self.h as u16) << 8 | self.l as u16, self.e);
                self.wait = 8;
                disasm!("LD (HL), E");
            }
            0x74 => {
                self.bus.write((self.h as u16) << 8 | self.l as u16, self.h);
                self.wait = 8;
                disasm!("LD (HL), H");
            }
            0x75 => {
                self.bus.write((self.h as u16) << 8 | self.l as u16, self.l);
                self.wait = 8;
                disasm!("LD (HL), L");
            }
            0x77 => {
                self.bus.write((self.h as u16) << 8 | self.l as u16, self.a);
                self.wait = 8;
                disasm!("LD (HL), A");
            }
            0x02 => {
                self.bus.write((self.b as u16) << 8 | self.c as u16, self.a);
                self.wait = 8;
                disasm!("LD (BC), A");
            }
            0x12 => {
                self.bus.write((self.d as u16) << 8 | self.e as u16, self.a);
                self.wait = 8;
                disasm!("LD (DE), A");
            }
            0x22 => {
                let mut hl = (self.h as u16) << 8 | self.h as u16;
                self.bus.write(hl, self.a);
                hl += 1;
                self.h = (hl >> 8) as u8;
                self.l = (hl & 0xff) as u8;
                self.wait = 8;
                disasm!("LD (HL+), A");
            }
            0x32 => {
                let mut hl = (self.h as u16) << 8 | self.h as u16;
                self.bus.write(hl, self.a);
                hl -= 1;
                self.h = (hl >> 8) as u8;
                self.l = (hl & 0xff) as u8;
                self.wait = 8;
                disasm!("LD (HL-), A");
            }
            0x06 => {
                self.b = self.bus.cartridge.read(self.pc + 1);
                self.pc += 1;
                self.wait = 8;
                disasm!("LD B, d8");
            }
            0x16 => {
                self.d = self.bus.cartridge.read(self.pc + 1);
                self.pc += 1;
                self.wait = 8;
                disasm!("LD D, d8");
            }
            0x26 => {
                self.h = self.bus.cartridge.read(self.pc + 1);
                self.pc += 1;
                self.wait = 8;
                disasm!("LD H, d8");
            }
            0x36 => {
                self.bus.write((self.h as u16) << 8 | self.l as u16, self.bus.cartridge.read(self.pc + 1));
                self.pc += 1;
                self.wait = 12;
                disasm!("LD (HL), d8");
            }
            _ => {
                eprintln!("Unknown opcode at 0x{:x} : 0x{:x}", self.pc, op);
            }
        };
        self.pc += 1;
    }

}

