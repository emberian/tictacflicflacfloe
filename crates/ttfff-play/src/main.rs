//! Tic Tac Flic Flac Floe - an egui 0.33 implementation

use eframe::egui;
use egui::{vec2, Color32, Rect, Shape, Stroke};

use ttfff_explore::*;

//=============================================================================
//  Playstation-like Colors
//=============================================================================

/// Blue for 'X'
const PS_BLUE: Color32 = Color32::from_rgb(102, 153, 255);
/// Red for 'O'
const PS_RED: Color32 = Color32::from_rgb(255, 102, 102);
/// Green for 'T'
const PS_GREEN: Color32 = Color32::from_rgb(102, 255, 102);
/// Pink for 'S'
const PS_PINK: Color32 = Color32::from_rgb(255, 102, 153);

fn sym_color(s: Sym) -> Color32 {
    match s {
        Sym::X => PS_BLUE,
        Sym::O => PS_RED,
        Sym::T => PS_GREEN,
        Sym::S => PS_PINK,
    }
}

//=============================================================================
//  Symbol Shape Drawing
//=============================================================================

/// Rendering style for a symbol
#[derive(Clone, Copy, Debug, PartialEq)]
enum SymbolStyle {
    /// Not yet placed - very dim
    Available,
    /// Placed by current player - bright
    PlacedByCurrentPlayer,
    /// Placed by opponent - medium brightness
    PlacedByOpponent,
    /// Available and hovered - highlighted
    AvailableHovered,
}

fn draw_sym(s: Sym, rect: Rect, ui: &mut egui::Ui, style: SymbolStyle) {
    let base_color = sym_color(s);
    let stroke_width = rect.width() * 0.12;
    
    let (color, alpha_multiplier) = match style {
        SymbolStyle::Available => (base_color, 0.15),
        SymbolStyle::PlacedByCurrentPlayer => (base_color, 1.0),
        SymbolStyle::PlacedByOpponent => (base_color, 0.7),
        SymbolStyle::AvailableHovered => (base_color, 0.5),
    };
    
    let final_color = Color32::from_rgba_premultiplied(
        (color.r() as f32 * alpha_multiplier) as u8,
        (color.g() as f32 * alpha_multiplier) as u8,
        (color.b() as f32 * alpha_multiplier) as u8,
        (255.0 * alpha_multiplier) as u8,
    );
    
    let stroke = Stroke::new(stroke_width, final_color);
    let shadow_offset = vec2(stroke_width * 0.3, stroke_width * 0.3);

    let shapes = match s {
        Sym::X => {
            let r = rect.shrink(stroke_width);
            vec![
                Shape::line_segment([r.left_top(), r.right_bottom()], stroke),
                Shape::line_segment([r.right_top(), r.left_bottom()], stroke),
            ]
        }
        Sym::O => {
            let r = rect.shrink(stroke_width * 1.5);
            vec![Shape::circle_stroke(r.center(), r.width() / 2.0, stroke)]
        }
        Sym::T => {
            let r = rect.shrink(stroke_width * 1.5);
            let top = r.center_top();
            let left = r.left_bottom();
            let right = r.right_bottom();
            vec![Shape::closed_line(vec![top, left, right], stroke)]
        }
        Sym::S => {
            let r = rect.shrink(stroke_width * 1.5);
            vec![Shape::rect_stroke(r, 0.0, stroke, egui::epaint::StrokeKind::Outside)]
        }
    };

    for shape in shapes {
        ui.painter().add(shape);
    }
}

//=============================================================================
//  EGUI APPLICATION LOGIC
//=============================================================================

/// Main application state for the egui frontend.
struct TtfffApp {
    /// The current state of the game board and whose turn it is.
    game: Game,
    /// The outcome of the game. `None` if ongoing, `Some(winner)` if concluded.
    outcome: Option<GameOutcome>,
    /// A history of game states, used for the "Undo" feature.
    history: Vec<Game>,
    /// All possible moves for the current player.
    possible_moves: Vec<GameMove>,
    /// The move the user is currently hovering over.
    hovered_move: Option<GameMove>,
    /// The last move that was made (for highlighting).
    last_move: Option<GameMove>,
    /// Move counter.
    move_count: usize,
    /// Game statistics.
    stats: GameStats,
    /// Winning lines to highlight at game end.
    winning_lines: Vec<WinningLine>,
    /// AI opponent configuration (None = human vs human).
    ai_player: Option<(Player, AiDifficulty)>,
    /// Whether AI is currently thinking.
    ai_thinking: bool,
}

/// Track game statistics across multiple games.
#[derive(Clone, Copy, Debug, Default)]
struct GameStats {
    xs_wins: usize,
    ot_wins: usize,
    draws: usize,
    games_played: usize,
}

/// Represents the result of a completed game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GameOutcome {
    Winner(Player),
    Draw,
}

/// A winning line on the board
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WinningLine {
    indices: [usize; 3],
    symbol: Sym,
    player: Player,
}

/// AI difficulty levels
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AiDifficulty {
    Random,
    Smart,
}

impl Default for TtfffApp {
    fn default() -> Self {
        let initial_game = Game::XS_STARTS;
        let mut possible_moves = Vec::new();
        ttfff_explore::possible_moves(&initial_game, &mut possible_moves);
        Self {
            game: initial_game,
            outcome: None,
            history: vec![initial_game],
            possible_moves,
            hovered_move: None,
            last_move: None,
            move_count: 0,
            stats: GameStats::default(),
            winning_lines: Vec::new(),
            ai_player: None,
            ai_thinking: false,
        }
    }
}

impl TtfffApp {
    /// Resets the application to a new game state.
    fn new_game(&mut self) {
        let stats = self.stats;
        let ai_player = self.ai_player;
        *self = Self::default();
        self.stats = stats;
        self.ai_player = ai_player;
    }

    /// Reverts the game to the previous state.
    fn undo_move(&mut self) {
        if self.history.len() > 1 {
            self.history.pop();
            self.game = *self.history.last().unwrap();
            self.outcome = None;
            self.last_move = None;
            self.move_count = self.move_count.saturating_sub(1);
            self.update_possible_moves();
        }
    }

    /// Handles a click on a potential move.
    fn handle_move_click(&mut self, game_move: GameMove) {
        if self.outcome.is_some() {
            return; // Game is over
        }

        if let Some(next_game_state) = self.game.try_move(game_move) {
            // Move was successful
            self.game = next_game_state;
            self.history.push(next_game_state);
            self.last_move = Some(game_move);
            self.move_count += 1;
            self.update_possible_moves();
            self.check_for_game_end();
        }
    }

    /// Recalculates the possible moves for the current game state.
    fn update_possible_moves(&mut self) {
        self.possible_moves.clear();
        ttfff_explore::possible_moves(&self.game, &mut self.possible_moves);
        self.hovered_move = None;
    }

    /// Detects all winning lines on the board.
    fn detect_winning_lines(&self) -> Vec<WinningLine> {
        let mut lines = Vec::new();
        for sym in [Sym::X, Sym::O, Sym::T, Sym::S] {
            for line in Game::LINES {
                if line.iter().all(|&idx| self.game.board.places[idx].contains(sym)) {
                    lines.push(WinningLine {
                        indices: line,
                        symbol: sym,
                        player: sym.player(),
                    });
                }
            }
        }
        lines
    }

    /// Checks if the game has ended (i.e., no more valid moves).
    fn check_for_game_end(&mut self) {
        if self.possible_moves.is_empty() {
            let outcome = match self.game.score().winner() {
                Some(player) => GameOutcome::Winner(player),
                None => GameOutcome::Draw,
            };
            self.outcome = Some(outcome);
            self.winning_lines = self.detect_winning_lines();
            
            // Update statistics
            self.stats.games_played += 1;
            match outcome {
                GameOutcome::Winner(Player::XS) => self.stats.xs_wins += 1,
                GameOutcome::Winner(Player::OT) => self.stats.ot_wins += 1,
                GameOutcome::Draw => self.stats.draws += 1,
            }
        }
    }
    
    /// Make an AI move if it's the AI's turn.
    fn try_ai_move(&mut self) {
        if self.outcome.is_some() || self.ai_thinking {
            return;
        }
        
        if let Some((ai_player, difficulty)) = self.ai_player {
            if self.game.whos_next == ai_player && !self.possible_moves.is_empty() {
                self.ai_thinking = true;
                
                let chosen_move = match difficulty {
                    AiDifficulty::Random => {
                        // Pick a random move
                        let idx = (self.move_count * 7919) % self.possible_moves.len();
                        self.possible_moves[idx]
                    }
                    AiDifficulty::Smart => {
                        // Simple heuristic: prefer moves that create winning lines
                        // or block opponent's winning lines
                        self.choose_smart_move()
                    }
                };
                
                self.handle_move_click(chosen_move);
                self.ai_thinking = false;
            }
        }
    }
    
    /// Choose a smart move using simple heuristics.
    fn choose_smart_move(&self) -> GameMove {
        let mut best_move = self.possible_moves[0];
        let mut best_score = i32::MIN;
        
        for &game_move in &self.possible_moves {
            if let Some(next_game) = self.game.try_move(game_move) {
                let mut score = 0;
                
                // Evaluate the position
                let game_score = next_game.score();
                let my_score = match self.game.whos_next {
                    Player::XS => game_score.xs as i32,
                    Player::OT => game_score.ot as i32,
                };
                let opp_score = match self.game.whos_next {
                    Player::XS => game_score.ot as i32,
                    Player::OT => game_score.xs as i32,
                };
                
                score += my_score * 100 - opp_score * 80;
                
                // Prefer center and corners
                if game_move.place.0 == 4 { score += 10; }
                if [0, 2, 6, 8].contains(&game_move.place.0) { score += 5; }
                
                if score > best_score {
                    best_score = score;
                    best_move = game_move;
                }
            }
        }
        
        best_move
    }
}

impl eframe::App for TtfffApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // --- Top Panel: Title and Status ---
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(10.0);
                ui.heading("Tic Tac Flic Flac Floe");
                ui.add_space(5.0);

                if let Some(outcome) = self.outcome {
                    let message = match outcome {
                        GameOutcome::Winner(Player::XS) => "Player XS Wins!",
                        GameOutcome::Winner(Player::OT) => "Player OT Wins!",
                        GameOutcome::Draw => "It's a Draw!",
                    };
                    ui.label(
                        egui::RichText::new(message)
                            .color(egui::Color32::GREEN)
                            .size(24.0),
                    );
                } else {
                    let (player_text, color) = match self.game.whos_next {
                        Player::XS => ("XS", PS_BLUE),
                        Player::OT => ("OT", PS_RED),
                    };
                    ui.label(
                        egui::RichText::new(format!("Turn: Player {}", player_text))
                            .color(color)
                            .size(18.0),
                    );
                }

                let score = self.game.score();
                ui.label(format!("Score: XS {} - {} OT", score.xs, score.ot));
                ui.label(format!("Moves: {}", self.move_count));
                ui.add_space(10.0);
            });
        });

        // --- Side Panel: Controls ---
        egui::SidePanel::left("controls").show(ctx, |ui| {
            ui.add_space(10.0);
            ui.heading("Controls");
            ui.separator();
            ui.add_space(10.0);

            if ui.button("New Game").clicked() {
                self.new_game();
            }

            if ui
                .add_enabled(self.history.len() > 1, egui::Button::new("Undo Move"))
                .clicked()
            {
                self.undo_move();
            }

            ui.add_space(20.0);
            ui.separator();
            ui.heading("AI Opponent");
            
            ui.horizontal(|ui| {
                if ui.radio(self.ai_player.is_none(), "Off").clicked() {
                    self.ai_player = None;
                }
                if ui.radio(matches!(self.ai_player, Some((Player::OT, _))), "Play as XS").clicked() {
                    self.ai_player = Some((Player::OT, AiDifficulty::Smart));
                    self.new_game();
                }
            });
            
            ui.horizontal(|ui| {
                if ui.radio(matches!(self.ai_player, Some((Player::XS, _))), "Play as OT").clicked() {
                    self.ai_player = Some((Player::XS, AiDifficulty::Smart));
                    self.new_game();
                }
            });
            
            if let Some((ai_player, difficulty)) = self.ai_player {
                ui.label(format!("AI plays as: {:?}", ai_player));
                ui.horizontal(|ui| {
                    if ui.radio(matches!(difficulty, AiDifficulty::Random), "Random").clicked() {
                        self.ai_player = Some((ai_player, AiDifficulty::Random));
                    }
                    if ui.radio(matches!(difficulty, AiDifficulty::Smart), "Smart").clicked() {
                        self.ai_player = Some((ai_player, AiDifficulty::Smart));
                    }
                });
            }
            
            ui.add_space(20.0);
            ui.separator();
            ui.heading("Statistics");
            ui.label(format!("Games Played: {}", self.stats.games_played));
            ui.label(format!("XS Wins: {}", self.stats.xs_wins));
            ui.label(format!("OT Wins: {}", self.stats.ot_wins));
            ui.label(format!("Draws: {}", self.stats.draws));
            
            if ui.button("Reset Stats").clicked() {
                self.stats = GameStats::default();
            }
        });

        // --- Central Panel: Game Board ---
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.centered_and_justified(|ui| {
                // Store cell centers for drawing winning lines
                let mut cell_centers = [egui::pos2(0.0, 0.0); 9];
                
                egui::Grid::new("board_grid")
                    .min_col_width(100.0)
                    .min_row_height(100.0)
                    .spacing([10.0, 10.0])
                    .show(ui, |ui| {
                        for i in 0..9 {
                            let (row, col) = (i / 3, i % 3);
                            let place_idx = PlaceIdx(i as u8);

                            let (rect, response) =
                                ui.allocate_exact_size(vec2(100.0, 100.0), egui::Sense::click());
                            
                            // Store cell center for winning line drawing
                            cell_centers[i] = rect.center();
                            
                            // Draw cell background
                            let is_last_move = self.last_move.map_or(false, |m| m.place == place_idx);
                            let cell_bg_color = if is_last_move {
                                Color32::from_rgb(60, 60, 80)
                            } else if response.hovered() {
                                Color32::from_rgb(50, 50, 60)
                            } else {
                                Color32::from_rgb(40, 40, 50)
                            };
                            ui.painter().rect_filled(rect, 5.0, cell_bg_color);
                            ui.painter().rect_stroke(rect, 5.0, Stroke::new(2.0, Color32::from_rgb(80, 80, 90)), egui::epaint::StrokeKind::Outside);

                            // --- Draw existing placed symbols and potential moves ---
                            let place = self.game.board.places[i];
                            let mut clicked_move = None;
                            
                            // Get potential moves for this cell
                            let potential_moves_for_cell: Vec<_> = if self.outcome.is_none() {
                                self.possible_moves
                                    .iter()
                                    .filter(|m| m.place == place_idx)
                                    .copied()
                                    .collect()
                            } else {
                                Vec::new()
                            };
                            
                            let is_hovered = response.hovered() && self.outcome.is_none();
                            
                            // Determine which symbols are placed and which player owns them
                            let placed_symbols: std::collections::HashSet<Sym> = match place {
                                Place::Empty => std::collections::HashSet::new(),
                                Place::OnePlaced(s) => {
                                    let mut set = std::collections::HashSet::new();
                                    set.insert(s);
                                    set
                                }
                                Place::TwoPlaced(s1, s2) => {
                                    let mut set = std::collections::HashSet::new();
                                    set.insert(s1);
                                    set.insert(s2);
                                    set
                                }
                            };
                            
                            // Which symbols can the current player place here?
                            let available_symbols: std::collections::HashSet<Sym> = 
                                potential_moves_for_cell.iter().map(|m| m.drawn_symbol).collect();
                            
                            // Draw all four symbols in a 2x2 grid
                            let inner = rect.shrink(8.0);
                            let half_width = inner.width() / 2.0;
                            let half_height = inner.height() / 2.0;
                            
                            // Define quadrants: X (top-left), O (top-right), T (bottom-left), S (bottom-right)
                            let quadrants = [
                                (Sym::X, Rect::from_min_max(
                                    inner.min,
                                    egui::pos2(inner.min.x + half_width, inner.min.y + half_height),
                                )),
                                (Sym::O, Rect::from_min_max(
                                    egui::pos2(inner.min.x + half_width, inner.min.y),
                                    egui::pos2(inner.max.x, inner.min.y + half_height),
                                )),
                                (Sym::T, Rect::from_min_max(
                                    egui::pos2(inner.min.x, inner.min.y + half_height),
                                    egui::pos2(inner.min.x + half_width, inner.max.y),
                                )),
                                (Sym::S, Rect::from_min_max(
                                    egui::pos2(inner.min.x + half_width, inner.min.y + half_height),
                                    inner.max,
                                )),
                            ];
                            
                            for (sym, quad_rect) in quadrants {
                                let style = if placed_symbols.contains(&sym) {
                                    // This symbol is already placed
                                    // Determine if it was placed by current player or opponent
                                    // If there's only one symbol, it was placed by the opponent (previous player)
                                    // If there are two, we need to check which was placed last
                                    if matches!(place, Place::TwoPlaced(_, _)) {
                                        // Cell is full, the last placed symbol is the current player's opponent's
                                        // Actually, both could be from either player, so let's just show them as placed
                                        SymbolStyle::PlacedByOpponent
                                    } else if matches!(place, Place::OnePlaced(_)) {
                                        // One symbol placed by the previous player
                                        SymbolStyle::PlacedByOpponent
                                    } else {
                                        SymbolStyle::PlacedByOpponent
                                    }
                                } else if available_symbols.contains(&sym) {
                                    // This symbol can be placed by current player
                                    if is_hovered {
                                        SymbolStyle::AvailableHovered
                                    } else {
                                        SymbolStyle::Available
                                    }
                                } else {
                                    // This symbol is not available (wrong player)
                                    SymbolStyle::Available
                                };
                                
                                draw_sym(sym, quad_rect, ui, style);
                            }
                            
                            // Handle clicks
                            if response.clicked() && !potential_moves_for_cell.is_empty() {
                                // Determine which quadrant was clicked
                                if let Some(pointer_pos) = response.interact_pointer_pos() {
                                    let rel_x = pointer_pos.x - inner.min.x;
                                    let rel_y = pointer_pos.y - inner.min.y;
                                    let clicked_sym = if rel_y < half_height {
                                        // Top half
                                        if rel_x < half_width { Sym::X } else { Sym::O }
                                    } else {
                                        // Bottom half
                                        if rel_x < half_width { Sym::T } else { Sym::S }
                                    };
                                    
                                    // Find the move that matches this symbol
                                    if let Some(&game_move) = potential_moves_for_cell.iter()
                                        .find(|m| m.drawn_symbol == clicked_sym) {
                                        clicked_move = Some(game_move);
                                    }
                                }
                            }

                            if let Some(game_move) = clicked_move {
                                self.handle_move_click(game_move);
                            }

                            if col == 2 {
                                ui.end_row();
                            }
                        }
                    });
                    
                // Draw winning lines on top of everything
                if !self.winning_lines.is_empty() {
                    for winning_line in &self.winning_lines {
                        let color = sym_color(winning_line.symbol);
                        let bright_color = Color32::from_rgba_premultiplied(
                            (color.r() as f32 * 0.9) as u8,
                            (color.g() as f32 * 0.9) as u8,
                            (color.b() as f32 * 0.9) as u8,
                            255,
                        );
                        
                        let start_pos = cell_centers[winning_line.indices[0]];
                        let end_pos = cell_centers[winning_line.indices[2]];
                        
                        // Draw thick line
                        ui.painter().line_segment(
                            [start_pos, end_pos],
                            Stroke::new(6.0, bright_color),
                        );
                        
                        // Draw glow effect
                        ui.painter().line_segment(
                            [start_pos, end_pos],
                            Stroke::new(12.0, Color32::from_rgba_premultiplied(
                                color.r(),
                                color.g(),
                                color.b(),
                                60,
                            )),
                        );
                    }
                }
            });
        });

        // Try to make an AI move if it's AI's turn
        if self.outcome.is_none() {
            self.try_ai_move();
        }

        // Reset hovered move if mouse is not over any cell
        if !ctx.input(|i| i.pointer.has_pointer()) {
            self.hovered_move = None;
        }
    }
}

//=============================================================================
//  MAIN FUNCTION
//=============================================================================

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 500.0])
            .with_min_inner_size([400.0, 400.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Tic Tac Flic Flac Floe",
        native_options,
        Box::new(|_cc| Ok(Box::<TtfffApp>::default())),
    )
}
