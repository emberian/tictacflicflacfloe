#![no_std]

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
    pub fn symbols(&self) -> &[Sym] {
        match self {
            Player::XS => &[Sym::X, Sym::S],
            Player::OT => &[Sym::O, Sym::T],
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
    pub fn other(self) -> Sym {
        match self {
            X => T,
            T => X,
            O => S,
            S => O,
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
            OnePlaced(this) if other != this.excludes() => TwoPlaced(this, other),
            _ => return None,
        })
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

#[derive(Clone, Hash, Eq, PartialEq, Default)]
pub struct Board {
    pub places: [Place; 9],
}

#[derive(Clone, Hash, Eq, PartialEq)]
pub struct Game {
    pub whos_next: Player,
    pub board: Board,
}

#[derive(Clone, Hash, Eq, PartialEq)]
pub struct GameMove {
    pub who: Player,
    pub place: PlaceIdx,
    pub drawn_symbol: Sym,
}
impl Game {
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
}


// step 2: optimize representation for tree exploration
// step 3: write enumerator
// step 4: analysis