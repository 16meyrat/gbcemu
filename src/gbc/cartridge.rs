use anyhow::{bail, Context, Result};
use num_enum::TryFromPrimitive;
use std::convert::TryFrom;
use std::fs::{File, OpenOptions};
use std::io::{prelude::*, SeekFrom};
use std::path::PathBuf;
use std::str;

pub trait Cartridge {
    fn read(&self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, val: u8);
}

pub fn load_rom(path: &str) -> Result<Box<dyn Cartridge>> {
    let mut rom = Rom::new(path)?;
    println!("Loading {} ...", path);
    let title = rom.get_title()?;
    println!("Title: {}", title);

    let gbc = rom.is_gbc();
    println!("Game Boy Color mode: {}", gbc);

    println!("External RAM size : {}KiB", rom.get_ram_size()? / 0x400);

    println!("ROM size : {}KiB", rom.get_rom_size()?);

    let mbc_type = rom.get_mbc_type()?;
    let res: Box<dyn Cartridge> = match mbc_type {
        MbcType::NRom => {
            println!("ROM without MBC");
            Box::new(NRom::new(&mut rom)?)
        }
        MbcType::Mbc1 | MbcType::Mbc1Ram | MbcType::Mbc1RamBattery => {
            println!("Mapper is MBC1");
            Box::new(MBC1::new(
                &mut rom,
                matches!(mbc_type, MbcType::Mbc1RamBattery),
            )?)
        }
        _ => {
            panic!("Unsuported MBC : {:?}", mbc_type);
        }
    };
    println!("Rom loaded !");
    Ok(res)
}

struct NRom {
    banks: [u8; 0x8000],
    ram: Vec<u8>,
}

impl Cartridge for NRom {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            x if x < 0x8000 => self.banks[addr as usize],
            x if (0xa000..0xc000).contains(&x) => {
                *self.ram.get(addr as usize - 0xa000).unwrap_or(&0)
            }
            _ => panic!("Illegal cartridge read at {:#x}", addr),
        }
    }
    fn write(&mut self, addr: u16, val: u8) {
        match addr {
            x if (0xa000..0xc000).contains(&x) => {
                let index = addr as usize - 0xa000;
                if index < self.ram.len() {
                    self.ram[index] = val;
                }
            }
            _ => {} // usually select MBC, but noop for NRom
        };
    }
}

impl NRom {
    pub fn new(rom: &mut Rom) -> Result<Self> {
        let rom_data = rom.read_range(0, 0x8000)?;
        let slice = &rom_data[..0x8000];
        let ram_size = rom.get_ram_size()?;
        let mut res = NRom {
            banks: [0; 0x8000],
            ram: vec![0; ram_size],
        };
        res.banks.copy_from_slice(slice);
        Ok(res)
    }
}
// TODO: support bundle cartriges
struct MBC1 {
    banks: Vec<[u8; 0x4000]>,
    ram: Vec<[u8; 0x2000]>,
    ram_file: Option<File>,
    ram_enable: bool,
    lower_selection: u8,
    upper_selection: u8,
    alt_bank_select: bool,
    upper_is_rom: bool,
}

impl MBC1 {
    pub fn new(rom: &mut Rom, battery: bool) -> Result<Self> {
        let bank_nb = rom.get_rom_size()? / 16;
        let ram_size = rom.get_ram_size()?;
        if ram_size != 0x0 && ram_size != 0x2000 && ram_size != 0x8000 {
            bail!("Invalid ram size for MBC1: {:x}", ram_size);
        }
        let ram_bank_nb = ram_size / 0x2000;
        let rom_data = rom.read_range(0, rom.get_data_len())?;
        let mut save_path = rom.path.clone();
        save_path.set_extension("save");
        let mut ram_file = if battery {
            Some(
                OpenOptions::new()
                    .write(true)
                    .read(true)
                    .create(true)
                    .open(&save_path)?,
            )
        } else {
            None
        };
        let mut ram = vec![[0u8; 0x2000]; ram_bank_nb];
        if let Some(ram_file) = &mut ram_file {
            let file_len = ram_file.seek(SeekFrom::End(0))?;
            ram_file.rewind()?;
            if file_len != 0 {
                for ram_bank in ram.iter_mut() {
                    ram_file.read_exact(ram_bank).with_context(|| {
                        format!("Save file {} is corrupted", save_path.display())
                    })?;
                }
                println!("Save file loaded from {}", save_path.display());
                ram_file.rewind()?;
            } else {
                println!("Save file created at {}", save_path.display());
            }
        }

        let mut res = MBC1 {
            banks: vec![[0; 0x4000]; bank_nb],
            ram,
            ram_file,
            ram_enable: false,
            lower_selection: 1,
            upper_selection: 0,
            alt_bank_select: true,
            upper_is_rom: bank_nb > 31,
        };
        let mut i = 0;
        for chunk in rom_data.chunks(0x4000) {
            res.banks[i][..chunk.len()].copy_from_slice(chunk);
            i += 1;
            if i & 0x1f == 0 {
                i += 1;
            }
        }
        Ok(res)
    }

    fn get_rom_bank(&self) -> usize {
        let upper = if !self.alt_bank_select && self.upper_is_rom {
            self.upper_selection
        } else {
            0
        };
        (self.lower_selection | upper << 5) as usize
    }

    fn get_ram_bank(&self) -> usize {
        if self.alt_bank_select || self.upper_is_rom {
            0
        } else {
            self.upper_selection as usize
        }
    }
}

impl Drop for MBC1 {
    fn drop(&mut self) {
        if let Err(e) = (|| -> Result<()> {
            if let Some(ram_file) = &mut self.ram_file {
                ram_file.rewind()?;
                for ram_bank in self.ram.iter_mut() {
                    ram_file.write_all(ram_bank)?;
                }
                println!("Game saved!");
            }
            Ok(())
        })() {
            eprintln!("Game save failed: {:#}", e);
        }
    }
}

impl Cartridge for MBC1 {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            x if x < 0x4000 => self.banks[0][addr as usize],
            x if x < 0x8000 => self.banks[self.get_rom_bank()][addr as usize - 0x4000],
            x if (0xa000..0xc000).contains(&x) => {
                if self.ram_enable {
                    *self.ram[self.get_ram_bank()]
                        .get(addr as usize - 0xa000)
                        .unwrap_or(&0)
                } else {
                    0
                }
            }
            _ => panic!("Illegal cartridge read at {:#x}", addr),
        }
    }
    fn write(&mut self, addr: u16, val: u8) {
        match addr {
            x if (0xa000..0xc000).contains(&x) => {
                if self.ram_enable {
                    let index = addr as usize - 0xa000;
                    let ram_bank = self.get_ram_bank();
                    if index < self.ram[ram_bank].len() {
                        self.ram[ram_bank][index] = val;
                    }
                }
            }
            x if x < 0x2000 => {
                self.ram_enable = val & 0xf == 0xa;
            }
            x if x < 0x4000 => {
                let value = if val & 0x1f == 0 { 1 } else { val & 0x1f }; // "bug" of MBC1
                self.lower_selection = value & (self.banks.len() - 1) as u8;
            }
            x if x < 0x6000 => {
                self.upper_selection = val & 0x3;
            }
            x if x < 0x8000 => {
                self.alt_bank_select = val & 1 == 0;
            }
            _ => {
                panic!("Illegal cartridge write at {:#x}", addr)
            }
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
    Unknown,
}

struct Rom {
    data: Vec<u8>,
    path: PathBuf,
}

impl Rom {
    fn new(path: &str) -> Result<Self> {
        let mut file = File::open(path).with_context(|| format!("Error opening {path}"))?;
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf)?;
        Ok(Rom {
            data: buf,
            path: PathBuf::try_from(path)?,
        })
    }

    fn get_title(&mut self) -> Result<String> {
        let str_arr = self.read_range(0x134, 0x143)?;
        let mut s = str::from_utf8(str_arr).context("Invalid UTF-8 in game title")?;
        s = s.trim_matches(char::from(0));
        Ok(s.to_owned())
    }

    fn is_gbc(&self) -> bool {
        self.data[0x143] & 0x80 != 0
    }

    fn get_ram_size(&self) -> Result<usize> {
        Ok(match self.data[0x149] {
            0 => 0,
            1 => 0x800,
            2 => 0x2000,
            3 => 0x8000,
            4 => 0x20000,
            5 => 0x10000,
            x => bail!("Invalid RAM size: {}", x),
        })
    }

    fn get_rom_size(&self) -> Result<usize> {
        Ok(match self.data[0x148] {
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
            x => bail!("Invalid ROM size: {}", x),
        })
    }

    fn get_mbc_type(&mut self) -> Result<MbcType> {
        let mbc_code = self.data[0x147];
        MbcType::try_from(mbc_code).context(format!("Unsuported MBC: {:x}", mbc_code))
    }

    fn get_data_len(&self) -> usize {
        self.data.len()
    }

    fn read_range(&self, begin: usize, end: usize) -> Result<&[u8]> {
        if self.data.len() < end {
            bail!("Load ROM out-of-bound read. Maybe the file is corrupted ?")
        }
        Ok(&self.data[begin..end])
    }
}
