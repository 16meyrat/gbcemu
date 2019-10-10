use super::bus::Busable;
use num_enum::IntoPrimitive;
use arrayvec::ArrayVec;

use std::sync::Arc;
use std::sync::Mutex;

use std::time::{Instant, Duration};
use std::thread::sleep;

use crate::gui;

pub struct Ppu {
    background_palette: ArrayVec<[Color; 4]>,
    obj_palette0: ArrayVec<[Color; 4]>,
    obj_palette1: ArrayVec<[Color; 4]>,
    scx: u8,
    scy: u8,
    wx: u8, // real_WX - 7
    wy: u8,
    ly: u8,
    lcy: u8,
    enabled: bool,
    win_enabled: bool,
    sprite_enabled: bool,
    bg_win_priority: bool,
    obj_size: ObjSize,
    win_map_select: WindowMapSelect,
    win_bg_data: WindowBGTileData,
    bg_map_select: BgMapSelect,

    int_vblank: bool,
    int_hblank: bool,
    int_lcy: bool,
    int_oam: bool,

    current_mode: Mode,

    vram: [u8; 0x2000],
    oam: [u8; 0xA0],

    wait: usize,
    last_time: Instant,

    rendering_texure: Arc<Mutex<[u8; gui::SIZE]>>,
    texture: Vec<[Color; gui::WIDTH]>,
}

pub enum PpuInterrupt {
    None,
    VBlank,
    Stat,
}

impl Ppu {
    pub fn new(rendering_texure: Arc<Mutex<[u8; gui::SIZE]>>) -> Self {
        
        Ppu{
            background_palette: (0..4).map(|idx|bw_palette(idx as u8)).collect::<ArrayVec<[Color; 4]>>(),
            obj_palette0: (0..4).map(|idx|bw_palette(idx as u8)).collect::<ArrayVec<[Color; 4]>>(),
            obj_palette1: (0..4).map(|idx|bw_palette(idx as u8)).collect::<ArrayVec<[Color; 4]>>(),
            scx: 0,
            scy: 0,
            wx: 0,
            wy: 0,
            ly: 0,
            lcy: 0,
            enabled: false,
            win_enabled: false,
            sprite_enabled: false,
            bg_win_priority: false,
            obj_size: ObjSize::Small,
            win_map_select: WindowMapSelect::Low,
            win_bg_data: WindowBGTileData::Low,
            bg_map_select: BgMapSelect::Low,

            int_vblank: false,
            int_hblank: false,
            int_lcy: false,
            int_oam: false,

            current_mode: Mode::VBlank,

            vram: [0; 0x2000],
            oam: [0; 0xA0],

            wait: 0,
            last_time: Instant::now(),

            rendering_texure,
            texture: vec![[Color::new(0, 0, 0); gui::WIDTH]; gui::HEIGHT],
        }
    }

    pub fn get_bgp(&self) -> u8 {
        self.background_palette.iter().enumerate().fold(0, |acc, (i, color)|(acc | color.palette_index << 2*i))
    }

    pub fn set_bgp(&mut self, new_palette: u8) {
        for (i, col) in self.background_palette.iter_mut().enumerate() {
            *col = bw_palette((new_palette & (0x3 << 2*i)) >> 2*i);
        }
    }

    pub fn get_obp0(&self) -> u8 {
        self.obj_palette0.iter().enumerate().fold(0, |acc, (i, color)|(acc | color.palette_index << 2*i))
    }

    pub fn set_obp0(&mut self, new_palette: u8) {
        for (i, col) in self.obj_palette0.iter_mut().enumerate() {
            *col = bw_palette((new_palette & (0x3 << 2*i)) >> 2*i);
        }
    }

    pub fn get_obp1(&self) -> u8 {
        self.obj_palette1.iter().enumerate().fold(0, |acc, (i, color)|(acc | color.palette_index << 2*i))
    }

    pub fn set_obp1(&mut self, new_palette: u8) {
        for (i, col) in self.obj_palette1.iter_mut().enumerate() {
            *col = bw_palette((new_palette & (0x3 << 2*i)) >> 2*i);
        }
    }

    pub fn get_wx(&self) -> u8 {
        self.wx + 7
    }

    pub fn set_wx(&mut self, val: u8) {
        self.wx = val - 7;
    }

    pub fn get_wy(&self) -> u8 {
        self.wy
    }

    pub fn set_wy(&mut self, val: u8) {
        self.wy = val;
    }

    pub fn get_scy(&self) -> u8 {
        self.scy
    }

    pub fn set_scy(&mut self, val: u8) {
        self.scy = val;
    }

    pub fn get_scx(&self) -> u8 {
        self.scx
    }

    pub fn set_scx(&mut self, val: u8) {
        self.scx = val;
    }

    pub fn get_ly(&self) -> u8 {
        self.ly
    }

    pub fn set_ly(&self, _val: u8) {
        panic!("LY is not writable");
    }

    pub fn get_lcy(&self) -> u8 {
        self.lcy
    }

    pub fn set_lcy(&mut self, val: u8) {
        self.lcy = val;
    }

    pub fn get_lcdc(&self) -> u8 {
        (if self.enabled {0x80} else {0})
        | (if let WindowMapSelect::High = self.win_map_select {0x40} else {0})
        | (if self.win_enabled {0x20} else {0})
        | (if let WindowBGTileData::High = self.win_bg_data {0x10} else {0})
        | (if let BgMapSelect::High = self.bg_map_select { 0x08} else {0})
        | (if let ObjSize::Big = self.obj_size {0x04} else {0})
        | (if self.sprite_enabled {0x02} else {0})
        | (if self.bg_win_priority {0x01} else {0})
    }

    pub fn set_lcdc(&mut self, val: u8) {
        self.enabled = val & 0x80 != 0;
        self.win_map_select = match val & 0x40 {
            0 => WindowMapSelect::Low,
            _ => WindowMapSelect::High,
        };
        self.win_enabled = val & 0x20 != 0;
        self.win_bg_data = match val & 0x10 {
            0 => WindowBGTileData::Low,
            _ => WindowBGTileData::High,
        };
        self.bg_map_select = match val & 0x08 {
            0 => BgMapSelect::Low,
            _ => BgMapSelect::High,
        };
       self.obj_size = match val & 0x04 {
            0 => ObjSize::Small,
            _ => ObjSize::Big,
        };
        self.sprite_enabled = val & 0x02 != 0;
        self.bg_win_priority = val & 0x01 != 0;
    }

    pub fn get_lcds(&self) -> u8 {
        let mode: u8 = self.current_mode.into();
        (self.int_lcy as u8) << 6
        | (self.int_oam as u8) << 5
        | (self.int_vblank as u8) << 4
        | (self.int_hblank as u8) << 3
        | if self.lcy == self.ly {0x04} else {0}
        | mode
    }

    pub fn set_lcds(&mut self, val: u8) {
        self.int_lcy = val & 0x40 != 0;
        self.int_oam = val & 0x20 != 0;
        self.int_vblank = val & 0x10 != 0;
        self.int_hblank = val & 0x08 != 0;
    }

    pub fn tick(&mut self) -> PpuInterrupt {
        if !self.enabled {
            return PpuInterrupt::None;
        }

        if self.wait != 0 {
            self.wait -= 1;
            return PpuInterrupt::None;
        }

        let mut res = PpuInterrupt::None;

        match self.current_mode {
            Mode::OamScan => {
                self.wait = 200;
                self.current_mode = Mode::Rendering;
            }
            Mode::Rendering => {
                self.render_line();
                self.wait = 100;
                self.current_mode = Mode::HBlank;
                if self.int_hblank {
                    res = PpuInterrupt::Stat;
                }
            }
            Mode::HBlank => {
                self.ly += 1;
                if self.ly == self.lcy && self.int_lcy {
                    res = PpuInterrupt::Stat;
                }
                if self.ly >= 144 {
                    self.render();
                    self.wait = 456;
                    self.current_mode = Mode::VBlank;
                    if self.int_vblank {
                        res = PpuInterrupt::VBlank;
                    }
                } else {
                    self.wait = 80;
                    self.current_mode = Mode::OamScan;
                }
            }
            Mode::VBlank => {
                if self.ly < 154 {
                    self.wait = 456;
                    self.ly += 1;
                } else {
                    self.ly = 0;
                    self.wait = 80;
                    self.current_mode = Mode::OamScan;
                    if self.int_oam {
                        res = PpuInterrupt::Stat;
                    }
                }
            }
        }
        res
    }

    fn render_line(&mut self) {
        self.render_background();

        let last_time = self.last_time;
        self.last_time = Instant::now();
        let elapsed = self.last_time.duration_since(last_time).as_micros();
        if elapsed < (16_666 - 1_000) {
            sleep(Duration::from_micros(16_666 - elapsed as u64));
        } else {
            std::thread::yield_now();
        }
    }

    fn render_background(&mut self) {
        let get_tile = if let WindowBGTileData::High = self.win_bg_data {
            Ppu::get_tile_line_signed
        } else {
            Ppu::get_tile_line_unsigned
        };
        let y = u8::wrapping_add(self.ly, self.scy);
        let mut x = 0;
        while x < gui::WIDTH {
            let rel_x = x + self.scx as usize;
            let tile_index = self.vram[self.bg_map_select as usize + y as usize * 8 + rel_x / 8];
            let tile_data = get_tile(&self, tile_index, y as usize);
            let offset_x = rel_x % 8;
            for tile_x in offset_x..8 {
                match self.texture[self.ly as usize].get_mut(x + tile_x - offset_x) {
                    Some(pixel) => *pixel = self.background_palette[tile_data[tile_x as usize] as usize],
                    _ => {return;}
                } 
            }
            x += 8 - offset_x;
        }
    }

    fn render_window(&mut self) {
        
    }

    fn get_tile_line_signed(&self, nb: u8 , y: usize) -> [u8; 8] {
        let addr = (0x1000 + nb as i8 as isize * 16 ) + y as isize % 8 * 2;
        let l = self.vram[addr as usize];
        let h = self.vram[addr as usize + 1];
        let mut res = [0u8; 8];
        for i in 0..8 {
            res[i] = ((h >> 7-i & 1u8) << 1) | ((l >> 7-i & 1u8));
        }
        res
    }

    fn get_tile_line_unsigned(&self, nb: u8 , y: usize) -> [u8; 8] {
        let addr = nb as usize * 16 + y % 8 * 2;
        let l = self.vram[addr];
        let h = self.vram[addr + 1];
        let mut res = [0u8; 8];
        for i in 0..8 {
            res[i] = ((h >> 7-i & 1u8) << 1) | ((l >> 7-i & 1u8));
        }
        res
    }

    fn render(&mut self) {
        let mut target = self.rendering_texure.lock().unwrap();
        let mut index = 0;
        for y in 0..gui::HEIGHT {
            for x in 0..gui::WIDTH {
                unsafe {
                    let color = self.texture.get_unchecked(y).get_unchecked(x);
                    *target.get_unchecked_mut(index) = color.r;
                    *target.get_unchecked_mut(index + 1) = color.g;
                    *target.get_unchecked_mut(index + 2) = color.b;
                    index += 3;
                }
            }
        }
    }
}

impl Busable for Ppu {
    fn read(&self, addr: u16) -> u8{
        if addr < 0xA000 {
            self.vram[(addr - 0x8000) as usize]
        } else if addr < 0xfea0 {
            self.oam[(addr - 0xfe00) as usize]
        } else {
            panic!("Illegal VRam read : {:#x}", addr)
        }
    }
    fn write(&mut self, addr: u16, val: u8){
        if addr < 0xA000 {
            self.vram[(addr - 0x8000) as usize] = val;
        } else if addr < 0xfea0 {
            self.oam[(addr - 0xfe00) as usize] = val;
        } else {
            panic!("Illegal VRam write : {:#x}", addr)
        }
    }
}

#[derive(Clone, Copy, IntoPrimitive)]
#[repr(u16)]
enum WindowMapSelect {
    Low = 0x1800,
    High = 0x1C00,
}

#[derive(Clone, Copy, IntoPrimitive)]
#[repr(u16)]
enum WindowBGTileData {
    Low = 0x0800,
    High = 0x0000,
}

#[derive(Clone, Copy, IntoPrimitive)]
#[repr(u16)]
enum BgMapSelect {
    Low = 0x1800,
    High = 0x1C00,
}

enum ObjSize {
    Small,
    Big
}

#[derive(Clone, Copy, IntoPrimitive)]
#[repr(u8)]
enum Mode {
    HBlank = 0,
    VBlank = 1,
    OamScan = 2,
    Rendering = 3,
}

#[derive(Clone, Copy)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
    palette_index: u8,
}

impl Color {
    fn new(r: u8, g: u8, b: u8) -> Self{
        Color{r, g, b, palette_index: 0}
    }
    fn from_palette(r: u8, g: u8, b: u8, index: u8) -> Self{
        Color{r, g, b, palette_index: index}
    }
    
}

fn bw_palette(entry: u8) -> Color {
    match entry {
        3 => Color::from_palette(0, 0, 0, 3),
        2 => Color::from_palette(50, 50, 50, 2),
        1 => Color::from_palette(100, 100, 100, 1),
        0 => Color::from_palette(150, 150, 150, 0),
        x => panic!("Unknown BW color : {}", x),
    }
}
