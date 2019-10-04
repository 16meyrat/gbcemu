
pub struct Ram {
    bank0: [u8; 0x800],
    bank1: [u8; 0x800],
    bank2: [u8; 0x800],
    bank3: [u8; 0x800],
    bank4: [u8; 0x800],
    bank5: [u8; 0x800],
    bank6: [u8; 0x800],
    bank7: [u8; 0x800],
}

impl Ram {
    pub fn new() -> Self{
        Ram{
            bank0: [0; 0x800],
            bank1: [0; 0x800],
            bank2: [0; 0x800],
            bank3: [0; 0x800],
            bank4: [0; 0x800],
            bank5: [0; 0x800],
            bank6: [0; 0x800],
            bank7: [0; 0x800],
        }
    }
}