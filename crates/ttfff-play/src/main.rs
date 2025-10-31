//! Tic Tac Flic Flac Floe - an egui 0.33 implementation

use eframe::egui;
use egui::{vec2, Color32, Rect, Shape, Stroke};
use std::time::Instant;

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

/// Line rendering style
#[derive(Clone, Copy, Debug, PartialEq)]
enum LineStyle {
    /// Winning line - game over, bright and solid
    FinalWinning,
    /// Active winning line during play - semi-transparent
    ActiveWinning,
    /// Potential winning line on hover - dashed/subtle
    PotentialWinning,
}

/// Visual theme
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Theme {
    Dark,
    Light,
}

/// UI Tab selection
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UiTab {
    Game,
    Analysis,
    Settings,
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

/// Draw a winning line with specified style
fn draw_winning_line(ui: &mut egui::Ui, line: &WinningLine, cell_centers: &[egui::Pos2; 9], style: LineStyle) {
    let color = sym_color(line.symbol);
    
    let (stroke_width, alpha, dashed) = match style {
        LineStyle::FinalWinning => (6.0, 1.0, false),
        LineStyle::ActiveWinning => (4.0, 0.5, false),
        LineStyle::PotentialWinning => (3.0, 0.35, true),
    };
    
    let line_color = Color32::from_rgba_premultiplied(
        (color.r() as f32 * alpha) as u8,
        (color.g() as f32 * alpha) as u8,
        (color.b() as f32 * alpha) as u8,
        (255.0 * alpha) as u8,
    );
    
    let start_pos = cell_centers[line.indices[0]];
    let end_pos = cell_centers[line.indices[2]];
    
    if dashed {
        // Draw dashed line
        let dash_length = 10.0;
        let gap_length = 5.0;
        let total = dash_length + gap_length;
        let dir = (end_pos - start_pos).normalized();
        let len = (end_pos - start_pos).length();
        let num_dashes = (len / total) as usize;
        
        for i in 0..num_dashes {
            let start = start_pos + dir * (i as f32 * total);
            let end = start + dir * dash_length;
            ui.painter().line_segment([start, end], Stroke::new(stroke_width, line_color));
        }
    } else {
        ui.painter().line_segment([start_pos, end_pos], Stroke::new(stroke_width, line_color));
        
        // Add glow for final winning lines
        if matches!(style, LineStyle::FinalWinning) {
            ui.painter().line_segment(
                [start_pos, end_pos],
                Stroke::new(stroke_width * 2.0, Color32::from_rgba_premultiplied(
                    color.r(),
                    color.g(),
                    color.b(),
                    60,
                )),
            );
        }
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
    /// Game mode
    game_mode: GameMode,
    /// Move history with timestamps
    move_history: Vec<MoveEntry>,
    /// Game start time
    game_start: Instant,
    /// Current active tab
    active_tab: UiTab,
    /// Show hints
    show_hints: bool,
    /// Visual theme
    theme: Theme,
    /// Animation time for smooth transitions
    animation_time: f64,
    /// AI thinking delay for visibility
    ai_delay: f64,
    /// Last frame time for animations
    last_frame_time: Option<Instant>,
    /// Active lines (formed during game)
    active_lines: Vec<WinningLine>,
    /// Potential lines (on hover)
    potential_lines: Vec<WinningLine>,
    /// Sound effects enabled
    sound_effects: bool,
    /// Show coordinates on board
    show_coordinates: bool,
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
    Perfect,
}

/// Game mode
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GameMode {
    HumanVsHuman,
    HumanVsAi { human_player: Player, ai_difficulty: AiDifficulty },
    AiVsAi { xs_difficulty: AiDifficulty, ot_difficulty: AiDifficulty },
}

/// Move history entry
#[derive(Clone, Copy, Debug)]
struct MoveEntry {
    game_move: GameMove,
    timestamp: f64,
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
            game_mode: GameMode::HumanVsHuman,
            move_history: Vec::new(),
            game_start: Instant::now(),
            active_tab: UiTab::Game,
            show_hints: false,
            theme: Theme::Dark,
            animation_time: 0.0,
            ai_delay: 0.0,
            last_frame_time: None,
            active_lines: Vec::new(),
            potential_lines: Vec::new(),
            sound_effects: false,
            show_coordinates: false,
        }
    }
}

impl TtfffApp {
    /// Resets the application to a new game state.
    fn new_game(&mut self) {
        let stats = self.stats;
        let game_mode = self.game_mode;
        let show_hints = self.show_hints;
        let theme = self.theme;
        let active_tab = self.active_tab;
        let sound_effects = self.sound_effects;
        let show_coordinates = self.show_coordinates;
        *self = Self::default();
        self.stats = stats;
        self.game_mode = game_mode;
        self.show_hints = show_hints;
        self.theme = theme;
        self.active_tab = active_tab;
        self.sound_effects = sound_effects;
        self.show_coordinates = show_coordinates;
        self.game_start = Instant::now();
    }
    
    /// Set game mode and restart
    fn set_game_mode(&mut self, mode: GameMode) {
        self.game_mode = mode;
        self.new_game();
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
            
            // Add to move history with timestamp
            let elapsed = self.game_start.elapsed().as_secs_f64();
            self.move_history.push(MoveEntry {
                game_move,
                timestamp: elapsed,
            });
            
            self.update_possible_moves();
            self.update_active_lines();
            self.check_for_game_end();
        }
    }

    /// Recalculates the possible moves for the current game state.
    fn update_possible_moves(&mut self) {
        self.possible_moves.clear();
        ttfff_explore::possible_moves(&self.game, &mut self.possible_moves);
        self.hovered_move = None;
    }
    
    /// Update active winning lines during gameplay
    fn update_active_lines(&mut self) {
        self.active_lines = self.detect_winning_lines();
    }
    
    /// Update potential lines when hovering over a move
    fn update_potential_lines(&mut self, game_move: GameMove) {
        self.potential_lines.clear();
        
        if let Some(next_game) = self.game.try_move(game_move) {
            // Check what new lines would form
            let new_lines = Self::detect_lines_for_game(&next_game);
            let current_lines = Self::detect_lines_for_game(&self.game);
            
            // Find lines that are new
            for line in new_lines {
                if !current_lines.iter().any(|l| l.indices == line.indices && l.symbol == line.symbol) {
                    self.potential_lines.push(line);
                }
            }
        }
    }
    
    fn detect_lines_for_game(game: &Game) -> Vec<WinningLine> {
        let mut lines = Vec::new();
        for sym in [Sym::X, Sym::O, Sym::T, Sym::S] {
            for line in Game::LINES {
                if line.iter().all(|&idx| game.board.places[idx].contains(sym)) {
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

    /// Detects all winning lines on the board.
    fn detect_winning_lines(&self) -> Vec<WinningLine> {
        Self::detect_lines_for_game(&self.game)
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
    fn try_ai_move(&mut self, dt: f64) {
        if self.outcome.is_some() {
            return;
        }
        
        // Handle AI thinking delay for visibility
        if self.ai_thinking {
            self.ai_delay -= dt;
            if self.ai_delay > 0.0 {
                return;
            }
            self.ai_thinking = false;
        }
        
        let should_move = match self.game_mode {
            GameMode::HumanVsHuman => false,
            GameMode::HumanVsAi { human_player, ai_difficulty: _ } => {
                self.game.whos_next != human_player
            }
            GameMode::AiVsAi { .. } => true,
        };
        
        if should_move && !self.possible_moves.is_empty() && !self.ai_thinking {
            self.ai_thinking = true;
            self.ai_delay = 0.3; // 300ms delay for visibility
            
            let difficulty = match self.game_mode {
                GameMode::HumanVsAi { ai_difficulty, .. } => ai_difficulty,
                GameMode::AiVsAi { xs_difficulty, ot_difficulty } => {
                    if self.game.whos_next == Player::XS {
                        xs_difficulty
                    } else {
                        ot_difficulty
                    }
                }
                _ => AiDifficulty::Random,
            };
            
            let chosen_move = match difficulty {
                AiDifficulty::Random => {
                    let idx = (self.move_count * 7919) % self.possible_moves.len();
                    self.possible_moves[idx]
                }
                AiDifficulty::Smart => self.choose_smart_move(),
                AiDifficulty::Perfect => self.choose_perfect_move(),
            };
            
            self.handle_move_click(chosen_move);
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
                
                // Check if this creates a new winning line
                let new_lines = Self::detect_lines_for_game(&next_game);
                let current_lines = Self::detect_lines_for_game(&self.game);
                let creates_line = new_lines.len() > current_lines.len();
                if creates_line { score += 50; }
                
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
    
    /// Choose the perfect move using minimax
    fn choose_perfect_move(&self) -> GameMove {
        // Use a simple minimax with limited depth for perfect play
        let depth = 4; // Look ahead 4 moves
        self.minimax_search(depth).unwrap_or_else(|| self.choose_smart_move())
    }
    
    /// Minimax search for best move
    fn minimax_search(&self, max_depth: usize) -> Option<GameMove> {
        if self.possible_moves.is_empty() {
            return None;
        }
        
        let mut best_move = self.possible_moves[0];
        let mut best_score = i32::MIN;
        
        for &game_move in &self.possible_moves {
            if let Some(next_game) = self.game.try_move(game_move) {
                let score = -self.minimax(&next_game, max_depth - 1, i32::MIN, i32::MAX, false);
                if score > best_score {
                    best_score = score;
                    best_move = game_move;
                }
            }
        }
        
        Some(best_move)
    }
    
    /// Minimax with alpha-beta pruning
    fn minimax(&self, game: &Game, depth: usize, mut alpha: i32, mut beta: i32, maximizing: bool) -> i32 {
        let mut moves = Vec::new();
        ttfff_explore::possible_moves(game, &mut moves);
        
        if depth == 0 || moves.is_empty() {
            let score = game.score();
            let eval = match self.game.whos_next {
                Player::XS => (score.xs as i32) * 100 - (score.ot as i32) * 100,
                Player::OT => (score.ot as i32) * 100 - (score.xs as i32) * 100,
            };
            return eval;
        }
        
        if maximizing {
            let mut max_eval = i32::MIN;
            for game_move in moves {
                if let Some(next_game) = game.try_move(game_move) {
                    let eval = self.minimax(&next_game, depth - 1, alpha, beta, false);
                    max_eval = max_eval.max(eval);
                    alpha = alpha.max(eval);
                    if beta <= alpha {
                        break;
                    }
                }
            }
            max_eval
        } else {
            let mut min_eval = i32::MAX;
            for game_move in moves {
                if let Some(next_game) = game.try_move(game_move) {
                    let eval = self.minimax(&next_game, depth - 1, alpha, beta, true);
                    min_eval = min_eval.min(eval);
                    beta = beta.min(eval);
                    if beta <= alpha {
                        break;
                    }
                }
            }
            min_eval
        }
    }
    
    /// Get best move suggestion for hints
    fn get_best_move_hint(&self) -> Option<(GameMove, String)> {
        if self.possible_moves.is_empty() {
            return None;
        }
        
        let best_move = self.choose_smart_move();
        let reason = format!("Place {} at position {}", 
            match best_move.drawn_symbol {
                Sym::X => "X",
                Sym::O => "O",
                Sym::T => "T",
                Sym::S => "S",
            },
            best_move.place.0 + 1
        );
        
        Some((best_move, reason))
    }
}

impl eframe::App for TtfffApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply theme
        let visuals = if self.theme == Theme::Dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        ctx.set_visuals(visuals);
        
        // Calculate delta time for animations
        let now = Instant::now();
        let dt = if let Some(last) = self.last_frame_time {
            now.duration_since(last).as_secs_f64()
        } else {
            0.016
        };
        self.last_frame_time = Some(now);
        self.animation_time += dt;
        
        // Handle keyboard shortcuts
        ctx.input(|i| {
            if i.key_pressed(egui::Key::N) && i.modifiers.command {
                self.new_game();
            }
            if i.key_pressed(egui::Key::Z) && i.modifiers.command {
                self.undo_move();
            }
            if i.key_pressed(egui::Key::H) {
                self.show_hints = !self.show_hints;
            }
        });
        
        // Request continuous repaint for animations and AI
        ctx.request_repaint();
        
        // --- Menu Bar ---
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            #[allow(deprecated)]
            egui::menu::bar(ui, |ui| {
                ui.menu_button("Game", |ui| {
                    if ui.button("New Game").clicked() {
                        self.new_game();
                        ui.close();
                    }
                    if ui.button("Reset Statistics").clicked() {
                        self.stats = GameStats::default();
                        ui.close();
                    }
                    ui.separator();
                    
                    ui.menu_button("Mode", |ui| {
                        if ui.radio_value(&mut self.game_mode, GameMode::HumanVsHuman, "Human vs Human").clicked() {
                            self.set_game_mode(GameMode::HumanVsHuman);
                            ui.close();
                        }
                        if ui.radio_value(&mut self.game_mode, 
                            GameMode::HumanVsAi { human_player: Player::XS, ai_difficulty: AiDifficulty::Smart },
                            "Play as XS vs AI").clicked() {
                            self.set_game_mode(GameMode::HumanVsAi { 
                                human_player: Player::XS, 
                                ai_difficulty: AiDifficulty::Smart 
                            });
                            ui.close();
                        }
                        if ui.radio_value(&mut self.game_mode,
                            GameMode::HumanVsAi { human_player: Player::OT, ai_difficulty: AiDifficulty::Smart },
                            "Play as OT vs AI").clicked() {
                            self.set_game_mode(GameMode::HumanVsAi { 
                                human_player: Player::OT, 
                                ai_difficulty: AiDifficulty::Smart 
                            });
                            ui.close();
                        }
                        if ui.radio_value(&mut self.game_mode,
                            GameMode::AiVsAi { xs_difficulty: AiDifficulty::Smart, ot_difficulty: AiDifficulty::Smart },
                            "AI vs AI").clicked() {
                            self.set_game_mode(GameMode::AiVsAi { 
                                xs_difficulty: AiDifficulty::Smart, 
                                ot_difficulty: AiDifficulty::Smart 
                            });
                            ui.close();
                        }
                    });
                });
                
                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut self.show_hints, "Show Hints");
                    ui.separator();
                    if ui.selectable_label(self.active_tab == UiTab::Game, "Game").clicked() {
                        self.active_tab = UiTab::Game;
                    }
                    if ui.selectable_label(self.active_tab == UiTab::Analysis, "Analysis").clicked() {
                        self.active_tab = UiTab::Analysis;
                    }
                    if ui.selectable_label(self.active_tab == UiTab::Settings, "Settings").clicked() {
                        self.active_tab = UiTab::Settings;
                    }
                });
            });
        });
        
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
                
                let elapsed = self.game_start.elapsed().as_secs();
                ui.label(format!("Time: {}:{:02}", elapsed / 60, elapsed % 60));
                
                ui.add_space(10.0);
            });
        });

        // --- Left Panel: Controls ---
        egui::SidePanel::left("controls").show(ctx, |ui| {
            ui.add_space(10.0);
            ui.heading("Controls");
            ui.separator();
            ui.add_space(10.0);

            if ui.button("🔄 New Game").clicked() {
                self.new_game();
            }

            if ui
                .add_enabled(self.history.len() > 1, egui::Button::new("↶ Undo Move"))
                .clicked()
            {
                self.undo_move();
            }

            ui.add_space(20.0);
            ui.separator();
            
            match self.active_tab {
                UiTab::Game => {
                    ui.heading("Game Mode");
                    
                    if ui.radio(matches!(self.game_mode, GameMode::HumanVsHuman), "Human vs Human").clicked() {
                        self.set_game_mode(GameMode::HumanVsHuman);
                    }
                    
                    if ui.radio(matches!(self.game_mode, GameMode::HumanVsAi { human_player: Player::XS, .. }), 
                        "Play as XS").clicked() {
                        self.set_game_mode(GameMode::HumanVsAi { 
                            human_player: Player::XS, 
                            ai_difficulty: AiDifficulty::Smart 
                        });
                    }
                    
                    if ui.radio(matches!(self.game_mode, GameMode::HumanVsAi { human_player: Player::OT, .. }), 
                        "Play as OT").clicked() {
                        self.set_game_mode(GameMode::HumanVsAi { 
                            human_player: Player::OT, 
                            ai_difficulty: AiDifficulty::Smart 
                        });
                    }
                    
                    if ui.radio(matches!(self.game_mode, GameMode::AiVsAi { .. }), "AI vs AI").clicked() {
                        self.set_game_mode(GameMode::AiVsAi { 
                            xs_difficulty: AiDifficulty::Smart, 
                            ot_difficulty: AiDifficulty::Smart 
                        });
                    }
                    
                    // AI Difficulty settings
                    match &mut self.game_mode {
                        GameMode::HumanVsAi { ai_difficulty, .. } => {
                            ui.add_space(10.0);
                            ui.label("AI Difficulty:");
                            ui.radio_value(ai_difficulty, AiDifficulty::Random, "Random");
                            ui.radio_value(ai_difficulty, AiDifficulty::Smart, "Smart");
                            ui.radio_value(ai_difficulty, AiDifficulty::Perfect, "Perfect");
                        }
                        GameMode::AiVsAi { xs_difficulty, ot_difficulty } => {
                            ui.add_space(10.0);
                            ui.label("XS Difficulty:");
                            ui.radio_value(xs_difficulty, AiDifficulty::Random, "Random");
                            ui.radio_value(xs_difficulty, AiDifficulty::Smart, "Smart");
                            ui.radio_value(xs_difficulty, AiDifficulty::Perfect, "Perfect");
                            
                            ui.add_space(5.0);
                            ui.label("OT Difficulty:");
                            ui.radio_value(ot_difficulty, AiDifficulty::Random, "Random");
                            ui.radio_value(ot_difficulty, AiDifficulty::Smart, "Smart");
                            ui.radio_value(ot_difficulty, AiDifficulty::Perfect, "Perfect");
                        }
                        _ => {}
                    }
                }
                UiTab::Analysis => {
                    ui.heading("Analysis");
                    ui.label(format!("Active Lines: {}", self.active_lines.len()));
                    ui.label(format!("Possible Moves: {}", self.possible_moves.len()));
                    
                    if self.show_hints {
                        if let Some((_best_move, reason)) = self.get_best_move_hint() {
                            ui.add_space(10.0);
                            ui.label(egui::RichText::new("💡 Hint:").color(Color32::YELLOW));
                            ui.label(reason);
                        }
                    }
                }
                UiTab::Settings => {
                    ui.heading("Settings");
                    ui.add_space(5.0);
                    
                    ui.checkbox(&mut self.show_hints, "Show Hints");
                    ui.label("💡 Shows AI move suggestions");
                    
                    ui.add_space(10.0);
                    ui.checkbox(&mut self.show_coordinates, "Show Coordinates");
                    ui.label("🔢 Display position numbers");
                    
                    ui.add_space(10.0);
                    ui.checkbox(&mut self.sound_effects, "Sound Effects");
                    ui.label("🔊 Play sounds (not implemented)");
                    
                    ui.add_space(10.0);
                    ui.label("Theme:");
                    ui.radio_value(&mut self.theme, Theme::Dark, "🌙 Dark");
                    ui.radio_value(&mut self.theme, Theme::Light, "☀️ Light");
                    
                    ui.add_space(20.0);
                    ui.separator();
                    ui.heading("Keyboard Shortcuts");
                    ui.label("⌘N / Ctrl+N: New Game");
                    ui.label("⌘Z / Ctrl+Z: Undo");
                    ui.label("H: Toggle Hints");
                }
            }
            
            // Only show statistics when not in Settings tab
            if self.active_tab != UiTab::Settings {
                ui.add_space(20.0);
                ui.separator();
                ui.heading("Statistics");
                ui.label(format!("Games: {}", self.stats.games_played));
                ui.label(format!("XS Wins: {}", self.stats.xs_wins));
                ui.label(format!("OT Wins: {}", self.stats.ot_wins));
                ui.label(format!("Draws: {}", self.stats.draws));
            }
        });
        
        // --- Right Panel: Move History ---
        egui::SidePanel::right("history").show(ctx, |ui| {
            ui.add_space(10.0);
            ui.heading("Move History");
            ui.separator();
            ui.add_space(5.0);
            
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (idx, entry) in self.move_history.iter().enumerate() {
                    let player_str = match entry.game_move.who {
                        Player::XS => "XS",
                        Player::OT => "OT",
                    };
                    let sym_str = match entry.game_move.drawn_symbol {
                        Sym::X => "X",
                        Sym::O => "O",
                        Sym::T => "T",
                        Sym::S => "S",
                    };
                    let pos = entry.game_move.place.0 + 1;
                    
                    ui.horizontal(|ui| {
                        ui.label(format!("{}.", idx + 1));
                        ui.colored_label(
                            sym_color(entry.game_move.drawn_symbol),
                            format!("{} {} → {}", player_str, sym_str, pos)
                        );
                    });
                }
            });
        });

        // --- Central Panel: Game Board ---
        egui::CentralPanel::default().show(ctx, |ui| {
            // Calculate square size based on available space
            let available_size = ui.available_size();
            let board_size = available_size.x.min(available_size.y) * 0.85;
            let cell_size = board_size / 3.2; // Leave some spacing
            let spacing = cell_size * 0.1;
            
            // Center the board both horizontally and vertically
            let grid_size = cell_size * 3.0 + spacing * 2.0;
            let x_offset = (available_size.x - grid_size) / 2.0;
            let y_offset = (available_size.y - grid_size) / 2.0;
            
            ui.allocate_ui_at_rect(
                egui::Rect::from_min_size(
                    ui.min_rect().min + egui::vec2(x_offset, y_offset),
                    egui::vec2(grid_size, grid_size)
                ),
                |ui| {
                    // Store cell centers for drawing winning lines
                    let mut cell_centers = [egui::pos2(0.0, 0.0); 9];
                    
                    egui::Grid::new("board_grid")
                        .min_col_width(cell_size)
                        .min_row_height(cell_size)
                        .spacing([spacing, spacing])
                        .show(ui, |ui| {
                        for i in 0..9 {
                            let (_row, col) = (i / 3, i % 3);
                            let place_idx = PlaceIdx(i as u8);

                            let (rect, response) =
                                ui.allocate_exact_size(vec2(cell_size, cell_size), egui::Sense::click());
                            
                            // Store cell center for winning line drawing
                            cell_centers[i] = rect.center();
                            
                            // Draw cell background with theme awareness
                            let is_last_move = self.last_move.map_or(false, |m| m.place == place_idx);
                            let (cell_bg_color, border_color) = if self.theme == Theme::Dark {
                                if is_last_move {
                                    (Color32::from_rgb(60, 60, 80), Color32::from_rgb(100, 100, 120))
                                } else if response.hovered() {
                                    (Color32::from_rgb(50, 50, 60), Color32::from_rgb(90, 90, 100))
                                } else {
                                    (Color32::from_rgb(40, 40, 50), Color32::from_rgb(80, 80, 90))
                                }
                            } else {
                                if is_last_move {
                                    (Color32::from_rgb(220, 220, 240), Color32::from_rgb(180, 180, 200))
                                } else if response.hovered() {
                                    (Color32::from_rgb(230, 230, 240), Color32::from_rgb(190, 190, 210))
                                } else {
                                    (Color32::from_rgb(240, 240, 250), Color32::from_rgb(200, 200, 220))
                                }
                            };
                            ui.painter().rect_filled(rect, 5.0, cell_bg_color);
                            ui.painter().rect_stroke(rect, 5.0, Stroke::new(2.0, border_color), egui::epaint::StrokeKind::Outside);
                            
                            // Draw coordinates if enabled
                            if self.show_coordinates {
                                let coord_color = if self.theme == Theme::Dark {
                                    Color32::from_rgba_premultiplied(200, 200, 200, 100)
                                } else {
                                    Color32::from_rgba_premultiplied(80, 80, 80, 100)
                                };
                                ui.painter().text(
                                    rect.left_top() + egui::vec2(5.0, 5.0),
                                    egui::Align2::LEFT_TOP,
                                    format!("{}", i + 1),
                                    egui::FontId::proportional(12.0),
                                    coord_color,
                                );
                            }

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
                            
                            // Update potential lines on hover
                            if is_hovered && !potential_moves_for_cell.is_empty() {
                                self.potential_lines.clear();
                                for &game_move in &potential_moves_for_cell {
                                    self.update_potential_lines(game_move);
                                }
                            }
                            
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
                            
                            // Highlight hint move if enabled
                            let hint_move = if self.show_hints && self.outcome.is_none() {
                                self.get_best_move_hint().map(|(m, _)| m)
                            } else {
                                None
                            };
                            
                            let _is_hint_cell = hint_move.map_or(false, |m| m.place == place_idx);
                            
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
                                let is_hint_sym = hint_move.map_or(false, |m| 
                                    m.place == place_idx && m.drawn_symbol == sym
                                );
                                
                                let style = if placed_symbols.contains(&sym) {
                                    SymbolStyle::PlacedByOpponent
                                } else if available_symbols.contains(&sym) {
                                    if is_hovered || is_hint_sym {
                                        SymbolStyle::AvailableHovered
                                    } else {
                                        SymbolStyle::Available
                                    }
                                } else {
                                    SymbolStyle::Available
                                };
                                
                                // Add subtle pulse animation for hint
                                let final_rect = if is_hint_sym {
                                    let pulse = (self.animation_time * 3.0).sin() * 0.5 + 0.5;
                                    let expand = (pulse * 2.0) as f32;
                                    quad_rect.expand(expand)
                                } else {
                                    quad_rect
                                };
                                
                                draw_sym(sym, final_rect, ui, style);
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
                    
                    // Draw potential lines (on hover)
                    for line in &self.potential_lines {
                        draw_winning_line(ui, line, &cell_centers, LineStyle::PotentialWinning);
                    }
                    
                    // Draw active winning lines (during game)
                    if self.outcome.is_none() {
                        for line in &self.active_lines {
                            draw_winning_line(ui, line, &cell_centers, LineStyle::ActiveWinning);
                        }
                    }
                    
                    // Draw final winning lines (game over)
                    if self.outcome.is_some() {
                        for line in &self.winning_lines {
                            draw_winning_line(ui, line, &cell_centers, LineStyle::FinalWinning);
                        }
                    }
                },
            );
        });

        // Try to make an AI move if it's AI's turn
        self.try_ai_move(dt);

        // Reset hovered move and potential lines if mouse is not over any cell
        if !ctx.input(|i| i.pointer.has_pointer()) {
            self.hovered_move = None;
            if !self.potential_lines.is_empty() {
                self.potential_lines.clear();
            }
        }
    }
}

//=============================================================================
//  MAIN FUNCTION
//=============================================================================

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 700.0])
            .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Tic Tac Flic Flac Floe",
        native_options,
        Box::new(|_cc| Ok(Box::<TtfffApp>::default())),
    )
}
