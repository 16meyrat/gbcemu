use std::fs::File;
use std::io::prelude::*;
use std::str;
use num_enum::TryFromPrimitive;
use std::convert::TryFrom;

pub trait Cartridge {
    fn read(&self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, val: u8);
    fn write16(&mut self, addr: u16, value: u16);
}

pub fn load_rom(path: &str) -> Box<dyn Cartridge> {
    let mut rom = Rom::new(path);
    println!("Loading {} ...", path);
    let title = rom.get_title();
    println!("Title: {}", title);

    let mbc_type = rom.get_mbc_type();
    let res = match mbc_type {
        MbcType::NRom => {
            println!("ROM without MBC");
            Box::new(NRom::new(& mut rom))
        },
        _ => {
            panic!("Unsuported MBC");
        },
    };
    println!("Rom loaded !");
    res
}

struct NRom {
    banks: [u8; 0x8000],
}

impl Cartridge for NRom {
    fn read(&self, addr: u16) -> u8{
        0
    }
    fn write(&mut self, addr: u16, val: u8){

    }

    fn write16(&mut self, addr: u16, value: u16){

    }
}

impl NRom {
    pub fn new(rom: & mut Rom) -> Self {
        let rom_data = rom.read_range(0, 0x8000);
        let slice = &rom_data[..0x8000];
        let mut res = NRom{
            banks: [0; 0x8000],
        };
        res.banks.copy_from_slice(slice);
        res
    }
}

#[derive(TryFromPrimitive)]
#[repr(u8)]
enum MbcType {
    NRom = 0x0,
    Mbc1 = 0x1,
    Mbc1Ram = 0x2,
    Mbc1RamBattery = 0x3,
    Mbc2 = 0x5,
    Mbc2Battery = 0x6,
    RomRam = 0x8,
    RomRamBattery = 0x9,
    MMm01 = 0xb,
    Mmm01Ram = 0xc,
    Mmm1RamBattery = 0xd,
    Mbc3TimerBattery = 0xF,
    Mbc3TimerRamBattery = 0x10,
    Mbc3 = 0x11,
    Mbc3Ram = 0x12,
    Mbc3RamBattery = 0x13,
    Unknown
} 

struct Rom {
    data: Vec<u8>,
}

impl Rom {
    fn new(path: &str) -> Self {
        let mut file = File::open(path).expect("File not found");
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(& mut buf).expect("IO error");
        Rom {
            data: buf,
        }
    }

    fn get_title(&mut self) -> String {
        let str_arr = self.read_range(0x134, 0x143);
        let mut s = str::from_utf8(&str_arr).expect("Invalid UTF-8 in game title");
        s = s.trim_matches(char::from(0));
        s.to_owned()
    }

    fn get_mbc_type(&mut self) -> MbcType {
        let mbc_code = self.data[0x147];
        MbcType::try_from(mbc_code).unwrap_or_else(|_| panic!("Unsuported MBC: {:x}", mbc_code))
    }

    fn read_range(&self, begin: usize, end: usize) -> &[u8] {
        &self.data[begin..end]
    }
}
