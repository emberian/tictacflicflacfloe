use rustc_hash::FxHashSet as HashSet;

pub use ttfff_logic::*;

pub fn possible_moves(game: &Game, acc: &mut Vec<GameMove>) {
    acc.reserve(18);
    for (idx, place) in game.board.places.iter().enumerate() {
        for &sym in game.whos_next.symbols() {
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

pub fn enumerate_from(stack: &mut Vec<Game>, games: &mut HashSet<Game>, moves: &mut Vec<GameMove>) {
    moves.clear();
    while let Some(g) = stack.pop() {
        if !games.insert(g.clone()) {
            continue;
        }
        possible_moves(&g, moves);
        for m in moves.drain(..) {
            if let Some(ng) = g.try_move(m) {
                stack.push(ng);
            }
        }
    }
}

pub fn enumerate(game: Game) -> HashSet<Game> {
    let mut initial_states = vec![game];
    let mut games = HashSet::default();
    let mut moves = Vec::with_capacity(18);
    enumerate_from(&mut initial_states, &mut games, &mut moves);

    games
}

pub fn enumerate_xs() -> HashSet<Game> {
    enumerate(Game::XS_STARTS)
}

pub fn enumerate_ot() -> HashSet<Game> {
    enumerate(Game::OT_STARTS)
}

pub fn enumerate_all() -> HashSet<Game> {
    let mut initial_states = vec![Game::XS_STARTS, Game::OT_STARTS];
    let mut games = HashSet::default();
    let mut moves = Vec::with_capacity(36);
    enumerate_from(&mut initial_states, &mut games, &mut moves);

    games
}

pub fn terminal_states(all_games: &HashSet<Game>) -> HashSet<Game> {
    let mut terminals = HashSet::default();
    for g in all_games.iter() {
        let mut moves = Vec::with_capacity(18);
        possible_moves(g, &mut moves);
        if moves.is_empty() {
            terminals.insert(g.clone());
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
    pub all_xs_wins: usize,
    pub all_ot_wins: usize,
    pub xs_wins_starting: usize,
    pub xs_loses_starting: usize,
    pub ot_wins_starting: usize,
    pub ot_loses_starting: usize,
    pub states: usize,
    pub terminal: usize,
    pub incomplete: HashSet<Game>,
}

pub fn report() -> Report {
    let all_games = enumerate_all();
    let all_terminal = terminal_states(&all_games);

    let mut xs_wins = 0;
    let mut ot_wins = 0;
    let mut incomplete = HashSet::default();

    for state in &all_terminal {
        if !board_is_complete(&state.board) {
            incomplete.insert(state.clone());
        }

        match state.whos_next {
            Player::XS => ot_wins += 1,
            Player::OT => xs_wins += 1,
        }
    }

    let xs_starts = enumerate_xs();
    let xs_terminal = terminal_states(&xs_starts);
    let mut xs_wins_starting = 0;
    let mut xs_loses_starting = 0;
    for state in xs_terminal.iter() {
        match state.whos_next {
            Player::XS => xs_loses_starting += 1,
            Player::OT => xs_wins_starting += 1,
        }
    }

    let ot_starts = enumerate_ot();
    let ot_terminal = terminal_states(&ot_starts);
    let mut ot_wins_starting = 0;
    let mut ot_loses_starting = 0;
    for state in ot_terminal.iter() {
        match state.whos_next {
            Player::XS => ot_wins_starting += 1,
            Player::OT => ot_loses_starting += 1,
        }
    }

    Report {
        all_xs_wins: xs_wins,
        all_ot_wins: ot_wins,
        xs_wins_starting,
        xs_loses_starting,
        ot_wins_starting,
        ot_loses_starting,
        states: all_games.len(),
        terminal: all_terminal.len(),
        incomplete: all_terminal,
    }
}

impl std::fmt::Display for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "There are {} distinct game states.", self.states)?;
        writeln!(f, "There are {} terminal game states.", self.terminal)?;
        writeln!(f, "XS wins overall: {}", self.all_xs_wins)?;
        writeln!(f, "OT wins overall: {}", self.all_ot_wins)?;
        writeln!(f, "XS wins when starting: {}", self.xs_wins_starting)?;
        writeln!(f, "XS loses when starting: {}", self.xs_loses_starting)?;
        writeln!(f, "OT wins when starting: {}", self.ot_wins_starting)?;
        writeln!(f, "OT loses when starting: {}", self.ot_loses_starting)?;
        for state in self.incomplete.iter() {
            writeln!(f, "Incomplete terminal state: {:?}", state)?;
        }
        Ok(())
    }
}
