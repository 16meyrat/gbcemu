use anyhow::{bail, Context, Result};
use num_enum::TryFromPrimitive;
use time::{OffsetDateTime, Duration};
use std::convert::TryFrom;
use std::fs::{File, OpenOptions};
use std::io::{prelude::*, SeekFrom};
use std::path::PathBuf;
use std::{str, slice};

pub trait Cartridge {
    fn read(&self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, val: u8);
}

pub fn load_rom(path: &str) -> Result<Box<dyn Cartridge>> {
    let mut rom = Rom::new(path)?;
    println!("Loading {path} ...");
    let title = rom.get_title()?;
    println!("Title: {title}");

    let gbc = rom.is_gbc();
    println!("Game Boy Color mode: {gbc}");

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
        MbcType::Mbc2 | MbcType::Mbc2Battery => {
            println!("Mapper is MBC2");
            Box::new(MBC2::new(
                &mut rom,
                matches!(mbc_type, MbcType::Mbc2Battery),
            )?)
        }
        MbcType::Mbc3 | MbcType::Mbc3Ram | MbcType::Mbc3RamBattery => {
            println!("Mapper is MBC3");
            Box::new(MBC3::new(
                &mut rom,
                matches!(mbc_type, MbcType::Mbc3RamBattery),
            )?)
        }
        _ => {
            bail!("Unsuported MBC : {mbc_type:?}");
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
            _ => panic!("Illegal cartridge read at {addr:#x}"),
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
            eprintln!("Game save failed: {e:#}");
        }
    }
}

impl Cartridge for MBC1 {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            x if x < 0x4000 => {
                if !self.alt_bank_select {
                    self.banks[0][addr as usize]
                } else {
                    self.banks[((self.upper_selection << 5) as usize & (self.banks.len() - 1))]
                        [addr as usize]
                }
            }
            x if x < 0x8000 => self.banks[self.get_rom_bank()][addr as usize - 0x4000],
            x if (0xa000..0xc000).contains(&x) => {
                if self.ram_enable && !self.ram.is_empty(){
                    *self.ram[self.get_ram_bank()]
                        .get(addr as usize - 0xa000)
                        .unwrap_or(&0)
                } else {
                    0
                }
            }
            _ => panic!("Illegal cartridge read at {addr:#x}"),
        }
    }
    fn write(&mut self, addr: u16, val: u8) {
        match addr {
            x if (0xa000..0xc000).contains(&x) => {
                if self.ram_enable {
                    if self.ram.is_empty() {
                        return;
                    }
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
                panic!("Illegal cartridge write at {addr:#x}")
            }
        };
    }
}

struct MBC2 {
    banks: Vec<[u8; 0x4000]>,
    ram: [u8; 0x200],
    ram_file: Option<File>,
    ram_enable: bool,
    bank_selection: u8,
}

impl MBC2 {
    pub fn new(rom: &mut Rom, battery: bool) -> Result<Self> {
        let bank_nb = rom.get_rom_size()? / 16;
        if bank_nb > 16 {
            bail!("Invalid bank count fo mbc1: {bank_nb}");
        }
        let ram_size = rom.get_ram_size()?;
        if ram_size != 0 {
            bail!("Invalid ram size for MBC2: {:x}", ram_size);
        }
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
        let mut ram = [0u8; 0x200];
        if let Some(ram_file) = &mut ram_file {
            let file_len = ram_file.seek(SeekFrom::End(0))?;
            ram_file.rewind()?;
            if file_len != 0 {
                ram_file.read_exact(&mut ram).with_context(|| {
                    format!("Save file {} is corrupted", save_path.display())
                })?;
                println!("Save file loaded from {}", save_path.display());
                ram_file.rewind()?;
            } else {
                println!("Save file created at {}", save_path.display());
            }
        }

        let mut res = MBC2 {
            banks: vec![[0; 0x4000]; bank_nb],
            ram,
            ram_file,
            ram_enable: false,
            bank_selection: 1,
        };
        for (i, chunk) in rom_data.chunks(0x4000).enumerate() {
            res.banks[i][..chunk.len()].copy_from_slice(chunk);
        }
        Ok(res)
    }
}

impl Drop for MBC2 {
    fn drop(&mut self) {
        if let Err(e) = (|| -> Result<()> {
            if let Some(ram_file) = &mut self.ram_file {
                ram_file.rewind()?;
                ram_file.write_all(&self.ram)?;
                println!("Game saved!");
            }
            Ok(())
        })() {
            eprintln!("Game save failed: {e:#}");
        }
    }
}

impl Cartridge for MBC2 {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            x if x < 0x4000 => {
                self.banks[0][addr as usize]
            }
            x if x < 0x8000 => self.banks[self.bank_selection as usize][addr as usize - 0x4000],
            x if (0xa000..0xa200).contains(&x) => {
                if self.ram_enable {
                    self.ram[(addr - 0xa000) as usize]
                } else {
                    0
                }
            }
            _ => panic!("Illegal cartridge read at {addr:#x}"),
        }
    }
    fn write(&mut self, addr: u16, val: u8) {
        match addr {
            x if (0xa000..0xa200).contains(&x) => {
                if self.ram_enable {
                    let index = addr as usize - 0xa000;
                    if index < self.ram.len() {
                        self.ram[index] = val;
                    }
                }
            }
            x if x < 0x4000 => {
                if x & 0x100 != 0 {
                    let masked = val & 0xf;
                    self.bank_selection = if masked > 0 {masked} else {1};
                } else {
                    self.ram_enable = val & 0x0f == 0x0a;
                }
            }
            _ => {
                panic!("Illegal cartridge write at {addr:#x}")
            }
        };
    }
}

struct MBC3 {
    banks: Vec<[u8; 0x4000]>,
    ram: Vec<[u8; 0x2000]>,
    save_file: Option<File>,
    ram_enable: bool,
    rom_selection: u8,
    ram_is_rtc: bool,
    ram_rtc_selection: u8,
    time_offset: Duration,
    latched_time: Option<OffsetDateTime>,
    time_stopped: Option<OffsetDateTime>,
}

impl MBC3 {
    pub fn new(rom: &mut Rom, battery: bool) -> Result<Self> {
        let bank_nb = rom.get_rom_size()? / 16;
        let ram_size = rom.get_ram_size()?;
        if ram_size != 0x0 && ram_size != 0x2000 && ram_size != 0x4000 && ram_size != 0x8000 {
            bail!("Invalid ram size for MBC3: {:x}", ram_size);
        }
        let ram_bank_nb = ram_size / 0x2000;
        let rom_data = rom.read_range(0, rom.get_data_len())?;
        let mut save_path = rom.path.clone();
        save_path.set_extension("save");
        let mut save_file = if battery {
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
        let mut time_offset = OffsetDateTime::UNIX_EPOCH - OffsetDateTime::now_utc();
        let mut time_stopped = Some(OffsetDateTime::UNIX_EPOCH);
        if let Some(save_file) = &mut save_file {
            let file_len = save_file.seek(SeekFrom::End(0))?;
            save_file.rewind()?;
            if file_len != 0 {
                for ram_bank in ram.iter_mut() {
                    save_file.read_exact(ram_bank).with_context(|| {
                        format!("Save file {} is corrupted", save_path.display())
                    })?;
                }
                let mut time_bytes = [0u8; 8];
                save_file.read_exact(&mut time_bytes).context("Failed to read save time info")?;
                time_offset = Duration::new(i64::from_be_bytes(time_bytes), 0);
                let mut time_stopped_byte = 0u8;
                save_file.read_exact(slice::from_mut(&mut time_stopped_byte))?;
                if time_stopped_byte != 0 {
                    time_stopped = Some(OffsetDateTime::UNIX_EPOCH + time_offset);
                } else {
                    time_stopped = None
                }
                println!("Save file loaded from {}", save_path.display());
                save_file.rewind()?;
            } else {
                println!("Save file created at {}", save_path.display());
            }
        }

        let mut res = MBC3 {
            banks: vec![[0; 0x4000]; bank_nb],
            ram,
            save_file,
            ram_enable: false,
            rom_selection: 1,
            ram_is_rtc: false,
            ram_rtc_selection: 0,
            time_offset,
            latched_time: None,
            time_stopped
        };
        for (i, chunk) in rom_data.chunks(0x4000).enumerate() {
            res.banks[i][..chunk.len()].copy_from_slice(chunk);
        }
        Ok(res)
    }

    fn get_game_time(&self) -> OffsetDateTime {
        self.latched_time.or(self.time_stopped).unwrap_or_else(||OffsetDateTime::now_utc() + self.time_offset)
    }

    fn set_game_time(&mut self, new_time: &OffsetDateTime) {
        if self.time_stopped.is_some() {
            self.time_stopped = Some(*new_time);
        } else {
            eprintln!("Setting the clock while it is running");
            self.time_offset = *new_time - OffsetDateTime::now_utc();
        }
    }
}

impl Drop for MBC3 {
    fn drop(&mut self) {
        if let Err(e) = (|| -> Result<()> {
            if let Some(ram_file) = &mut self.save_file {
                ram_file.rewind()?;
                for ram_bank in self.ram.iter_mut() {
                    ram_file.write_all(ram_bank)?;
                }
                if let Some(time_stopped) = self.time_stopped {
                    ram_file.write_all(&(time_stopped - OffsetDateTime::UNIX_EPOCH).whole_seconds().to_be_bytes())?;
                    ram_file.write_all(&[1u8])?;
                } else {
                    ram_file.write_all(&self.time_offset.whole_seconds().to_le_bytes())?;
                    ram_file.write_all(&[0u8])?;
                }
                println!("Game saved!");
            }
            Ok(())
        })() {
            eprintln!("Game save failed: {e:#}");
        }
    }
}

impl Cartridge for MBC3 {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            x if x < 0x4000 => {
                self.banks[0][addr as usize]
            }
            x if x < 0x8000 => self.banks[self.rom_selection as usize][addr as usize - 0x4000],
            x if (0xa000..0xc000).contains(&x) => {
                if !self.ram_enable {
                    return 0;
                }
                if self.ram_is_rtc {
                    match self.ram_rtc_selection {
                        8 => self.get_game_time().second(),
                        9 => self.get_game_time().minute(),
                        10 => self.get_game_time().hour(),
                        11 => ((self.get_game_time() - OffsetDateTime::UNIX_EPOCH).whole_days() & 0xff) as u8,
                        12 => {
                            let day_count = (self.get_game_time() - OffsetDateTime::UNIX_EPOCH).whole_days();
                            let mut res = 0u8;
                            if day_count & 0x100 != 0 {
                                res |= 1;
                            }
                            if day_count>= 0x200 {
                                res |= 0x80;
                            }
                            res |= if self.time_stopped.is_some() {0x40} else {0};
                            res
                        },
                        _ => panic!("Invalid MBC3 RTC selection at {:x}", self.ram_rtc_selection)
                    }
                } else {
                    self.ram[self.ram_rtc_selection as usize][(addr - 0xa000) as usize]
                }
            }
            _ => panic!("Illegal cartridge read at {addr:#x}"),
        }
    }
    fn write(&mut self, addr: u16, val: u8) {
        match addr {
            x if (0xa000..0xc000).contains(&x) => {
                if self.ram_enable {
                    if self.ram_is_rtc {
                        match self.ram_rtc_selection {
                            8 => {
                                self.set_game_time(&self.get_game_time().replace_second(val).expect("Invalid second write to RTC"));
                            },
                            9 => self.set_game_time(&self.get_game_time().replace_minute(val).expect("Invalid minute write to RTC")),
                            10 => self.set_game_time(&self.get_game_time().replace_hour(val).expect("Invalid hour write to RTC")),
                            11 => {
                                let old_offset = self.get_game_time() - OffsetDateTime::UNIX_EPOCH;
                                let old_seconds = old_offset.whole_seconds() % (3600*24);
                                let old_days = old_offset.whole_days();
                                let new_offset = Duration::seconds(old_seconds) + Duration::days(val as i64 + (old_days & 0x100));
                                self.set_game_time(&(OffsetDateTime::UNIX_EPOCH + new_offset));
                            }
                            12 => {
                                let old_offset = self.get_game_time() - OffsetDateTime::UNIX_EPOCH;
                                let old_seconds = old_offset.whole_seconds() % (3600*24);
                                let old_days = old_offset.whole_days();
                                let new_offset = Duration::seconds(old_seconds) + Duration::days((((val & 1) as i64) << 8) | (old_days & 0xff));
                                self.set_game_time(&(OffsetDateTime::UNIX_EPOCH + new_offset));
                                // TODO: ignore overflow flag at 0x80 for now
                                if val & 0x40 != 0 {
                                    if self.time_stopped.is_none() {
                                        self.time_stopped = Some(self.get_game_time());
                                    }
                                } else if let Some(time_stopped) = self.time_stopped{
                                    self.time_offset = time_stopped - OffsetDateTime::now_utc();
                                    self.time_stopped = None;
                                }
                            },
                            _ => panic!("Invalid MBC3 RTC selection at {:x}", self.ram_rtc_selection)
                        };
                    } else {
                        let index = addr as usize - 0xa000;
                        self.ram[self.ram_rtc_selection as usize][index] = val;
                    }
                }
            }
            x if x < 0x2000 => {
                self.ram_enable = val & 0xf == 0xa;
            }
            x if x < 0x4000 => {
                let value = val & 0x7f;
                self.rom_selection = if value > 0 {value} else {1};
            }
            x if x < 0x6000 => {
                match val {
                    0..=3 => {
                        self.ram_is_rtc = false;
                        self.ram_rtc_selection = val;
                    }
                    8..=12 => {
                        self.ram_is_rtc = true;
                        self.ram_rtc_selection = val;
                    }
                    _ => {
                        eprintln!("Invalid RAM RTC selection: {val:#x}");
                    }
                }
            }
            x if x < 0x8000 => {
                if val == 0 {
                    self.latched_time = None;
                } else {
                    self.latched_time = Some(self.get_game_time());
                }
            }
            _ => {
                panic!("Illegal cartridge write at {addr:#x}")
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
    Mbc5 = 0x19,
    Mbc5Ram = 0x1a,
    Mbc5RamBattery = 0x1b,
    Mbc5Rumble = 0x1c,
    Mbc5RumbleRam = 0x1d,
    Mbc5RumbleRamBattery = 0x1e,
    Mbc6 = 0x20,
    Mbc6All = 0x21,
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
        MbcType::try_from(mbc_code).context(format!("Unsuported MBC: {mbc_code:x}"))
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
