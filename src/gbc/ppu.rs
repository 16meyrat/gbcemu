use super::bus::Busable;
use arrayvec::ArrayVec;

pub struct Ppu {
    background_palette: ArrayVec<[Color; 4]>,
    obj_palette0: ArrayVec<[Color; 4]>,
    obj_palette1: ArrayVec<[Color; 4]>,
    scx: u8,
    scy: u8,
    wx: u8, // real_WX - 7
    wy: u8,
    enabled: bool,
    win_enabled: bool,
    sprite_enabled: bool,
    bg_win_priority: bool,
    obj_size: ObjSize,
    win_map_select: WindowMapSelect,
    win_bg_data: WindowBGTileData,
    bg_map_select: BgMapSelect,
}

impl Ppu {
    pub fn new() -> Self {
        
        Ppu{
            background_palette: (0..3).map(|idx|bw_palette(idx as u8)).collect::<ArrayVec<[Color; 4]>>(),
            obj_palette0: (0..3).map(|idx|bw_palette(idx as u8)).collect::<ArrayVec<[Color; 4]>>(),
            obj_palette1: (0..3).map(|idx|bw_palette(idx as u8)).collect::<ArrayVec<[Color; 4]>>(),
            scx: 0,
            scy: 0,
            wx: 0,
            wy: 0,
            enabled: false,
            win_enabled: false,
            sprite_enabled: false,
            bg_win_priority: false,
            obj_size: ObjSize::Small,
            win_map_select: WindowMapSelect::Low,
            win_bg_data: WindowBGTileData::Low,
            bg_map_select: BgMapSelect::Low,
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

    pub fn set_wy(&self, val: u8) {
        self.wy = val;
    }

    pub fn get_scy(&self) -> u8 {
        self.scy
    }

    pub fn set_scy(&self, val: u8) {
        self.scy = val;
    }

    pub fn get_scx(&self) -> u8 {
        self.scx
    }

    pub fn set_scx(&self, val: u8) {
        self.scx = val;
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
}

impl Busable for Ppu {
    fn read(&self, addr: u16) -> u8{
        0
    }
    fn write(&mut self, addr: u16, val: u8){

    }
}

enum WindowMapSelect {
    Low = 0x9800,
    High = 0x9C00,
}

enum WindowBGTileData {
    Low = 0x8800,
    High = 0x8000,
}

enum BgMapSelect {
    Low = 0x9800,
    High = 0x9C00,
}

enum ObjSize {
    Small,
    Big
}

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