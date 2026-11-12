use crate::{best_move, uci_move};
use chess::{Board, BoardStatus, ChessMove, Color, MoveGen, Piece, Square};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, TextureHandle, Vec2};
use std::collections::HashMap;

pub struct ChessApp {
    board: Board,
    history: Vec<Board>,
    moves: Vec<String>,
    selected: Option<Square>,
    promotion: Option<(Square, Square)>,
    textures: HashMap<char, TextureHandle>,
    engine_enabled: bool,
    depth: u8,
    status: String,
}

impl ChessApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut textures = HashMap::new();
        let assets: [(char, &[u8]); 12] = [
            ('K', include_bytes!("../assets/pieces/wK.png")),
            ('Q', include_bytes!("../assets/pieces/wQ.png")),
            ('R', include_bytes!("../assets/pieces/wR.png")),
            ('B', include_bytes!("../assets/pieces/wB.png")),
            ('N', include_bytes!("../assets/pieces/wN.png")),
            ('P', include_bytes!("../assets/pieces/wP.png")),
            ('k', include_bytes!("../assets/pieces/bK.png")),
            ('q', include_bytes!("../assets/pieces/bQ.png")),
            ('r', include_bytes!("../assets/pieces/bR.png")),
            ('b', include_bytes!("../assets/pieces/bB.png"),
            ('n', include_bytes!("../assets/pieces/bN.png"),
            ('p', include_bytes!("../assets/pieces/bP.png"),
        ];
        for (key, bytes) in assets {
            let image = image::load_from_memory(bytes).expect("piece PNG").to_rgba8();
            let size = [image.width() as usize, image.height() as usize];
            let color = egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw());
            textures.insert(key, cc.egui_ctx.load_texture(key.to_string(), color, Default::default()));
        }
        let mut style = (*cc.egui_ctx.style()).clone();
        style.visuals = egui::Visuals::dark();
        style.visuals.panel_fill = Color32::from_rgb(23, 25, 29);
        style.visuals.window_fill = Color32::from_rgb(29, 31, 36);
        cc.egui_ctx.set_style(style);
        Self {
            board: Board::default(),
            history: vec![Board::default()],
            moves: Vec::new(),
            selected: None,
            promotion: None,
            textures,
            engine_enabled: true,
            depth: 3,
            status: "Your move".to_owned(),
        }
    }

    fn code(piece: Piece, color: Color) -> char {
        let letter = match piece {
            Piece::King => 'k',
            Piece::Queen => 'q',
            Piece::Rook => 'r',
            Piece::Bishop => 'b',
            Piece::Knight => 'n',
            Piece::Pawn => 'p',
        };
        if color == Color::White { letter.to_ascii_uppercase() } else { letter }
    }

    fn legal_moves(&self) -> Vec<ChessMove> {
        MoveGen::new_legal(&self.board).collect()
    }

    fn commit(&mut self, mv: ChessMove) {
        self.board = self.board.make_move_new(mv);
        self.history.push(self.board);
        self.moves.push(uci_move(mv));
        self.selected = None;
        self.promotion = None;
        self.status = match self.board.status() {
            BoardStatus::Checkmate => "Checkmate".to_owned(),
            BoardStatus::Stalemate => "Draw by stalemate".to_owned(),
            BoardStatus::Ongoing if self.board.checkers().popcnt() > 0 => "Check".to_owned(),
            BoardStatus::Ongoing if self.engine_enabled => "Your move".to_owned(),
            BoardStatus::Ongoing if self.board.side_to_move() == Color::White => "White to move".to_owned(),
            BoardStatus::Ongoing => "Black to move".to_owned(),
        };
        if self.engine_enabled && self.board.status() == BoardStatus::Ongoing && self.board.side_to_move() == Color::Black {
            self.status = "Phoenix is thinking...".to_owned();
            if let Some(reply) = best_move(&self.board, self.depth) {
                self.board = self.board.make_move_new(reply);
                self.history.push(self.board);
                self.moves.push(uci_move(reply));
            }
            self.status = match self.board.status() {
                BoardStatus::Checkmate => "Checkmate".to_owned(),
                BoardStatus::Stalemate => "Draw by stalemate".to_owned(),
                BoardStatus::Ongoing if self.board.checkers().popcnt() > 0 => "Check".to_owned(),
                BoardStatus::Ongoing => "Your move".to_owned(),
            };
        }
    }

    fn tap_square(&mut self, square: Square) {
        if self.promotion.is_some() || self.board.status() != BoardStatus::Ongoing {
            return;
        }
        if let Some(from) = self.selected {
            let candidate = self.legal_moves().into_iter().find(|mv| {
                mv.get_source() == from && mv.get_dest() == square
            });
            if let Some(mv) = candidate {
                if mv.get_promotion().is_some() {
                    self.promotion = Some((from, square));
                } else {
                    self.commit(mv);
                }
                return;
            }
        }
        let selectable = if self.engine_enabled { Color::White } else { self.board.side_to_move() };
        if self.board.color_on(square) == Some(selectable) {
            self.selected = Some(square);
        } else {
            self.selected = None;
        }
    }

    fn choose_promotion(&mut self, piece: Piece) {
        if let Some((from, to)) = self.promotion {
            self.commit(ChessMove::new(from, to, Some(piece)));
        }
    }

    fn undo(&mut self) {
        let count = if self.engine_enabled && self.history.len() > 2 { 2 } else { 1 };
        for _ in 0..count {
            if self.history.len() > 1 {
                self.history.pop();
                self.moves.pop();
            }
        }
        self.board = *self.history.last().unwrap_or(&Board::default());
        self.selected = None;
        self.promotion = None;
        self.status = "Move undone".to_owned();
    }

    fn restart(&mut self) {
        self.board = Board::default();
        self.history = vec![self.board];
        self.moves.clear();
        self.selected = None;
        self.promotion = None;
        self.status = "New game".to_owned();
    }

    fn draw_board(&mut self, ui: &mut egui::Ui) {
        let size = ui.available_width().min(ui.available_height().max(280.0)).min(640.0);
        let cell = size / 8.0;
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
        let painter = ui.painter_at(rect);
        let legal = self.legal_moves();
        for row in 0..8 {
            for col in 0..8 {
                let square = Square::make_square(
                    chess::Rank::from_index(7 - row),
                    chess::File::from_index(col),
                );
                let cell_rect = Rect::from_min_size(
                    Pos2::new(rect.left() + col as f32 * cell, rect.top() + row as f32 * cell),
                    Vec2::splat(cell),
                );
                let light = (row + col) % 2 == 0;
                let base = if light { Color32::from_rgb(232, 218, 190) } else { Color32::from_rgb(111, 139, 117) };
                painter.rect_filled(cell_rect, 0.0, base);
                if self.selected == Some(square) {
                    painter.rect_filled(cell_rect, 0.0, Color32::from_rgba_unmultiplied(250, 210, 65, 105));
                } else if self.selected.is_some() && legal.iter().any(|mv| mv.get_source() == self.selected.unwrap() && mv.get_dest() == square) {
                    painter.circle_filled(cell_rect.center(), cell * 0.12, Color32::from_rgba_unmultiplied(20, 30, 25, 105));
                }
                if let (Some(piece), Some(color)) = (self.board.piece_on(square), self.board.color_on(square)) {
                    let key = Self::code(piece, color);
                    if let Some(texture) = self.textures.get(&key) {
                        let padding = cell * 0.07;
                        let piece_rect = cell_rect.shrink(padding);
                        painter.image(texture.id(), piece_rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                    }
                }
            }
        }
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let col = ((pos.x - rect.left()) / cell) as usize;
                let row = ((pos.y - rect.top()) / cell) as usize;
                if row < 8 && col < 8 {
                    self.tap_square(Square::make_square(
                        chess::Rank::from_index(7 - row),
                        chess::File::from_index(col),
                    ));
                }
            }
        }
    }
}

impl eframe::App for ChessApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                ui.heading("PHOENIX CHESS");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(&self.status);
                });
            });
            ui.add_space(5.0);
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Play as White");
                    ui.checkbox(&mut self.engine_enabled, "Engine");
                    ui.add(egui::Slider::new(&mut self.depth, 1..=6).text("AI strength"));
                    ui.label(match self.depth { 1 => "Beginner", 2 => "Easy", 3 => "Medium", 4 => "Hard", 5 => "Expert", _ => "Phoenix" });
                });
                ui.add_space(8.0);
                self.draw_board(ui);
                ui.add_space(10.0);
                if let Some((_, _)) = self.promotion {
                    ui.horizontal(|ui| {
                        ui.label("Promote to");
                        if ui.button("Queen").clicked() { self.choose_promotion(Piece::Queen); }
                        if ui.button("Rook").clicked() { self.choose_promotion(Piece::Rook); }
                        if ui.button("Bishop").clicked() { self.choose_promotion(Piece::Bishop); }
                        if ui.button("Knight").clicked() { self.choose_promotion(Piece::Knight); }
                    });
                }
                ui.horizontal(|ui| {
                    if ui.button("Undo").clicked() { self.undo(); }
                    if ui.button("New game").clicked() { self.restart(); }
                });
                ui.add_space(8.0);
                egui::ScrollArea::horizontal().show(ui, |ui| {
                    ui.label(if self.moves.is_empty() { "Move history" } else { self.moves.join("   ") });
                });
            });
        });
        ctx.request_repaint();
    }
}
