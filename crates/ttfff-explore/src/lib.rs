use rustc_hash::FxHashSet as HashSet;

pub use ttfff_logic::*;

pub const PERMS: [[usize; 9]; 8] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8], // id
    [6, 3, 0, 7, 4, 1, 8, 5, 2], // rotations
    [8, 7, 6, 5, 4, 3, 2, 1, 0],
    [2, 5, 8, 1, 4, 7, 0, 3, 6],
    [2, 1, 0, 5, 4, 3, 8, 7, 6], // reflections
    [6, 7, 8, 3, 4, 5, 0, 1, 2],
    [0, 3, 6, 1, 4, 7, 2, 5, 8],
    [8, 5, 2, 7, 4, 1, 6, 3, 0],
];

pub fn canonicalize(board: &Board) -> Board {
    let mut best = board.to_bits();
    for perm in PERMS.iter() {
        let mut permuted_places = [Place::Empty; 9];
        for (i, &src_idx) in perm.iter().enumerate() {
            permuted_places[i] = board.places[src_idx];
        }
        let permuted_board = Board {
            places: permuted_places,
        };
        let pb_bits = permuted_board.to_bits();
        if pb_bits < best {
            best = pb_bits;
        }
    }
    Board::from_bits(best)
}

pub fn canonicalize_game(game: &Game) -> Game {
    let raw = Game {
        whos_next: game.whos_next,
        board: canonicalize(&game.board),
    };
    let transposed = Game {
        whos_next: game.whos_next.other(),
        board: canonicalize(&game.board.transpose()),
    };
    if raw.to_bits() < transposed.to_bits() {
        raw
    } else {
        transposed
    }
}

pub fn possible_moves(game: &Game, acc: &mut Vec<GameMove>) {
    for (idx, place) in game.board.places.iter().enumerate() {
        for sym in game.whos_next.symbols() {
            if let Some(_) = place.combine(sym) {
                acc.push(GameMove {
                    who: game.whos_next,
                    place: PlaceIdx(idx as u8),
                    drawn_symbol: sym,
                });
            }
        }
    }
}

pub fn enumerate_from(stack: &mut Vec<u64>, games: &mut HashSet<u64>, moves: &mut Vec<GameMove>) {
    moves.clear();
    while let Some(g) = stack.pop() {
        if !games.insert(g) {
            continue;
        }
        let g = Game::from_bits(g);
        possible_moves(&g, moves);
        for m in moves.drain(..) {
            if let Some(ng) = g.try_move(m) {
                stack.push(canonicalize_game(&ng).to_bits());
            }
        }
    }
}

pub fn enumerate(game: Game) -> HashSet<Game> {
    let mut initial_states = vec![game.to_bits()];
    let mut games = HashSet::default();
    let mut moves = Vec::with_capacity(18);
    enumerate_from(&mut initial_states, &mut games, &mut moves);

    games.into_iter().map(Game::from_bits).collect()
}

pub fn enumerate_xs() -> HashSet<Game> {
    enumerate(Game::XS_STARTS)
}

pub fn enumerate_ot() -> HashSet<Game> {
    enumerate(Game::OT_STARTS)
}

pub fn enumerate_all() -> HashSet<Game> {
    let mut initial_states = vec![Game::XS_STARTS.to_bits(), Game::OT_STARTS.to_bits()];
    let mut games = HashSet::default();
    let mut moves = Vec::with_capacity(18);
    enumerate_from(&mut initial_states, &mut games, &mut moves);

    games.into_iter().map(Game::from_bits).collect()
}

pub fn terminal_states(all_games: &HashSet<Game>) -> HashSet<Game> {
    let mut terminals = HashSet::default();
    let mut moves = Vec::with_capacity(18);
    for g in all_games {
        moves.clear();
        possible_moves(g, &mut moves);
        if moves.is_empty() {
            terminals.insert(*g);
        }
    }
    terminals
}

pub fn board_is_complete(board: &Board) -> bool {
    board
        .places
        .iter()
        .all(|o| matches!(o, Place::TwoPlaced(_, _)))
}

pub struct Report {
    pub xs_wins: usize,
    pub ot_wins: usize,
    pub draws: usize,
    pub states: usize,
    pub terminal: usize,
    pub incomplete: HashSet<Game>,
}

pub fn report() -> Report {
    let all_games = enumerate_all();
    let all_terminal = terminal_states(&all_games);

    let mut xs_wins = 0;
    let mut ot_wins = 0;
    let mut draws = 0;
    let mut incomplete = HashSet::default();

    for state in &all_terminal {
        if !board_is_complete(&state.board) {
            incomplete.insert(*state);
        }

        match state.score().winner() {
            None => {
                draws += 1;
            }
            Some(Player::XS) => xs_wins += 1,
            Some(Player::OT) => ot_wins += 1,
        }
    }

    Report {
        xs_wins: xs_wins,
        ot_wins: ot_wins,
        draws,
        states: all_games.len(),
        terminal: all_terminal.len(),
        incomplete: incomplete,
    }
}

impl std::fmt::Display for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "There are {} distinct game states.", self.states)?;
        writeln!(f, "There are {} terminal game states.", self.terminal)?;
        writeln!(f, "XS wins: {}", self.xs_wins)?;
        writeln!(f, "OT wins: {}", self.ot_wins)?;
        writeln!(f, "Draws: {}", self.draws)?;
        for state in self.incomplete.iter() {
            writeln!(f, "Incomplete terminal state: {:?}", state)?;
        }
        Ok(())
    }
}
