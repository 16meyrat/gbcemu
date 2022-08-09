use super::bus::Busable;
use arrayvec::ArrayVec;
use num_enum::IntoPrimitive;

use crate::gui;

pub struct Ppu {
    background_palette: ArrayVec<Color, 4>,
    obj_palette0: ArrayVec<Color, 4>,
    obj_palette1: ArrayVec<Color, 4>,
    scx: u8,
    scy: u8,
    wx: u8, // real_WX - 7
    wy: u8,
    ly: u8,
    lyc: u8,
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
    int_lyc: bool,
    int_oam: bool,

    current_mode: Mode,

    vram: [u8; 0x2000],
    oam: [u8; 0xA0],

    wait: usize,

    texture: Vec<[Color; gui::WIDTH]>,
}

pub enum PpuInterrupt {
    None,
    VBlank,
    Stat,
}

impl Ppu {
    pub fn new() -> Self {
        Ppu {
            background_palette: (0..4)
                .map(|idx| bw_palette(idx as u8, idx))
                .collect::<ArrayVec<Color, 4>>(),
            obj_palette0: (0..4)
                .map(|idx| bw_palette(idx as u8, idx))
                .collect::<ArrayVec<Color, 4>>(),
            obj_palette1: (0..4)
                .map(|idx| bw_palette(idx as u8, idx))
                .collect::<ArrayVec<Color, 4>>(),
            scx: 0,
            scy: 0,
            wx: 0,
            wy: 0,
            ly: 0,
            lyc: 0,
            enabled: true,
            win_enabled: false,
            sprite_enabled: false,
            bg_win_priority: true,
            obj_size: ObjSize::Small,
            win_map_select: WindowMapSelect::High,
            win_bg_data: WindowBGTileData::Low,
            bg_map_select: BgMapSelect::Low,

            int_vblank: false,
            int_hblank: false,
            int_lyc: false,
            int_oam: false,

            current_mode: Mode::OamScan,

            vram: [0; 0x2000],
            oam: [0; 0xA0],

            wait: 0,

            texture: vec![[Color::new(0, 0, 0); gui::WIDTH]; gui::HEIGHT],
        }
    }

    pub fn get_bgp(&self) -> u8 {
        self.background_palette
            .iter()
            .enumerate()
            .fold(0, |acc, (i, color)| (acc | color.palette_index << (2 * i)))
    }

    pub fn set_bgp(&mut self, new_palette: u8) {
        for (i, col) in self.background_palette.iter_mut().enumerate() {
            *col = bw_palette((new_palette & (0x3 << (2 * i))) >> (2 * i), i as u8);
        }
    }

    pub fn get_obp0(&self) -> u8 {
        self.obj_palette0
            .iter()
            .enumerate()
            .fold(0, |acc, (i, color)| (acc | color.palette_index << (2 * i)))
    }

    pub fn set_obp0(&mut self, new_palette: u8) {
        for (i, col) in self.obj_palette0.iter_mut().enumerate() {
            *col = bw_palette((new_palette & (0x3 << (2 * i))) >> (2 * i), i as u8);
        }
    }

    pub fn get_obp1(&self) -> u8 {
        self.obj_palette1
            .iter()
            .enumerate()
            .fold(0, |acc, (i, color)| (acc | color.palette_index << (2 * i)))
    }

    pub fn set_obp1(&mut self, new_palette: u8) {
        for (i, col) in self.obj_palette1.iter_mut().enumerate() {
            *col = bw_palette((new_palette & (0x3 << (2 * i))) >> (2 * i), i as u8);
        }
    }

    pub fn get_wx(&self) -> u8 {
        self.wx
    }

    pub fn set_wx(&mut self, val: u8) {
        self.wx = val;
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

    pub fn get_lyc(&self) -> u8 {
        self.lyc
    }

    pub fn set_lyc(&mut self, val: u8) {
        self.lyc = val;
    }

    pub fn get_lcdc(&self) -> u8 {
        (if self.enabled { 0x80 } else { 0 })
            | (if let WindowMapSelect::High = self.win_map_select {
                0x40
            } else {
                0
            })
            | (if self.win_enabled { 0x20 } else { 0 })
            | (if let WindowBGTileData::High = self.win_bg_data {
                0x10
            } else {
                0
            })
            | (if let BgMapSelect::High = self.bg_map_select {
                0x08
            } else {
                0
            })
            | (if let ObjSize::Big = self.obj_size {
                0x04
            } else {
                0
            })
            | (if self.sprite_enabled { 0x02 } else { 0 })
            | (if self.bg_win_priority { 0x01 } else { 0 })
    }

    pub fn set_lcdc(&mut self, val: u8) {
        self.enabled = if val & 0x80 != 0 {
            if !self.enabled {
                self.ly = 0;
                self.current_mode = Mode::OamScan;
            }
            true
        } else {
            false
        };
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
        (self.int_lyc as u8) << 6
            | (self.int_oam as u8) << 5
            | (self.int_vblank as u8) << 4
            | (self.int_hblank as u8) << 3
            | if self.lyc == self.ly { 0x04 } else { 0 }
            | mode
    }

    pub fn set_lcds(&mut self, val: u8) {
        self.int_lyc = val & 0x40 != 0;
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
                self.wait = 180;
                self.current_mode = Mode::Rendering;
            }
            Mode::Rendering => {
                self.render_line();
                self.wait = 196;
                self.current_mode = Mode::HBlank;
                if self.int_hblank {
                    res = PpuInterrupt::Stat;
                }
            }
            Mode::HBlank => {
                self.ly += 1;
                if self.ly == self.lyc && self.int_lyc {
                    res = PpuInterrupt::Stat;
                }
                if self.ly >= 144 {
                    self.wait = 456;
                    self.current_mode = Mode::VBlank;
                    res = PpuInterrupt::VBlank;
                } else {
                    self.wait = 80;
                    self.current_mode = Mode::OamScan;
                    if self.int_oam {
                        res = PpuInterrupt::Stat;
                    }
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
        if self.bg_win_priority {
            self.render_background();
            if self.win_enabled {
                self.render_window();
            }
        }
        if self.sprite_enabled {
            self.render_sprites();
        }
    }

    fn render_background(&mut self) {
        let get_tile = if let WindowBGTileData::Low = self.win_bg_data {
            Ppu::get_tile_line_signed
        } else {
            Ppu::get_tile_line_unsigned
        };
        let y = u8::wrapping_add(self.ly, self.scy);
        let mut x = 0;
        while x < gui::WIDTH {
            let rel_x = (x + self.scx as usize) % 256;
            let tile_index =
                self.vram[self.bg_map_select as usize + y as usize / 8 * 32 + rel_x / 8];
            let tile_data = get_tile(self, tile_index, y as usize);
            let offset_x = rel_x % 8;
            for tile_x in offset_x..8 {
                match self.texture[self.ly as usize].get_mut(x + tile_x - offset_x) {
                    Some(pixel) => {
                        *pixel = self.background_palette[tile_data[tile_x as usize] as usize]
                    }
                    _ => {
                        return;
                    }
                }
            }
            x += 8 - offset_x;
        }
    }

    fn render_window(&mut self) {
        let get_tile = if let WindowBGTileData::Low = self.win_bg_data {
            Ppu::get_tile_line_signed
        } else {
            Ppu::get_tile_line_unsigned
        };
        let y = match u8::checked_sub(self.ly, self.wy) {
            Some(result) => result,
            _ => return,
        };
        let mut x = (self.wx as usize).saturating_sub(7);
        while x < gui::WIDTH {
            let rel_x = x + 7 - self.wx as usize;
            let tile_index =
                self.vram[self.win_map_select as usize + y as usize / 8 * 32 + rel_x / 8];
            let tile_data = get_tile(self, tile_index, y as usize);
            for tile_x in 0..8 {
                match self.texture[self.ly as usize].get_mut(x + tile_x) {
                    Some(pixel) => {
                        *pixel = self.background_palette[tile_data[tile_x as usize] as usize]
                    }
                    _ => {
                        return;
                    }
                }
            }
            x += 8;
        }
    }

    fn render_sprites(&mut self) {
        let mut oam_data = self.get_sprites_on_line();

        oam_data.sort_by_key(|sprite| sprite.x);
        for sprite in oam_data.iter().take(10).rev() {
            let tile = self.get_sprite_tile_line(sprite);
            let palette = if sprite.palette {
                self.obj_palette1.clone()
            } else {
                self.obj_palette0.clone()
            };
            if !sprite.x_flip {
                for tile_x in 0..8 {
                    let x = match sprite.x.saturating_add(tile_x).checked_sub(8) {
                        Some(x) => x,
                        None => continue,
                    };
                    if let Some(pixel) =
                        self.texture[self.ly as usize].get_mut(x as usize)
                    {
                        if !sprite.behind_bg || pixel.palette_index == 0 {
                            let color = palette[tile[tile_x as usize] as usize];
                            if color.palette_index != 0 {
                                *pixel = color;
                            }
                        }
                    } // do not break, because of the left screen border
                }
            } else {
                for tile_x in 0..8 {
                    let x = match (sprite.x + tile_x).checked_sub(8) {
                        Some(x) => x,
                        None => continue,
                    };
                    if let Some(pixel) =
                        self.texture[self.ly as usize].get_mut(x as usize)
                    {
                        if !sprite.behind_bg || pixel.palette_index == 0 {
                            let color = palette[tile[7 - tile_x as usize] as usize];
                            if color.palette_index != 0 {
                                *pixel = color;
                            }
                        }
                    } // do not break, because of the left screen border
                }
            }
        }
    }

    fn get_sprites_on_line(&self) -> Vec<SpriteOam> {
        let mut res = Vec::new();
        let height = match self.obj_size {
            ObjSize::Small => 8,
            ObjSize::Big => 16,
        };
        let y_sec = self.ly + 16;
        for i in (0..0xa0).step_by(4) {
            let data = &self.oam[i..i + 4];
            let y_sprite = data[0];
            if y_sprite <= y_sec && y_sec < y_sprite + height {
                let flags = data[3];
                res.push(SpriteOam {
                    y: data[0],
                    x: data[1],
                    tile: data[2],
                    behind_bg: flags & 80 != 0,
                    y_flip: flags & 0x40 != 0,
                    x_flip: flags & 0x20 != 0,
                    palette: flags & 0x10 != 0,
                });
                if res.len() >= 10 {
                    return res;
                }
            }
        }
        res
    }

    fn get_tile_line_signed(&self, nb: u8, y: usize) -> [u8; 8] {
        let addr = (0x1000 + nb as i8 as isize * 16) + y as isize % 8 * 2;
        let l = self.vram[addr as usize];
        let h = self.vram[addr as usize + 1];
        let mut res = [0u8; 8];
        for (i, x) in res.iter_mut().enumerate() {
            *x = ((h >> (7 - i) & 1u8) << 1) | ((l >> (7 - i)) & 1u8);
        }
        res
    }

    fn get_tile_line_unsigned(&self, nb: u8, y: usize) -> [u8; 8] {
        let addr = nb as usize * 16 + y % 8 * 2;
        let l = self.vram[addr];
        let h = self.vram[addr + 1];
        let mut res = [0u8; 8];
        for (i, x) in res.iter_mut().enumerate() {
            *x = (((h >> (7 - i)) & 1u8) << 1) | ((l >> (7 - i)) & 1u8);
        }
        res
    }

    fn get_sprite_tile_line(&self, sprite: &SpriteOam) -> [u8; 8] {
        let mut y_offset = self.ly + 16 - sprite.y;

        if sprite.y_flip {
            let sprite_height = match self.obj_size {
                ObjSize::Small => 8,
                ObjSize::Big => 16,
            };
            y_offset = sprite_height - 1 - y_offset;
        }

        let addr = match y_offset {
            y if y < 8 => {
                if let ObjSize::Big = self.obj_size {
                    (sprite.tile & 0xfe) as usize * 16 + y as usize * 2
                } else {
                    sprite.tile as usize * 16 + y as usize * 2
                }
            }
            y if y >= 8 => (sprite.tile | 1) as usize * 16 + y as usize % 8 * 2, // necessarily ObjSize::Big
            _ => panic!("Unexpected y offset"),
        };
        let l = self.vram[addr];
        let h = self.vram[addr + 1];
        let mut res = [0u8; 8];
        for (i, x) in res.iter_mut().enumerate() {
            *x = ((h >> (7 - i) & 1u8) << 1) | ((l >> (7 - i)) & 1u8);
        }
        res
    }

    pub fn render(&self, target: &mut [u8; gui::SIZE]) {
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
    fn read(&self, addr: u16) -> u8 {
        if addr < 0xA000 {
            self.vram[(addr - 0x8000) as usize]
        } else if addr < 0xfea0 {
            self.oam[(addr - 0xfe00) as usize]
        } else {
            panic!("Illegal VRam read : {:#x}", addr)
        }
    }
    fn write(&mut self, addr: u16, val: u8) {
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
    Low = 0x1000,
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
    Big,
}

#[derive(Clone, Copy, IntoPrimitive)]
#[repr(u8)]
enum Mode {
    HBlank = 0,
    VBlank = 1,
    OamScan = 2,
    Rendering = 3,
}

struct SpriteOam {
    x: u8,
    y: u8,
    tile: u8,
    behind_bg: bool,
    y_flip: bool,
    x_flip: bool,
    palette: bool,
}

#[derive(Clone, Copy)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
    palette_index: u8,
}

impl Color {
    fn new(r: u8, g: u8, b: u8) -> Self {
        Color {
            r,
            g,
            b,
            palette_index: 0,
        }
    }
    fn from_palette(r: u8, g: u8, b: u8, index: u8) -> Self {
        Color {
            r,
            g,
            b,
            palette_index: index,
        }
    }
}

fn bw_palette(entry: u8, index: u8) -> Color {
    match entry {
        3 => Color::from_palette(0, 0, 0, index),
        2 => Color::from_palette(50, 50, 50, index),
        1 => Color::from_palette(100, 100, 100, index),
        0 => Color::from_palette(150, 150, 150, index),
        x => panic!("Unknown BW color : {}", x),
    }
}
