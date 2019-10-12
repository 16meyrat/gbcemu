use std::fs::File;
use std::io::prelude::*;
use std::str;
use num_enum::TryFromPrimitive;
use std::convert::TryFrom;

pub trait Cartridge {
    fn read(&self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, val: u8);
}

pub fn load_rom(path: &str) -> Box<dyn Cartridge> {
    let mut rom = Rom::new(path);
    println!("Loading {} ...", path);
    let title = rom.get_title();
    println!("Title: {}", title);

    let gbc = rom.is_gbc();
    println!("Game Boy Color mode: {}", gbc);

    println!("External RAM size : {:#x}", rom.get_ram_size());

    println!("ROM size : {}KB", rom.get_rom_size());

    let mbc_type = rom.get_mbc_type();
    let res: Box<dyn Cartridge> = match mbc_type {
        MbcType::NRom => {
            println!("ROM without MBC");
            Box::new(NRom::new(& mut rom))
        },
        MbcType::Mbc1 | MbcType::Mbc1Ram | MbcType::Mbc1RamBattery => {
            println!("Mapper is MBC1");
            Box::new(MBC1::new(&mut rom))
        }
        _ => {
            panic!("Unsuported MBC : {:?}", mbc_type);
        },
    };
    println!("Rom loaded !");
    res
}

struct NRom {
    banks: [u8; 0x8000],
    ram: Vec<u8>,
}

impl Cartridge for NRom {
    fn read(&self, addr: u16) -> u8{
        match addr {
            x if x < 0x8000 => self.banks[addr as usize],
            x if x >= 0xa000 && x < 0xc000 => *self.ram.get(addr as usize - 0xa000).unwrap_or(&0),
            _ => panic!("Illegal cartridge read at {:#x}", addr),
        }
    }
    fn write(&mut self, addr: u16, val: u8){
        match addr {
            x if x >= 0xa000 && x < 0xc000  => {
                let index = addr as usize - 0xa000;
                if index < self.ram.len() {
                    self.ram[index] = val;
                }
            }
            _ => {}, // usually select MBC, but noop for NRom
        };
    }
}

impl NRom {
    pub fn new(rom: & mut Rom) -> Self {
        let rom_data = rom.read_range(0, 0x8000);
        let slice = &rom_data[..0x8000];
        let ram_size = rom.get_ram_size();
        let mut res = NRom{
            banks: [0; 0x8000],
            ram: vec![0; ram_size],
        };
        res.banks.copy_from_slice(slice);
        res
    }
}

struct MBC1 {
    banks: Vec<[u8; 0x4000]>,
    ram: Vec<[u8; 0x2000]>,
    lower_selection: u8,
    upper_selection: u8,
    rom_selected: bool,
}

impl MBC1 {
    pub fn new(rom: & mut Rom) -> Self {
        let bank_nb = rom.get_rom_size() / 16;
        let ram_size = rom.get_ram_size();
        let rom_data = rom.read_range(0, rom.get_data_len());

        let mut res = MBC1{
            banks: vec![[0; 0x4000]; bank_nb],
            ram: vec![[0; 0x2000]; ram_size],
            lower_selection: 1,
            upper_selection: 0,
            rom_selected: true,
        };
        let mut i = 0;
        for chunk in rom_data.chunks(0x4000) {
            res.banks[i][..chunk.len()].copy_from_slice(chunk);
            i += 1;
            if i & 0x1f == 0 {
                i += 1;
            }
        }
        res
    }

    fn get_rom_bank(&self) -> usize {
        let upper = if self.rom_selected {self.upper_selection} else {0};
        (self.lower_selection | upper << 4) as usize
    }

    fn get_ram_bank(&self) -> usize {
        if self.rom_selected {0} else {self.upper_selection as usize}
    }
}

impl Cartridge for MBC1 {
    fn read(&self, addr: u16) -> u8{
        match addr {
            x if x < 0x4000 => self.banks[0][addr as usize],
            x if x < 0x8000 => self.banks[self.get_rom_bank()][addr as usize - 0x4000],
            x if x >= 0xa000 && x < 0xc000 => *self.ram[self.get_ram_bank()].get(addr as usize - 0xa000).unwrap_or(&0),
            _ => panic!("Illegal cartridge read at {:#x}", addr),
        }
    }
    fn write(&mut self, addr: u16, val: u8){
        match addr {
            x if x >= 0xa000 && x < 0xc000  => {
                let index = addr as usize - 0xa000;
                let ram_bank = self.get_ram_bank();
                if index < self.ram[ram_bank].len() {
                    self.ram[ram_bank][index] = val;
                }
            }
            x if x < 0x2000 => {
                // ram enable / disable
            }
            x if x < 0x4000 => {
                let value = if val == 0 {1} else {val}; // "bug" of MBC1
                self.lower_selection = value & 0x1f;
            }
            x if x < 0x6000 => {
                self.upper_selection = val & 0x3;
            }
            x if x < 0x8000 => {
                self.rom_selected = val & 1 == 0;
            }
            _ => {
                panic!("Illegal cartridge write at {:#x}", addr)
            },
        };
    }
}

#[derive(TryFromPrimitive, Debug)]
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

    fn is_gbc(&self) -> bool {
        self.data[0x143] & 0x80 != 0
    }

    fn get_ram_size(&self) -> usize {
        match self.data[0x149] {
            0 => 0,
            1 => 0x800,
            2 => 0x2000,
            3 => 0x8000,
            4 => 0x20000,
            5 => 0x10000,
            x => panic!("Invalid RAM size: {}", x)
        }
    }

    fn get_rom_size(&self) -> usize {
        match self.data[0x148] {
            0 => 0x20,
            1 => 0x40,
            2 => 0x80,
            3 => 0x100,
            4 => 0x200,
            5 => 0x400,
            6 => 0x800,
            7 => 0x1000,
            8 => 0x2000,
            0x52 => 0x480,
            0x53 => 0x500,
            0x54 => 0x600,
            x => panic!("Invalid ROM size: {}", x)
        }
    }

    fn get_mbc_type(&mut self) -> MbcType {
        let mbc_code = self.data[0x147];
        MbcType::try_from(mbc_code).unwrap_or_else(|_| panic!("Unsuported MBC: {:x}", mbc_code))
    }

    fn get_data_len(&self) -> usize {
        self.data.len()
    }

    fn read_range(&self, begin: usize, end: usize) -> &[u8] {
        &self.data[begin..end]
    }
}
