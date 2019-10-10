use super::bus::*;

pub struct Cpu {
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

    wait: i32,
    interrupts_enabled: bool,
}

const ZERO : u8 = 7;
const ADDSUB : u8 = 6;
const HALFCARRY : u8 = 6;
const CARRY : u8 = 4;

impl Cpu {
    pub fn new() -> Self {
        Cpu{
            a: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            pc: 0,
            sp: 0xfffe,
            
            zerof: 0,
            add_subf: 0,
            half_carryf: 0,
            carryf: 0,

            wait: 0,
            interrupts_enabled: false,
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
        self.sp = 0xfff4;
        
        self.zerof = 0;
        self.add_subf = 0;
        self.half_carryf = 0;
        self.carryf = 0;
        self.interrupts_enabled = false;
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

    fn call(& mut self, bus: &mut Bus, addr: u16,){
        bus.write16(self.sp - 2, self.pc + 1);
        self.sp -= 2;
        self.pc = u16::wrapping_sub(addr, 1);
        self.wait = 16;
    }

    fn ret(& mut self, bus: &mut Bus){
        let l = bus.read(self.sp);
        let h = bus.read(self.sp + 1);
        self.sp += 2;
        self.pc = (h as u16) << 8 | l as u16;
        self.pc = u16::wrapping_sub(self.pc, 1);
        self.wait = 16;
    }

    pub fn tick(&mut self, bus: &mut Bus) {
        self.wait -= 1;
        if self.wait > 0 {
            return;
        }

        if self.interrupts_enabled && (bus.enabled_interrupts & bus.requested_interrupts != 0){
            let active_interrupts = bus.enabled_interrupts & bus.requested_interrupts;
            self.sp -= 2;
            self.interrupts_enabled = false;
            bus.write16(self.sp, self.pc);
            self.wait = 20;
            if active_interrupts & VBLANK != 0 {
                self.pc = 0x40;
                bus.requested_interrupts &= !VBLANK;
            } else if active_interrupts & LCD_STAT != 0{
                self.pc = 0x48;
                bus.requested_interrupts &= !LCD_STAT;
            } else if active_interrupts & TIMER != 0{
                self.pc = 0x50;
                bus.requested_interrupts &= !TIMER;
            } else if active_interrupts & SERIAL != 0 {
                self.pc = 0x58;
                bus.requested_interrupts &= !SERIAL;
            } else if active_interrupts & JOYPAD != 0 {
                self.pc = 0x60;
                bus.requested_interrupts &= !JOYPAD;
            }
            return;
        } else {
            bus.requested_interrupts = 0;
        }

        macro_rules! disasm {
            ($($arg:tt)+) => (
                if cfg!(debug_assertions) {
                    print!("0x{:<8x}: ", self.pc);
                    println!($($arg)+);
                }
            )
        }

        macro_rules! disasm_pc {
            ($pc: expr, $($arg:tt)+) => (
                if cfg!(debug_assertions) {
                    print!("0x{:<8x}: ", $pc);
                    println!($($arg)+);
                }
            )
        }

        macro_rules! inc {
            ($arg:ident) => ({
                let before = self.$arg;
                self.$arg = u8::wrapping_add(self.$arg, 1);
                self.zerof = if self.$arg == 0 {1} else {0};
                self.add_subf = 0;
                self.half_carryf = if ((before & 0xf) + 1) & 0x10 != 0 {1} else {0};
                self.wait = 4;
                disasm!("Inc {}", stringify!($arg));
            });
        }

        macro_rules! dec {
            ($arg:ident) => ({
                let before = self.$arg;
                self.$arg = u8::wrapping_sub(self.$arg, 1);
                self.zerof = if self.$arg == 0 {1} else {0};
                self.add_subf = 1;
                self.half_carryf = if (before & 0xf) == 0 {1} else {0};
                self.wait = 4;
                disasm!("Dec {}", stringify!($arg));
            });
        }

        macro_rules! addA {
            ($arg:ident) => ({
                let before = self.a;
                let (new_a, carry) = u8::overflowing_add(self.$arg, self.a);
                self.zerof = if new_a == 0 {1} else {0};
                self.add_subf = 0;
                self.half_carryf = if ((before & 0xf) + self.a & 0xf) & 0x10 != 0 {1} else {0};
                self.carryf = if carry {1} else {0};
                self.a = new_a;
                self.wait = 4;
                disasm!("Add a, {}={:#x}", stringify!($arg), self.$arg);
            });
        }

        macro_rules! addHL {
            ($arg:ident) => ({
                let hl = (self.h as u16) << 8 | self.l as u16;
                let (res, carry) = u16::overflowing_add(hl, $arg);
                self.add_subf = 0;
                self.carryf = carry as u8;
                self.half_carryf = if ((hl >> 8 & 0xf) + ($arg >> 8 & 0xf)) & 0x10 != 0 {1} else {0};
                self.h = (res >> 8) as u8;
                self.l = res as u8;
                self.wait = 8;
                disasm!("Add HL:{:#x}, {}:{:#x} => {:#x}", hl, stringify!($arg), $arg, res);
            });
        }

        macro_rules! push16 {
            ($arg:ident) => {
                self.sp -= 2;
                bus.write16(self.sp, $arg);
                self.wait = 16;
                disasm!("PUSH {}", stringify!($arg))
            };
        }

        macro_rules! pop16 {
            ($h:ident, $l:ident) => {
                *$l = bus.read(self.sp);
                self.sp += 1;
                *$h = bus.read(self.sp);
                self.sp += 1;
                self.wait = 12;
                disasm!("POP {}{}:0x{:x}{:x}", stringify!($h), stringify!($l), $h, $l);
            };
        }

        macro_rules! subA {
            ($arg:ident) => ({
                let before = self.$arg;
                let (new_a, carry) = u8::overflowing_sub(self.a, self.$arg);
                self.zerof = if new_a == 0 {1} else {0};
                self.add_subf = 1;
                self.half_carryf = if self.a & 0xf > (before & 0xf) {0} else {1};
                self.carryf = if carry {1} else {0};
                self.a = new_a;
                self.wait = 4;
                disasm!("Sub a, {}", stringify!($arg));
            });
        }

        macro_rules! adcA {
            ($arg:ident) => ({
                let before = self.a;
                let (new_a, carry) = u8::overflowing_add(self.$arg, self.a);
                let (new_a2, carry2) = u8::overflowing_add(new_a, self.carryf);
                self.zerof = if new_a2 == 0 {1} else {0};
                self.add_subf = 0;
                self.half_carryf = if ((before & 0xf) + self.a & 0xf + self.carryf) & 0x10 != 0 {1} else {0};
                self.carryf = if carry || carry2 {1} else {0};
                self.a = new_a2;
                self.wait = 4;
                disasm!("Adc a, {}", stringify!($arg));
            });
        }

        macro_rules! sbcA {
            ($arg:ident) => ({
                let before = self.$arg;
                let (new_a, carry) = u8::overflowing_sub(self.a, self.$arg);
                let (new_a2, carry2) = u8::overflowing_sub(new_a, self.carryf);
                self.zerof = if new_a2 == 0 {1} else {0};
                self.add_subf = 1;
                self.half_carryf = if self.a & 0xf < (before & 0xf) + self.carryf {1} else {0};
                self.carryf = if carry || carry2 {1} else {0};
                self.a = new_a;
                self.wait = 4;
                disasm!("SBC a, {}", stringify!($arg));
            });
        }

        macro_rules! andA {
            ($arg:expr) => ({
                self.a &= $arg;
                self.zerof = if self.a != 0 {0} else {1};
                self.add_subf = 0;
                self.half_carryf = 1;
                self.carryf = 0;
                self.wait = 4;
                disasm!("And A, {}", stringify!($arg));
            });
        }

        macro_rules! orA {
            ($arg:expr) => ({
                self.a |= $arg;
                self.zerof = if self.a == 0 {1} else {0};
                self.add_subf = 0;
                self.half_carryf = 0;
                self.carryf = 0;
                self.wait = 4;
                disasm!("Or A, {}", stringify!($arg));
            });
        }

        macro_rules! xorA {
            ($arg:expr) => ({
                self.a ^= $arg;
                self.zerof = if self.a == 0 {1} else {0};
                self.add_subf = 0;
                self.half_carryf = 0;
                self.carryf = 0;
                self.wait = 4;
                disasm!("Xor A, {}", stringify!($arg));
            });
        }

        macro_rules! cmpA {
            ($arg:expr) => ({
                let (new_a, carry) = u8::overflowing_sub(self.a, $arg);
                self.zerof = if new_a == 0 {1} else {0};
                self.add_subf = 1;
                self.half_carryf = if $arg & 0xf > (self.a & 0xf) {0} else {1};
                self.carryf = if carry {1} else {0};
                self.wait = 4;
                disasm!("Cmp A, {}:{:#x}", stringify!($arg), $arg);
            });
        }

        fn jump (this: &mut Cpu, bus: &mut Bus) -> u16 {
            let addr = bus.read16(this.pc + 1);
            this.pc = addr;
            this.wait = 16;
            this.pc = u16::wrapping_sub(this.pc, 1);
            addr
        };

        fn jrel (this: &mut Cpu, bus: &mut Bus) -> u8 {
            let addr = bus.read(this.pc+1);
            this.pc = u16::wrapping_add(this.pc, addr as i8 as u16);
            this.wait = 12;
            this.pc = u16::wrapping_add(this.pc, 1);
            addr
        };

        if self.pc == 0x0395 {
            //eprintln!("Breakpoint !");
        }

        let op = bus.read(self.pc);
        match op {
            0x0 => {
                self.wait = 4;
                disasm!("NOP");
            }
            0x1 => {
                self.c = bus.read(self.pc+1);
                self.b = bus.read(self.pc+2);
                disasm!("LD BC, 0x{:x}{:x}", self.b, self.c);
                self.wait = 12;
                self.pc += 2;
            }
            0x11 => {
                self.e = bus.read(self.pc+1);
                self.d = bus.read(self.pc+2);
                disasm!("LD DE, 0x{:x}{:x}", self.d, self.e);
                self.wait = 12;
                self.pc += 2;
            }
            0x21 => {
                self.l = bus.read(self.pc+1);
                self.h = bus.read(self.pc+2);
                disasm!("LD HL, 0x{:x}{:x}", self.h, self.l);
                self.wait = 12;
                self.pc += 2;
            }
            0x31 => {
                self.sp = bus.read(self.pc+1) as u16 | (bus.read(self.pc+2) as u16) << 8;
                disasm!("LD HL, 0x{:x}", self.sp);
                self.wait = 12;
                self.pc += 2;
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
                self.b = bus.read((self.h as u16) << 8 | self.l as u16);
                self.wait = 8;
                disasm!("LD B, (HL)");
            }
            0x56 => {
                self.d = bus.read((self.h as u16) << 8 | self.l as u16);
                self.wait = 8;
                disasm!("LD D, (HL):{:#x}", self.d);
            }
            0x66 => {
                self.h = bus.read((self.h as u16) << 8 | self.l as u16);
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
                self.c = bus.read((self.h as u16) << 8 | self.l as u16);
                self.wait = 8;
                disasm!("LD C, (HL)");
            }
            0x5e => {
                self.e = bus.read((self.h as u16) << 8 | self.l as u16);
                self.wait = 8;
                disasm!("LD E, (HL):{:#x}", self.e);
            }
            0x6e => {
                self.l = bus.read((self.h as u16) << 8 | self.l as u16);
                self.wait = 8;
                disasm!("LD L, (HL)");
            }
            0x7e => {
                self.a = bus.read((self.h as u16) << 8 | self.l as u16);         
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
                bus.write((self.h as u16) << 8 | self.l as u16, self.b);
                self.wait = 8;
                disasm!("LD (HL), B");
            }
            0x71 => {
                bus.write((self.h as u16) << 8 | self.l as u16, self.c);
                self.wait = 8;
                disasm!("LD (HL), C");
            }
            0x72 => {
                bus.write((self.h as u16) << 8 | self.l as u16, self.d);
                self.wait = 8;
                disasm!("LD (HL), D");
            }
            0x73 => {
                bus.write((self.h as u16) << 8 | self.l as u16, self.e);
                self.wait = 8;
                disasm!("LD (HL), E");
            }
            0x74 => {
                bus.write((self.h as u16) << 8 | self.l as u16, self.h);
                self.wait = 8;
                disasm!("LD (HL), H");
            }
            0x75 => {
                bus.write((self.h as u16) << 8 | self.l as u16, self.l);
                self.wait = 8;
                disasm!("LD (HL), L");
            }
            0x77 => {
                bus.write((self.h as u16) << 8 | self.l as u16, self.a);
                self.wait = 8;
                disasm!("LD (HL), A");
            }
            0x02 => {
                bus.write((self.b as u16) << 8 | self.c as u16, self.a);
                self.wait = 8;
                disasm!("LD (BC), A");
            }
            0x12 => {
                bus.write((self.d as u16) << 8 | self.e as u16, self.a);
                self.wait = 8;
                disasm!("LD (DE), A");
            }
            0x22 => {
                let mut hl = (self.h as u16) << 8 | self.l as u16;
                bus.write(hl, self.a);
                hl = u16::wrapping_add(hl, 1);
                self.h = (hl >> 8) as u8;
                self.l = (hl & 0xff) as u8;
                self.wait = 8;
                disasm!("LD (HL+), A");
            }
            0x32 => {
                let mut hl = (self.h as u16) << 8 | self.l as u16;
                bus.write(hl, self.a);
                hl = u16::wrapping_sub(hl, 1);
                self.h = (hl >> 8) as u8;
                self.l = (hl & 0xff) as u8;
                self.wait = 8;
                disasm!("LD (HL-), A");
            }
            0x06 => {
                self.b = bus.read(self.pc + 1);
                disasm!("LD B, d8:{:#x}", self.b);
                self.wait = 8;
                self.pc += 1;
            }
            0x16 => {
                self.d = bus.read(self.pc + 1);
                disasm!("LD D, d8");
                self.wait = 8;
                self.pc += 1;
            }
            0x26 => {
                self.h = bus.read(self.pc + 1);
                disasm!("LD H, d8");
                self.wait = 8;
                self.pc += 1;
            }
            0x36 => {
                let arg = bus.read(self.pc + 1);
                bus.write((self.h as u16) << 8 | self.l as u16, arg);
                disasm!("LD (HL), d8");
                self.wait = 12;
                self.pc += 1;
            }
            0x0a => {
                self.a = bus.read((self.b as u16) << 8 | self.c as u16);
                self.wait = 8;
                disasm!("LD A, (BC)");
            }
            0x1a => {
                self.a = bus.read((self.d as u16) << 8 | self.e as u16);
                self.wait = 8;
                disasm!("LD A, (DE)");
            }
            0x2a => {
                let mut hl = (self.h as u16) << 8 | self.l as u16;
                self.a = bus.read(hl);
                hl = u16::wrapping_add(hl, 1);
                self.h = (hl >> 8) as u8;
                self.l = (hl & 0xff) as u8;
                self.wait = 8;
                disasm!("LD A, (HL+)");
            }
            0x3a => {
                let mut hl = (self.h as u16) << 8 | self.l as u16;
                self.a = bus.read(hl);
                hl = u16::wrapping_sub(hl, 1);
                self.h = (hl >> 8) as u8;
                self.l = (hl & 0xff) as u8;
                self.wait = 8;
                disasm!("LD A, (HL-)");
            }
            0x0e => {
                self.c = bus.read(self.pc + 1);
                disasm!("LD C, d8");
                self.wait = 8;
                self.pc += 1;
            }
            0x1e => {
                self.e = bus.read(self.pc + 1);
                disasm!("LD E, d8");
                self.wait = 8;
                self.pc += 1;
            }
            0x2e => {
                self.l = bus.read(self.pc + 1);
                disasm!("LD L, d8");
                self.wait = 8;
                self.pc += 1;
            }
            0x3e => {
                self.a = bus.read(self.pc + 1);
                disasm!("LD A, d8:{:#x}", self.a);
                self.wait = 8;
                self.pc += 1;
            }
            0x08 => {
                let low = bus.read(self.pc + 1);
                let high = bus.read(self.pc + 2);
                bus.write16((high as u16) << 8 | low as u16, self.sp);
                disasm!("LD (a16), SP");
                self.wait = 20;
                self.pc += 2;
            }
            0xe0 => {
                let a8 = bus.read(self.pc + 1);
                bus.write( a8 as u16 | 0xFF00, self.a);
                disasm!("LDH (a8):0xff{:02x}, A", a8);
                self.wait = 12;
                self.pc += 1;
            }
            0xf0 => {
                let a8 = bus.read(self.pc + 1);
                self.a = bus.read( a8 as u16 | 0xFF00);
                disasm!("LDH A, (a8):{:#x}", a8);
                self.wait = 12;
                self.pc += 1;
            }
            0xe2 => {
                bus.write( self.c as u16 | 0xFF00, self.a);
                disasm!("LDH (C), A");
                self.wait = 8;
            }
            0xf2 => {
                self.a = bus.read( self.c as u16 | 0xFF00);
                self.wait = 8;
                disasm!("LDH A, (C)");
            }
            0xf8 => {
                let r8 = bus.read(self.pc + 1);
                let addr = u16::wrapping_add(r8 as i8 as i16 as u16, self.sp);
                self.a = bus.read(addr);
                let (_, overflow) = u8::overflowing_add(r8, (self.sp & 0xff) as u8);
                self.carryf = if overflow {1} else {0};
                self.half_carryf = if ((r8 as u16) & 0xF + (self.sp & 0xF)) & 0x10 != 0 {1} else {0};
                self.zerof = 0;
                self.add_subf = 0;
                disasm!("LDH A, (a8)");
                self.wait = 12;
                self.pc += 1;
            }
            0xf9 => {
                self.sp = (self.h as u16) << 8 | self.l as u16;
                self.wait = 8;
                disasm!("LD SP, HL");
            }
            0xea => {
                let addr = bus.read16(self.pc + 1);
                bus.write(addr, self.a);
                self.pc += 2;
                disasm!("LD ({:#x}), A", addr);
            }
            0xfa => {
                let addr = bus.read16(self.pc + 1);
                self.a = bus.read(addr);
                self.pc += 2;
                disasm!("LD A, ({:#x})", addr);
            }
            0x04 => inc!(b),
            0x14 => inc!(d),
            0x24 => inc!(h),
            0x0c => inc!(c),
            0x1c => inc!(e),
            0x2c => inc!(l),
            0x3c => inc!(a),
            0x05 => dec!(b),
            0x15 => dec!(d),
            0x25 => dec!(h),
            0x0d => dec!(c),
            0x1d => dec!(e),
            0x2d => dec!(l),
            0x3d => dec!(a),
            0x34 => {
                let hl = (self.h as u16) << 8 | self.l as u16;
                let x = bus.read(hl);
                let res = u8::wrapping_add(x, 1);
                self.zerof = if res == 0 {1} else {0};
                self.add_subf = 0;
                self.half_carryf = if ((x & 0xf) + 1) & 0x10 != 0 {1} else {0};
                bus.write(hl, res);
                disasm!("INC (HL)");
                self.wait = 12;
            }
            0x35 => {
                let hl = (self.h as u16) << 8 | self.l as u16;
                let x = bus.read(hl);
                let res = u8::wrapping_sub(x, 1);
                self.zerof = if res == 0 {1} else {0};
                self.add_subf = 1;
                self.half_carryf = if (x & 0xf) == 0 {1} else {0};
                bus.write(hl, res);
                disasm!("DEC (HL)");
                self.wait = 12;
            }
            0x03 => {
                let mut bc = (self.b as u16) << 8 | self.c as u16;
                bc = u16::wrapping_add(bc, 1);
                self.c = (bc & 0xff) as u8;
                self.b = (bc >> 8) as u8;
                disasm!("INC BC");
                self.wait = 8;
            }
            0x13 => {
                let mut de = (self.d as u16) << 8 | self.e as u16;
                de = u16::wrapping_add(de, 1);
                self.e = (de & 0xff) as u8;
                self.d = (de >> 8) as u8;
                disasm!("INC DE");
                self.wait = 8;
            }
            0x23 => {
                let mut hl = (self.h as u16) << 8 | self.l as u16;
                hl = u16::wrapping_add(hl, 1);
                self.l = (hl & 0xff) as u8;
                self.h = (hl >> 8) as u8;
                disasm!("INC HL");
                self.wait = 8;
            }
            0x33 => {
                self.sp = u16::wrapping_add(self.sp, 1);
                disasm!("INC SP");
                self.wait = 8;
            }
            0x0b => {
                let mut bc = (self.b as u16) << 8 | self.c as u16;
                bc = u16::wrapping_sub(bc, 1);
                self.c = (bc & 0xff) as u8;
                self.b = (bc >> 8) as u8;
                disasm!("DEC BC");
                self.wait = 8;
            }
            0x1b => {
                let mut de = (self.d as u16) << 8 | self.e as u16;
                de = u16::wrapping_sub(de, 1);
                self.e = (de & 0xff) as u8;
                self.d = (de >> 8) as u8;
                disasm!("DEC DE");
                self.wait = 8;
            }
            0x2b => {
                let mut hl = (self.h as u16) << 8 | self.l as u16;
                hl = u16::wrapping_sub(hl, 1);
                self.l = (hl & 0xff) as u8;
                self.h = (hl >> 8) as u8;
                disasm!("DEC HL");
                self.wait = 8;
            }
            0x3b => {
                self.sp = u16::wrapping_sub(self.sp, 1);
                disasm!("DEC SP");
                self.wait = 8;
            }
            0x80 => addA!(b),
            0x81 => addA!(c),
            0x82 => addA!(d),
            0x83 => addA!(e),
            0x84 => addA!(h),
            0x85 => addA!(l),
            0x87 => addA!(a),
            0x90 => subA!(b),
            0x91 => subA!(c),
            0x92 => subA!(d),
            0x93 => subA!(e),
            0x94 => subA!(h),
            0x95 => subA!(l),
            0x97 => subA!(a),
            0x88 => adcA!(b),
            0x89 => adcA!(c),
            0x8a => adcA!(d),
            0x8b => adcA!(e),
            0x8c => adcA!(h),
            0x8d => adcA!(l),
            0x8f => adcA!(a),
            0x98 => sbcA!(b),
            0x99 => sbcA!(c),
            0x9a => sbcA!(d),
            0x9b => sbcA!(e),
            0x9c => sbcA!(h),
            0x9d => sbcA!(l),
            0x9f => sbcA!(a),
            0x86 => {
                let val = bus.read((self.h as u16) << 8 | self.l as u16);
                let (new_a, carry) = u8::overflowing_add(val, self.a);
                self.zerof = if new_a == 0 {1} else {0};
                self.add_subf = 0;
                self.half_carryf = if ((self.a & 0xf) + val & 0xf) & 0x10 != 0 {1} else {0};
                self.carryf = if carry {1} else {0};
                self.a = new_a;
                self.wait = 8;
                disasm!("Add a (HL)");
            }
            0x96 => {
                let val = bus.read((self.h as u16) << 8 | self.l as u16);
                let (new_a, carry) = u8::overflowing_sub(self.a, val);
                self.zerof = if new_a == 0 {1} else {0};
                self.add_subf = 0;
                self.half_carryf = if ((self.a & 0xf) - val & 0xf) & 0x10 != 0 {1} else {0};
                self.carryf = if carry {1} else {0};
                self.a = new_a;
                self.wait = 8;
                disasm!("Sub a (HL)");
            }
            0x8e => {
                let val = bus.read((self.h as u16) << 8 | self.l as u16);
                let (new_a, carry) = u8::overflowing_add(self.a, val);
                let (new_a2, carry2) = u8::overflowing_add(new_a, self.carryf);
                self.zerof = if new_a2 == 0 {1} else {0};
                self.add_subf = 0;
                self.half_carryf = if ((self.a & 0xf) + self.a & 0xf + self.carryf) & 0x10 != 0 {1} else {0};
                self.carryf = if carry || carry2 {1} else {0};
                self.a = new_a2;
                self.wait = 8;
                disasm!("Adc a, (HL)");
            }
            0x9e => {
                let val = bus.read((self.h as u16) << 8 | self.l as u16);
                let (new_a, carry) = u8::overflowing_sub(self.a, val);
                let (new_a2, carry2) = u8::overflowing_sub(new_a, self.carryf);
                self.zerof = if new_a2 == 0 {1} else {0};
                self.add_subf = 1;
                self.half_carryf = if (self.a & 0xf - (val & 0xf) - self.carryf) & 0x10 != 0 {1} else {0};
                self.carryf = if carry || carry2 {1} else {0};
                self.a = new_a;
                self.wait = 8;
                disasm!("SBC a, (HL)");
            }
            0xa0 => andA!(self.b),
            0xa1 => andA!(self.c),
            0xa2 => andA!(self.d),
            0xa3 => andA!(self.e),
            0xa4 => andA!(self.h),
            0xa5 => andA!(self.l),
            0xa7 => andA!(self.a),
            0xb0 => orA!(self.b),
            0xb1 => orA!(self.c),
            0xb2 => orA!(self.d),
            0xb3 => orA!(self.e),
            0xb4 => orA!(self.h),
            0xb5 => orA!(self.l),
            0xb7 => orA!(self.a),
            0xa6 => {
                let hl = bus.read((self.h as u16) << 8 | self.l as u16);
                andA!(hl);
                self.wait = 8;
            }
            0xb6 => {
                let hl = bus.read((self.h as u16) << 8 | self.l as u16);
                orA!(hl);
                self.wait = 8;
            }
            0xa8 => xorA!(self.b),
            0xa9 => xorA!(self.c),
            0xaa => xorA!(self.d),
            0xab => xorA!(self.e),
            0xac => xorA!(self.h),
            0xad => xorA!(self.l),
            0xaf => xorA!(self.a),
            0xb8 => cmpA!(self.b),
            0xb9 => cmpA!(self.c),
            0xba => cmpA!(self.d),
            0xbb => cmpA!(self.e),
            0xbc => cmpA!(self.h),
            0xbd => cmpA!(self.l),
            0xbf => cmpA!(self.a),
            0xae => {
                let hl = bus.read((self.h as u16) << 8 | self.l as u16);
                xorA!(hl);
                self.wait = 8;
            }
            0xbe => {
                let hl = bus.read((self.h as u16) << 8 | self.l as u16);
                cmpA!(hl);
                self.wait = 8;
            }
            0xc6 => {
                let arg = bus.read(self.pc +1 );
                let (new_a, carry) = u8::overflowing_add(arg, self.a);
                self.zerof = if new_a == 0 {1} else {0};
                self.add_subf = 0;
                self.half_carryf = if ((arg & 0xf) + self.a & 0xf) & 0x10 != 0 {1} else {0};
                self.carryf = if carry {1} else {0};
                self.a = new_a;
                self.wait = 8;
                self.pc += 1;
                disasm!("Add a, {:#x}", arg);
            }
            0xd6 => {
                let arg = bus.read(self.pc +1 );
                let (new_a, carry) = u8::overflowing_sub(self.a, arg);
                self.zerof = if new_a == 0 {1} else {0};
                self.add_subf = 1;
                self.half_carryf = if self.a & 0xf > (arg & 0xf) {0} else {1};
                self.carryf = if carry {1} else {0};
                self.a = new_a;
                self.wait = 8;
                self.pc += 1;
                disasm!("Sub a, {:#}", arg);
            },
            0xce => {
                let arg = bus.read(self.pc +1 );
                let (new_a, carry) = u8::overflowing_add(arg, self.a);
                let (new_a2, carry2) = u8::overflowing_add(new_a, self.carryf);
                self.zerof = if new_a2 == 0 {1} else {0};
                self.add_subf = 0;
                self.half_carryf = if ((arg & 0xf) + (self.a & 0xf) + self.carryf) & 0x10 != 0 {1} else {0};
                self.carryf = if carry || carry2 {1} else {0};
                self.a = new_a2;
                self.wait = 8;
                self.pc += 1;
                disasm!("Adc a, {:#x}", arg);
            },
            0xde => {
                let arg = bus.read(self.pc +1 );
                let (new_a, carry) = u8::overflowing_sub(self.a, arg);
                let (new_a2, carry2) = u8::overflowing_sub(new_a, self.carryf);
                self.zerof = if new_a2 == 0 {1} else {0};
                self.add_subf = 1;
                self.half_carryf = ((self.a & 0xf) + self.carryf < (arg & 0xf)) as u8;
                self.carryf = if carry || carry2 {1} else {0};
                self.a = new_a;
                self.wait = 8;
                self.pc += 1;
                disasm!("Sbc a, {:#}", arg);
            },
            0xe6 => {
                let arg8 = bus.read(self.pc +1 );
                andA!(arg8);
                self.pc += 1;
                self.wait = 8;
            },
            0xf6 => {
                let arg8 = bus.read(self.pc +1 );
                orA!(arg8);
                self.pc += 1;
                self.wait = 8;
            },
            0xee => {
                let arg8 = bus.read(self.pc +1 );
                xorA!(arg8);
                self.pc += 1;
                self.wait = 8;
            },
            0xfe => {
                let arg8 = bus.read(self.pc +1 );
                cmpA!(arg8);
                self.pc += 1;
                self.wait = 8;
            },
            0x09 => {
                let bc = ((self.b as u16) << 8) | self.c as u16;
                addHL!(bc);
            },
            0x19 => {
                let de = ((self.d as u16) << 8) | self.e as u16;
                addHL!(de);
            },
            0x29 => {
                let hl = ((self.h as u16) << 8) | self.l as u16;
                addHL!(hl);
            },
            0x39 => {
                let sp = self.sp;
                addHL!(sp);
            },
            0xc5 => {
                let bc = ((self.b as u16) << 8) | self.c as u16;
                push16!(bc);
            },
            0xd5 => {
                let de = ((self.d as u16) << 8) | self.e as u16;
                push16!(de);
            },
            0xe5 => {
                let hl = ((self.h as u16) << 8) | self.l as u16;
                push16!(hl);
            },
            0xf5 => {
                let af = ((self.a as u16) << 8) | self.flags() as u16;
                push16!(af);
            },
            0xc1 => {
                let b = &mut self.b;
                let c = &mut self.c;
                pop16!(b, c);
            },
            0xd1 => {
                let d = &mut self.d;
                let e = &mut self.e;
                pop16!(d, e);
            },
            0xe1 => {
                let h = &mut self.h;
                let l = &mut self.l;
                pop16!(h, l);
            },
            0xf1 => {
                let mut flags = self.flags();
                {
                    let f = &mut flags;
                    let a = &mut self.a;
                    pop16!(a, f);
                }
                self.set_flags(flags);
            },
            0xc3 => {
                let pc = self.pc;
                let addr = jump(self, bus);
                disasm_pc!(pc, "JP {:#x}", addr);
            },
            0xc2 => {
                if self.zerof != 0 {
                    self.wait = 12;
                    disasm!("JP NZ, <no_jump>");
                    self.pc += 2;
                }else{
                    let pc = self.pc;
                    let addr = jump(self, bus);
                    disasm_pc!(pc, "JP NZ, {:#x}", addr);
                }
            }
            0xd2 => {
                if self.carryf != 0 {
                    self.wait = 12;
                    disasm!("JP NC, <no_jump>");
                    self.pc += 2;
                }else{
                    let pc = self.pc;
                    let addr = jump(self, bus);
                    disasm_pc!(pc, "JP NC, {:#x}", addr);
                }
            }
            0xca => {
                if self.zerof == 0 {
                    self.wait = 12;
                    disasm!("JP Z, <no_jump>");
                    self.pc += 2;
                }else{
                    let pc = self.pc;
                    let addr = jump(self, bus);
                    disasm_pc!(pc, "JP Z, {:#x}", addr);
                }
            }
            0xda => {
                if self.carryf == 0 {
                    self.wait = 12;
                    disasm!("JP C, <no_jump>");
                    self.pc += 2;
                }else{
                    let pc = self.pc;
                    let addr = jump(self, bus);
                    disasm_pc!(pc, "JP C, {:#x}", addr);
                }
            }
            0xe9 => {
                let hl = ((self.h as u16) << 8) | self.l as u16;
                disasm!("JP HL:{:#x}", hl);
                self.wait = 4;
                self.pc = u16::wrapping_sub(hl, 1);
            }
            0x18 => {
                let pc = self.pc;
                let addr = jrel(self, bus);
                disasm_pc!(pc, "JR {:#x}", addr);
            }
            0x20 => {
                if self.zerof == 0 {
                    let pc = self.pc;
                    let addr = jrel(self, bus);
                    disasm_pc!(pc, "JR NZ, {:#x}", addr);
                }else{
                    self.wait = 8;
                    disasm!("JR NZ, <no_jump>");
                    self.pc += 1;
                }
            }
            0x30 => {
                if self.carryf == 0 {
                    let pc = self.pc;
                    let addr = jrel(self, bus);
                    disasm_pc!(pc, "JR NC, {:#x}", addr);
                }else{
                    self.wait = 8;
                    disasm!("JR NC, <no_jump>");
                    self.pc += 1;
                }
            }
            0x28 => {
                if self.zerof != 0 {
                    let pc = self.pc;
                    let addr = jrel(self, bus);
                    disasm_pc!(pc, "JR Z, {:#x}", addr);
                }else{
                    self.wait = 8;
                    disasm!("JR Z, <no_jump>");
                    self.pc += 1;
                }
            }
            0x38 => {
                if self.carryf != 0 {
                    let pc = self.pc;
                    let addr = jrel(self, bus);
                    disasm_pc!(pc, "JR C, {:#x}", addr);
                }else{
                    self.wait = 8;
                    disasm!("JR C, <no_jump>");
                    self.pc += 1;
                }
            }
            0xc7 => {
                disasm!("RST 0x00");
                self.call(bus, 0x00);
            }
            0xd7 => {
                disasm!("RST 0x10");
                self.call(bus, 0x10);
            }
            0xe7 => {
                disasm!("RST 0x20");
                self.call(bus, 0x20);
            }
            0xf7 => {
                disasm!("RST 0x30");
                self.call(bus, 0x30);
            }
            0xcf => {
                disasm!("RST 0x08");
                self.call(bus, 0x08);
            }
            0xdf => {
                disasm!("RST 0x18");
                self.call(bus, 0x18);
            }
            0xef => {
                disasm!("RST 0x28");
                self.call(bus, 0x28);
            }
            0xff => {
                disasm!("RST 0x38");
                self.call(bus, 0x38);
            }
            0xcd => {
                let addr = bus.read16(self.pc + 1);
                disasm!("CALL {:#x}", addr);
                self.pc += 2;
                self.call(bus, addr);
                self.wait = 24;
            }
            0xc4 => {
                let addr = bus.read16(self.pc + 1);
                if self.zerof == 0 {
                    disasm!("CALL NZ, {:#x}", addr);
                    self.call(bus, addr);
                }else{
                    self.wait = 12;
                    disasm!("CALL NZ, <no_jump>");
                    self.pc += 2;
                }
            }
            0xd4 => {
                let addr = bus.read16(self.pc + 1);
                if self.carryf == 0 {
                    disasm!("CALL NC, {:#x}", addr);
                    self.call(bus, addr);
                    self.wait = 24;
                }else{
                    self.wait = 12;
                    disasm!("CALL NC, <no_jump>");
                    self.pc += 2;
                }
            }
            0xcc => {
                let addr = bus.read16(self.pc + 1);
                if self.zerof != 0 {
                    disasm!("CALL Z, {:#x}", addr);
                    self.call(bus, addr);
                    self.wait = 24;
                }else{
                    self.wait = 12;
                    disasm!("CALL Z, <no_jump>");
                    self.pc += 2;
                }
            }
            0xdc => {
                let addr = bus.read16(self.pc + 1);
                self.pc += 2;
                if self.carryf != 0 {
                    disasm!("CALL C, {:#x}", addr);
                    self.call(bus, addr);
                    self.wait = 24;
                }else{
                    self.wait = 12;
                    disasm!("CALL C, <no_jump>");
                    self.pc += 2;
                }
            }
            0xc9 => {
                disasm!("RET");
                self.ret(bus);
            }
            0xd9 => {
                disasm!("RETI");
                self.interrupts_enabled = true;
                self.ret(bus);
            }
            0xc8 => {
                if self.zerof != 0 {
                    disasm!("RET Z");
                    self.ret(bus);
                    self.wait = 20;
                }else{
                    disasm!("RET Z <no jmp>");
                    self.wait = 8;
                }
            }
            0xd8 => {
                if self.carryf == 0 {
                    disasm!("RET NC");
                    self.ret(bus);
                    self.wait = 20;
                }else{
                    disasm!("RET NC <no jmp>");
                    self.wait = 8;
                }
            }
            0xc0 => {
                if self.zerof == 0 {
                    disasm!("RET NZ");
                    self.ret(bus);
                    self.wait = 20;
                }else{
                    disasm!("RET NZ <no jmp>");
                    self.wait = 8;
                }
            }
            0xd0 => {
                if self.carryf != 0 {
                    disasm!("RET C");
                    self.ret(bus);
                    self.wait = 20;
                }else{
                    disasm!("RET C <no jmp>");
                    self.wait = 8;
                }
            }
            0xF3 => {
                self.wait = 4;
                self.interrupts_enabled = false;
                disasm!("DI");
            }
            0xfb => {
                self.wait = 4;
                self.interrupts_enabled = true;
                disasm!("EI");
            }
            0x2f => {
                self.wait = 4;
                self.half_carryf = 1;
                self.add_subf = 1;
                self.a = !self.a;
                disasm!("CPL");
            }
            0x3f => {
                self.wait = 4;
                self.half_carryf = 0;
                self.add_subf = 0;
                self.carryf = if self.carryf != 0 {0} else {1};
                disasm!("CCF");
            }
            0x37 => {
                self.wait = 4;
                self.half_carryf = 0;
                self.add_subf = 0;
                self.carryf = 1;
                disasm!("SCF");
            }
            0xcb => {
                self.pc += 1;
                self.cb_ext(bus);
            }
            _ => {
                eprintln!("Unknown opcode at 0x{:x} : 0x{:x}", self.pc, op);
            }
        };
        self.pc += 1;
    }

    fn cb_ext(&mut self, bus: &mut Bus) {
        macro_rules! disasm {
            ($($arg:tt)+) => (
                if cfg!(debug_assertions) {
                    print!("0x{:<8x}: ", self.pc - 1);
                    println!($($arg)+);
                }
            )
        }

        macro_rules! sub_match {
            ($opcode:ident, $operation:ident) => (
                match $opcode & 0x7 {
                    0x00 => {
                        self.wait = 8;
                        self.b = self.$operation(self.b, ($opcode & 0x38) >> 3);
                        disasm!("{} B, {}", stringify!($operation), ($opcode & 0x38) >> 3);
                    }
                    0x01 => {
                        self.wait = 8;
                        self.c = self.$operation(self.c, ($opcode & 0x38) >> 3);
                        disasm!("{} C, {}", stringify!($operation), ($opcode & 0x38) >> 3);
                    }
                    0x02 => {
                        self.wait = 8;
                        self.d = self.$operation(self.d, ($opcode & 0x38) >> 3);
                        disasm!("{} D, {}", stringify!($operation), ($opcode & 0x38) >> 3);
                    }
                    0x03 => {
                        self.wait = 8;
                        self.e = self.$operation(self.e, ($opcode & 0x38) >> 3);
                        disasm!("{} E, {}", stringify!($operation), ($opcode & 0x38) >> 3);
                    }
                    0x04 => {
                        self.wait = 8;
                        self.h = self.$operation(self.h, ($opcode & 0x38) >> 3);
                        disasm!("{} H, {}", stringify!($operation), ($opcode & 0x38) >> 3);
                    }
                    0x05 => {
                        self.wait = 8;
                        self.l = self.$operation(self.l, ($opcode & 0x38) >> 3);
                        disasm!("{} L, {}", stringify!($operation), ($opcode & 0x38) >> 3);
                    }
                    0x07 => {
                        self.wait = 8;
                        self.a = self.$operation(self.a, ($opcode & 0x38) >> 3);
                        disasm!("{} A, {}", stringify!($operation), ($opcode & 0x38) >> 3);
                    }
                    0x06 => {
                        self.wait = 16;
                        let hl = (self.h as u16) << 8 | self.l as u16;
                        bus.write(hl, self.$operation(bus.read(hl), ($opcode & 0x38) >> 3));
                        disasm!("{} (HL), {}", stringify!($operation), ($opcode & 0x38) >> 3);
                    }
                    _ => panic!("Invalid cb submatch {:#x}", $opcode)
                }
            )
        }

        let op = bus.read(self.pc);

        match op {
            0x00 => {
                self.wait = 8;
                self.b = self.rlc(self.b);
                disasm!("rlc B");
            }
            0x01 => {
                self.wait = 8;
                self.c = self.rlc(self.c);
                disasm!("rlc C");
            }
            0x02 => {
                self.wait = 8;
                self.d = self.rlc(self.d);
                disasm!("rlc D");
            }
            0x03 => {
                self.wait = 8;
                self.e = self.rlc(self.e);
                disasm!("rlc E");
            }
            0x04 => {
                self.wait = 8;
                self.h = self.rlc(self.h);
                disasm!("rlc H");
            }
            0x05 => {
                self.wait = 8;
                self.l = self.rlc(self.l);
                disasm!("rlc L");
            }
            0x07 => {
                self.wait = 8;
                self.a = self.rlc(self.a);
                disasm!("rlc A");
            }
            0x06 => {
                self.wait = 16;
                let hl = (self.h as u16) << 8 | self.l as u16;
                bus.write(hl, self.rlc(bus.read(hl)));
                disasm!("rlc (HL)");
            }
            0x08 => {
                self.wait = 8;
                self.b = self.rrc(self.b);
                disasm!("rrc B");
            }
            0x09 => {
                self.wait = 8;
                self.c = self.rrc(self.c);
                disasm!("rrc C");
            }
            0x0a => {
                self.wait = 8;
                self.d = self.rrc(self.d);
                disasm!("rrc D");
            }
            0x0b => {
                self.wait = 8;
                self.e = self.rrc(self.e);
                disasm!("rrc E");
            }
            0x0c => {
                self.wait = 8;
                self.h = self.rrc(self.h);
                disasm!("rrc H");
            }
            0x0d => {
                self.wait = 8;
                self.l = self.rrc(self.l);
                disasm!("rrc L");
            }
            0x0f => {
                self.wait = 8;
                self.a = self.rrc(self.a);
                disasm!("rrc A");
            }
            0x0e => {
                self.wait = 16;
                let hl = (self.h as u16) << 8 | self.l as u16;
                bus.write(hl, self.rrc(bus.read(hl)));
                disasm!("rrc (HL)");
            }
            0x10 => {
                self.wait = 8;
                self.b = self.rl(self.b);
                disasm!("rl B");
            }
            0x11 => {
                self.wait = 8;
                self.c = self.rl(self.c);
                disasm!("rl C");
            }
            0x12 => {
                self.wait = 8;
                self.d = self.rl(self.d);
                disasm!("rl D");
            }
            0x13 => {
                self.wait = 8;
                self.e = self.rl(self.e);
                disasm!("rl E");
            }
            0x14 => {
                self.wait = 8;
                self.h = self.rl(self.h);
                disasm!("rl H");
            }
            0x15 => {
                self.wait = 8;
                self.l = self.rl(self.l);
                disasm!("rl L");
            }
            0x17 => {
                self.wait = 8;
                self.a = self.rl(self.a);
                disasm!("rl A");
            }
            0x16 => {
                self.wait = 16;
                let hl = (self.h as u16) << 8 | self.l as u16;
                bus.write(hl, self.rl(bus.read(hl)));
                disasm!("rl (HL)");
            }
            0x18 => {
                self.wait = 8;
                self.b = self.rr(self.b);
                disasm!("rr B");
            }
            0x19 => {
                self.wait = 8;
                self.c = self.rr(self.c);
                disasm!("rr C");
            }
            0x1a => {
                self.wait = 8;
                self.d = self.rr(self.d);
                disasm!("rr D");
            }
            0x1b => {
                self.wait = 8;
                self.e = self.rr(self.e);
                disasm!("rr E");
            }
            0x1c => {
                self.wait = 8;
                self.h = self.rr(self.h);
                disasm!("rr H");
            }
            0x1d => {
                self.wait = 8;
                self.l = self.rr(self.l);
                disasm!("rr L");
            }
            0x1f => {
                self.wait = 8;
                self.a = self.rr(self.a);
                disasm!("rr A");
            }
            0x1e => {
                self.wait = 16;
                let hl = (self.h as u16) << 8 | self.l as u16;
                bus.write(hl, self.rr(bus.read(hl)));
                disasm!("rr (HL)");
            }
            0x20 => {
                self.wait = 8;
                self.b = self.sla(self.b);
                disasm!("SLA B");
            }
            0x21 => {
                self.wait = 8;
                self.c = self.sla(self.c);
                disasm!("SLA C");
            }
            0x22 => {
                self.wait = 8;
                self.d = self.sla(self.d);
                disasm!("SLA D");
            }
            0x23 => {
                self.wait = 8;
                self.e = self.sla(self.e);
                disasm!("SLA E");
            }
            0x24 => {
                self.wait = 8;
                self.h = self.sla(self.h);
                disasm!("SLA H");
            }
            0x25 => {
                self.wait = 8;
                self.l = self.sla(self.l);
                disasm!("SLA L");
            }
            0x27 => {
                self.wait = 8;
                self.a = self.sla(self.a);
                disasm!("SLA A");
            }
            0x26 => {
                self.wait = 16;
                let hl = (self.h as u16) << 8 | self.l as u16;
                bus.write(hl, self.sla(bus.read(hl)));
                disasm!("SLA (HL)");
            }
            0x28 => {
                self.wait = 8;
                self.b = self.sra(self.b);
                disasm!("sra B");
            }
            0x29 => {
                self.wait = 8;
                self.c = self.sra(self.c);
                disasm!("sra C");
            }
            0x2a => {
                self.wait = 8;
                self.d = self.sra(self.d);
                disasm!("sra D");
            }
            0x2b => {
                self.wait = 8;
                self.e = self.sra(self.e);
                disasm!("sra E");
            }
            0x2c => {
                self.wait = 8;
                self.h = self.sra(self.h);
                disasm!("sra H");
            }
            0x2d => {
                self.wait = 8;
                self.l = self.sra(self.l);
                disasm!("sra L");
            }
            0x2f => {
                self.wait = 8;
                self.a = self.sra(self.a);
                disasm!("sra A");
            }
            0x2e => {
                self.wait = 16;
                let hl = (self.h as u16) << 8 | self.l as u16;
                bus.write(hl, self.sra(bus.read(hl)));
                disasm!("sra (HL)");
            }
            0x30 => {
                self.wait = 8;
                self.b = self.swap(self.b);
                disasm!("swap B");
            }
            0x31 => {
                self.wait = 8;
                self.c = self.swap(self.c);
                disasm!("swap C");
            }
            0x32 => {
                self.wait = 8;
                self.d = self.swap(self.d);
                disasm!("swap D");
            }
            0x33 => {
                self.wait = 8;
                self.e = self.swap(self.e);
                disasm!("swap E");
            }
            0x34 => {
                self.wait = 8;
                self.h = self.swap(self.h);
                disasm!("swap H");
            }
            0x35 => {
                self.wait = 8;
                self.l = self.swap(self.l);
                disasm!("swap L");
            }
            0x37 => {
                self.wait = 8;
                self.a = self.swap(self.a);
                disasm!("swap A");
            }
            0x36 => {
                self.wait = 16;
                let hl = (self.h as u16) << 8 | self.l as u16;
                bus.write(hl, self.swap(bus.read(hl)));
                disasm!("swap (HL)");
            }
            0x38 => {
                self.wait = 8;
                self.b = self.srl(self.b);
                disasm!("srl B");
            }
            0x39 => {
                self.wait = 8;
                self.c = self.srl(self.c);
                disasm!("srl C");
            }
            0x3a => {
                self.wait = 8;
                self.d = self.srl(self.d);
                disasm!("srl D");
            }
            0x3b => {
                self.wait = 8;
                self.e = self.srl(self.e);
                disasm!("srl E");
            }
            0x3c => {
                self.wait = 8;
                self.h = self.srl(self.h);
                disasm!("srl H");
            }
            0x3d => {
                self.wait = 8;
                self.l = self.srl(self.l);
                disasm!("srl L");
            }
            0x3f => {
                self.wait = 8;
                self.a = self.srl(self.a);
                disasm!("srl A");
            }
            0x3e => {
                self.wait = 16;
                let hl = (self.h as u16) << 8 | self.l as u16;
                bus.write(hl, self.srl(bus.read(hl)));
                disasm!("srl (HL)");
            }
            x if x & 0xc0 == 0x80 & 0xc0 => {
                sub_match!(x, res);
            }
            x if x & 0xc0 == 0x40 & 0xc0 => {
                sub_match!(x, bit);
            }
            x if x & 0xc0 == 0xC0 & 0xc0 => {
                sub_match!(x, set);
            }
            _ => eprintln!("{:#x}: CB not supported : {:#x}", self.pc, op),
        }
    }

    fn sla(&mut self, val: u8) -> u8 {
        let res = (val as u16) << 1;
        self.carryf = (res >> 8 & 1) as u8;
        self.zerof = if res & 0xff != 0 {0} else {1};
        self.half_carryf = 0;
        self.add_subf = 0;
        res as u8
    }

    fn sra(&mut self, val: u8) -> u8 {
        let mut res = val  >> 1;
        res |= val & 0x80;
        self.carryf = (val & 1) as u8;
        self.zerof = if res & 0xff != 0 {0} else {1};
        self.half_carryf = 0;
        self.add_subf = 0;
        res as u8
    }

    fn rlc(&mut self, val: u8) -> u8 {
        let res = u8::rotate_left(val, 1);
        self.carryf = (res >> 7 & 1) as u8;
        self.zerof = if res != 0 {0} else {1};
        self.half_carryf = 0;
        self.add_subf = 0;
        res as u8
    }
    fn rrc(&mut self, val: u8) -> u8 {
        let res = u8::rotate_right(val, 1);
        self.carryf = (res & 1) as u8;
        self.zerof = if res != 0 {0} else {1};
        self.half_carryf = 0;
        self.add_subf = 0;
        res as u8
    }
    fn rl(&mut self, val: u8) -> u8 {
        let mut res = (val as u16) << 1;
        res |= self.carryf as u16;
        self.carryf = (res >> 8 & 1) as u8;
        self.zerof = if res != 0 {0} else {1};
        self.half_carryf = 0;
        self.add_subf = 0;
        res as u8
    }
    fn rr(&mut self, val: u8) -> u8 {
        let mut res = (val as u16) << 7;
        res |= (self.carryf as u16) << 15;
        res >>= 8;
        self.carryf = (val & 1) as u8;
        self.zerof = if res != 0 {0} else {1};
        self.half_carryf = 0;
        self.add_subf = 0;
        res as u8
    }

    fn swap(&mut self, val: u8) -> u8 {
        let h = val >> 4;
        let l = val & 0xf;
        let res = l << 4 | h;
        self.carryf = 0;
        self.zerof = if res != 0 {0} else {1};
        self.half_carryf = 0;
        self.add_subf = 0;
        res as u8
    }

    fn srl(&mut self, val: u8) -> u8 {
        let res = val >> 1;
        self.carryf = (val & 1) as u8;
        self.zerof = if res & 0xff != 0 {0} else {1};
        self.half_carryf = 0;
        self.add_subf = 0;
        res as u8
    }

    fn res(&self, val: u8, index: u8) -> u8 {
        val & !(1 << index)
    }

    fn set(&self, val: u8, index: u8) -> u8 {
        val | (1 << index)
    }

    fn bit(&mut self, val: u8, index: u8) -> u8 {
        self.add_subf = 0;
        self.half_carryf = 1;
        val ^ (1 << index)
    }

    /*
    let mut hl = (self.h as u16) << 8 | self.l as u16;
   */

}

