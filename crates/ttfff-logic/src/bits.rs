use crate::*;

impl Place {
    pub fn to_bits(self) -> u8 {
        let mut xo: u8 = 0;
        let mut ts: u8 = 0;
        let mut set = |s: Sym| match s {
            Sym::X => xo = 1,
            Sym::O => xo = 2,
            Sym::T => ts = 1,
            Sym::S => ts = 2,
        };
        match self {
            Empty => {}
            OnePlaced(a) => set(a),
            TwoPlaced(a, b) => {
                set(a);
                set(b);
            }
        }
        (ts << 2) | xo
    }

    pub fn from_bits(bits: u8) -> Place {
        let xo = bits & 0b11;
        let ts = (bits >> 2) & 0b11;
        match (xo, ts) {
            (0, 0) => Empty,
            (1, 0) => OnePlaced(X),
            (2, 0) => OnePlaced(O),
            (0, 1) => OnePlaced(T),
            (0, 2) => OnePlaced(S),
            (1, 1) => TwoPlaced(X, T),
            (1, 2) => TwoPlaced(X, S),
            (2, 1) => TwoPlaced(O, T),
            (2, 2) => TwoPlaced(O, S),
            _ => unreachable!("because masking & 0b11"),
        }
    }
}

impl GameMove {
    pub fn to_bits(&self) -> u8 {
        let who_bit = match self.who {
            Player::XS => 0,
            Player::OT => 1,
        };
        let place_bits = self.place.0 & 0b1111;
        let sym_bit = match self.drawn_symbol {
            Sym::X => 0,
            Sym::S => 1,
            Sym::O => 0,
            Sym::T => 1,
        };
        (who_bit << 5) | (place_bits << 1) | sym_bit
    }
    pub fn from_bits(bits: u8) -> GameMove {
        let who = if (bits & (1 << 5)) == 0 {
            Player::XS
        } else {
            Player::OT
        };
        let place = PlaceIdx((bits >> 1) & 0b1111);
        let drawn_symbol = match who {
            Player::XS => {
                if (bits & 0b1) == 0 {
                    Sym::X
                } else {
                    Sym::S
                }
            }
            Player::OT => {
                if (bits & 0b1) == 0 {
                    Sym::O
                } else {
                    Sym::T
                }
            }
        };
        GameMove {
            who,
            place,
            drawn_symbol,
        }
    }
}

impl Game {
    pub fn to_bits(&self) -> u64 {
        // 9*4 bits per place, 1 bit for whos_next
        // 36 + 1 <= 64
        let mut k = match self.whos_next {
            Player::XS => 0,
            Player::OT => 1 << 63,
        };

        for (i, p) in self.board.places.iter().enumerate() {
            let bits = p.to_bits() as u64;
            k |= bits << (4 * i);
        }
        k
    }

    pub fn from_bits(key: u64) -> Game {
        let whos_next = if (key & (1 << 63)) == 0 {
            Player::XS
        } else {
            Player::OT
        };
        let mut places = [Place::Empty; 9];
        for i in 0..9 {
            let bits = ((key >> (4 * i)) & 0b1111) as u8;
            places[i] = Place::from_bits(bits);
        }
        Game {
            whos_next,
            board: Board { places },
        }
    }

}