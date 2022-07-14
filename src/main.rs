mod gbc;
mod gui;

use gbc::Emu;

use gui::{Message, Gui};

use std::thread;
use std::sync::{mpsc, Arc, Mutex};
use std::env;

use anyhow::{Result, bail};

fn main() -> Result<()>{

    if env::args().count() != 2 {
        bail!("Please enter the path to ROM.GB");
    }

    let rom_name = env::args().nth(1).unwrap();
    let mut emu = Emu::new(&rom_name)?;
    let mut gui = Gui::new(emu);
    gui.run();
    Ok(())
}

