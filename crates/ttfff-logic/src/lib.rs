#![no_std]

use core::cmp::{max, min};

mod bits;

// step 1: basic implement of game

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum Player {
    XS,
    OT,
}
impl Player {
    pub fn other(&self) -> Player {
        match self {
            Player::XS => Player::OT,
            Player::OT => Player::XS,
        }
    }
    pub fn symbols(&self) -> [Sym; 2] {
        match self {
            Player::XS => [X, S],
            Player::OT => [O, T],
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum Sym {
    X,
    O,
    T,
    S,
}

use Sym::*;

impl Sym {
    pub fn excludes(self) -> Sym {
        match self {
            X => O,
            O => X,
            T => S,
            S => T,
        }
    }
    pub fn player(self) -> Player {
        match self {
            X | S => Player::XS,
            O | T => Player::OT,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub enum Place {
    #[default]
    Empty,
    OnePlaced(Sym),
    TwoPlaced(Sym, Sym),
}
use Place::*;

impl Place {
    pub fn combine(self, other: Sym) -> Option<Place> {
        Some(match self {
            Empty => OnePlaced(other),
            OnePlaced(this) if this != other && other != this.excludes() => {
                // note: this forgets the order of placement in favor of
                // a canonical representation.
                TwoPlaced(min(this, other), max(this, other))
            }
            _ => return None,
        })
    }

    pub fn contains(&self, sym: Sym) -> bool {
        match self {
            Empty => false,
            OnePlaced(s) => *s == sym,
            TwoPlaced(s1, s2) => *s1 == sym || *s2 == sym,
        }
    }

    pub fn transpose(self) -> Place {
        match self {
            Empty => Empty,
            OnePlaced(s) => OnePlaced(s.excludes()),
            TwoPlaced(s1, s2) => {
                let s1 = s1.excludes();
                let s2 = s2.excludes();
                TwoPlaced(min(s1, s2), max(s1, s2))
            }
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct PlaceIdx(pub u8);
impl TryFrom<u8> for PlaceIdx {
    type Error = ();
    fn try_from(val: u8) -> Result<PlaceIdx, Self::Error> {
        if val >= 9 { Err(()) } else { Ok(PlaceIdx(val)) }
    }
}
impl PlaceIdx {
    pub fn index(&self) -> usize {
        self.0 as usize
    }
}

#[derive(Copy, Clone, Hash, Eq, PartialEq, Default, Debug)]
pub struct Board {
    pub places: [Place; 9],
}

impl Board {
    pub fn transpose(&self) -> Board {
        Board {
            places: self.places.map(|p| p.transpose()),
        }
    }
}

// A game is only 19 bytes.
// But game.to_bits() packs it into u64.
#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct Game {
    pub whos_next: Player,
    pub board: Board,
}

impl Game {
    pub fn transpose(&self) -> Game {
        Game {
            whos_next: self.whos_next.other(),
            board: self.board.transpose(),
        }
    }
}

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct GameMove {
    pub who: Player,
    pub place: PlaceIdx,
    pub drawn_symbol: Sym,
}

pub struct GameScore {
    pub xs: u8,
    pub ot: u8,
}

impl GameScore {
    pub fn winner(&self) -> Option<Player> {
        if self.xs > self.ot {
            Some(Player::XS)
        } else if self.ot > self.xs {
            Some(Player::OT)
        } else {
            None
        }
    }
}

impl Game {
    pub const XS_STARTS: Game = Game {
        whos_next: Player::XS,
        board: Board {
            places: [Place::Empty; 9],
        },
    };
    pub const OT_STARTS: Game = Game {
        whos_next: Player::OT,
        board: Board {
            places: [Place::Empty; 9],
        },
    };

    pub fn from_history(moves: &[GameMove]) -> Option<Game> {
        if moves.is_empty() {
            return None;
        }
        let who_first = moves[0].who;
        let mut g = if who_first == Player::XS {
            Game::XS_STARTS
        } else {
            Game::OT_STARTS
        };

        for &m in moves {
            match g.try_move(m) {
                Some(ng) => g = ng,
                None => return None,
            }
        }

        Some(g)
    }

    // spec: try_move succeeds when m.who is self.whos_next
    // and their chosen place symbol compatibly combines
    // with existing places.
    pub fn try_move(&self, m: GameMove) -> Option<Game> {
        if m.who != self.whos_next {
            return None;
        }
        let mut nb = self.board.clone();
        match nb.places[m.place.index()].combine(m.drawn_symbol) {
            Some(new_pl) => {
                nb.places[m.place.index()] = new_pl;
                return Some(Game {
                    whos_next: self.whos_next.other(),
                    board: nb,
                });
            }
            None => return None,
        }
    }

    pub const LINES: [[usize; 3]; 8] = [
        // horizontals
        [0, 1, 2],
        [3, 4, 5],
        [6, 7, 8],
        // verticals
        [0, 3, 6],
        [1, 4, 7],
        [2, 5, 8],
        // diagonals
        [0, 4, 8],
        [2, 4, 6],
    ];

    pub fn score(&self) -> GameScore {
        // detect three-in-a-row for each symbol.
        // player gets one point per three-in-a-row.
        let mut scores = [0; 2];

        for sym in [X, O, T, S] {
            for line in Self::LINES {
                if line
                    .iter()
                    .all(|&place_idx| self.board.places[place_idx].contains(sym))
                {
                    scores[sym.player() as usize] += 1;
                }
            }
        }

        GameScore {
            xs: scores[Player::XS as usize],
            ot: scores[Player::OT as usize],
        }
    }
}

// step 2: optimize representation for tree exploration
// step 3: write enumerator
// step 4: analysis
