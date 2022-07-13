use crate::gui::{GBKey, Message};

pub struct Joypad {
    key_a: KeyState,
    key_b: KeyState,
    key_st: KeyState,
    key_se: KeyState,
    key_up: KeyState,
    key_dw: KeyState,
    key_le: KeyState,
    key_ri: KeyState,

    show_directions: bool,
}

impl Joypad {
    pub fn new() -> Self {
        Joypad {
            key_a: KeyState::Released,
            key_b: KeyState::Released,
            key_st: KeyState::Released,
            key_se: KeyState::Released,
            key_up: KeyState::Released,
            key_dw: KeyState::Released,
            key_le: KeyState::Released,
            key_ri: KeyState::Released,

            show_directions: false,
        }
    }

    pub fn update(&mut self, msg: Message) -> bool {
        match msg {
            Message::KeyDown(x) => {
                let prev_state = self.get_key_state(x);
                self.set_key_state(x, KeyState::Pressed);
                prev_state != self.get_key_state(x)
            }
            Message::KeyUp(x) => {
                self.set_key_state(x, KeyState::Released);
                false
            }
            _ => false,
        }
    }

    pub fn write(&mut self, val: u8) {
        self.show_directions = !val & 0x10 != 0
    }

    pub fn read(&self) -> u8 {
        if self.show_directions {
            0x20 | self.key_dw.to_bit() << 3
                | self.key_up.to_bit() << 2
                | self.key_le.to_bit() << 1
                | self.key_ri.to_bit()
        } else {
            0x10 | self.key_st.to_bit() << 3
                | self.key_se.to_bit() << 2
                | self.key_b.to_bit() << 1
                | self.key_a.to_bit()
        }
    }

    fn set_key_state(&mut self, key: GBKey, status: KeyState) {
        match key {
            GBKey::A => self.key_a = status,
            GBKey::B => self.key_b = status,
            GBKey::Start => self.key_st = status,
            GBKey::Select => self.key_se = status,
            GBKey::Up => self.key_up = status,
            GBKey::Down => self.key_dw = status,
            GBKey::Left => self.key_le = status,
            GBKey::Right => self.key_ri = status,
        }
    }

    fn get_key_state(&mut self, key: GBKey) -> KeyState {
        match key {
            GBKey::A => self.key_a,
            GBKey::B => self.key_b,
            GBKey::Start => self.key_st,
            GBKey::Select => self.key_se,
            GBKey::Up => self.key_up,
            GBKey::Down => self.key_dw,
            GBKey::Left => self.key_le,
            GBKey::Right => self.key_ri,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum KeyState {
    Pressed,
    Released,
}

impl KeyState {
    fn to_bit(&self) -> u8 {
        match self {
            KeyState::Pressed => 0,
            KeyState::Released => 1,
        }
    }
}
