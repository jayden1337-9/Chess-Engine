use crate::{best_move, uci_move};
use crate::chat::{ChatClient, ChatTab};
use chess::{Board, BoardStatus, ChessMove, Color, MoveGen, Piece, Square};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, TextureHandle, Vec2};
use std::{collections::HashMap, str::FromStr};

#[derive(Clone, Copy, PartialEq, Eq)]
enum AppPage { Home, Play, Puzzles, Learn, Watch, Profile }

pub struct ChessApp {
    page: AppPage,
    board: Board,
    history: Vec<Board>,
    moves: Vec<String>,
    selected: Option<Square>,
    promotion: Option<(Square, Square)>,
    textures: HashMap<char, TextureHandle>,
    engine_enabled: bool,
    depth: u8,
    status: String,
    chat: ChatClient,
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
            ('b', include_bytes!("../assets/pieces/bB.png")),
            ('n', include_bytes!("../assets/pieces/bN.png")),
            ('p', include_bytes!("../assets/pieces/bP.png")),
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
            chat: ChatClient::default(),
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
        if !self.engine_enabled && !self.chat.room_code.is_empty() {
            if self.chat.player_color != Some(self.board.side_to_move()) { return; }
            self.chat.send_move(&uci_move(mv));
        }
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
            self.status = "Chess Engine is thinking...".to_owned();
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
        let selectable = if self.engine_enabled { Color::White } else { self.chat.player_color.unwrap_or_else(|| self.board.side_to_move()) };
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

    fn piece_locations(&self, color: Color) -> Vec<String> {
        let pieces = [
            (Piece::King, "King"),
            (Piece::Queen, "Queen"),
            (Piece::Rook, "Rook"),
            (Piece::Bishop, "Bishop"),
            (Piece::Knight, "Knight"),
            (Piece::Pawn, "Pawn"),
        ];
        pieces.iter().map(|(piece, name)| {
            let squares: Vec<String> = (0..8).flat_map(|rank| {
                (0..8).filter_map(move |file| {
                    let square = Square::make_square(
                        chess::Rank::from_index(rank),
                        chess::File::from_index(file),
                    );
                    (self.board.piece_on(square) == Some(*piece)
                        && self.board.color_on(square) == Some(color))
                        .then(|| square.to_string())
                })
            }).collect();
            format!("{}: {}", name, if squares.is_empty() { "—".to_owned() } else { squares.join(", ") })
        }).collect()
    }

    fn draw_board(&mut self, ui: &mut egui::Ui) {
        let size = ui.available_width().min((ui.available_height() - 8.0).max(180.0)).min(640.0);
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
                let coordinate = format!("{}{}", (b'a' + col as u8) as char, 8 - row);
                painter.text(
                    cell_rect.left_top() + Vec2::new(3.0, 2.0),
                    egui::Align2::LEFT_TOP,
                    coordinate,
                    egui::FontId::proportional((cell * 0.13).max(8.0)),
                    if light { Color32::from_gray(105) } else { Color32::from_gray(235) },
                );
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
        self.chat.poll();
        for (uci, fen) in self.chat.take_remote_moves() {
            if let Ok(board) = Board::from_str(&fen) {
                self.board = board;
                self.history.push(board);
                self.moves.push(uci);
                self.selected = None;
                self.promotion = None;
                self.status = match board.status() {
                    BoardStatus::Checkmate => "Checkmate".to_owned(),
                    BoardStatus::Stalemate => "Draw by stalemate".to_owned(),
                    BoardStatus::Ongoing if board.checkers().popcnt() > 0 => "Check".to_owned(),
                    BoardStatus::Ongoing => "Opponent moved".to_owned(),
                };
            }
        }

        egui::TopBottomPanel::top("site_header").show(ctx, |ui| {
            ui.add_space(5.0);
            ui.horizontal_wrapped(|ui| {
                ui.heading("♟ Chess Engine");
                ui.separator();
                for (page, label) in [
                    (AppPage::Home, "Home"),
                    (AppPage::Play, "Play"),
                    (AppPage::Puzzles, "Puzzles"),
                    (AppPage::Learn, "Learn"),
                    (AppPage::Watch, "Watch"),
                    (AppPage::Profile, "Profile"),
                ] {
                    if ui.selectable_label(self.page == page, label).clicked() {
                        self.page = page;
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(&self.status).small().color(Color32::from_rgb(170, 190, 175)));
                });
            });
            ui.add_space(5.0);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(12.0);
                match self.page {
                    AppPage::Home => {
                        ui.heading("Play chess. Get better.");
                        ui.label("Welcome to Chess Engine — your place to play, learn and improve.");
                        ui.add_space(18.0);
                        ui.columns(2, |columns| {
                            columns[0].group(|ui| {
                                ui.heading("Play online");
                                ui.label("Challenge another player with live multiplayer.");
                                ui.add_space(8.0);
                                if ui.add_sized([ui.available_width(), 42.0], egui::Button::new("Play a friend")).clicked() {
                                    self.engine_enabled = false;
                                    self.page = AppPage::Play;
                                    self.restart();
                                }
                            });
                            columns[1].group(|ui| {
                                ui.heading("Play vs Chess Engine");
                                ui.label("Choose your difficulty and test your game.");
                                ui.add_space(8.0);
                                if ui.add_sized([ui.available_width(), 42.0], egui::Button::new("Play computer")).clicked() {
                                    self.engine_enabled = true;
                                    self.page = AppPage::Play;
                                    self.restart();
                                }
                            });
                        });
                        ui.add_space(14.0);
                        ui.heading("Explore");
                        ui.columns(3, |columns| {
                            columns[0].group(|ui| {
                                ui.heading("Puzzles");
                                ui.label("Sharpen your tactics.");
                                if ui.button("Explore").clicked() { self.page = AppPage::Puzzles; }
                            });
                            columns[1].group(|ui| {
                                ui.heading("Learn");
                                ui.label("Openings and chess basics.");
                                if ui.button("Explore").clicked() { self.page = AppPage::Learn; }
                            });
                            columns[2].group(|ui| {
                                ui.heading("Watch");
                                ui.label("Follow featured games.");
                                if ui.button("Explore").clicked() { self.page = AppPage::Watch; }
                            });
                        });
                        ui.add_space(14.0);
                        ui.group(|ui| {
                            ui.heading("Your chess space");
                            ui.label("Play online and against the engine from the same Chess Engine account experience.");
                            ui.label("Ratings, saved games and player profiles will be added as account services are built.");
                        });
                    }
                    AppPage::Play => {
                        ui.horizontal_wrapped(|ui| {
                            if ui.selectable_label(self.engine_enabled, "Play vs AI").clicked() && !self.engine_enabled {
                                self.engine_enabled = true;
                                self.restart();
                            }
                            if ui.selectable_label(!self.engine_enabled, "PvP").clicked() && self.engine_enabled {
                                self.engine_enabled = false;
                                self.restart();
                            }
                            if self.engine_enabled {
                                ui.add(egui::Slider::new(&mut self.depth, 1..=6).text("AI strength"));
                                ui.label(match self.depth { 1 => "Beginner", 2 => "Easy", 3 => "Medium", 4 => "Hard", 5 => "Expert", _ => "Chess Engine" });
                            }
                        });
                        ui.add_space(10.0);
                        ui.columns(2, |columns| {
                            columns[0].vertical_centered(|ui| {
                                self.draw_board(ui);
                                if self.promotion.is_some() {
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
                            });
                            columns[1].vertical(|ui| {
                                ui.group(|ui| {
                                    ui.heading("Game");
                                    ui.label(&self.status);
                                    ui.label(if self.engine_enabled { "You are playing the computer." } else if self.chat.room_code.is_empty() { "Local board or connect to an online room." } else { "Online room active." });
                                    ui.separator();
                                    ui.label("Move history");
                                    egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                                        if self.moves.is_empty() { ui.weak("Moves will appear here."); }
                                        for (index, mv) in self.moves.iter().enumerate() {
                                            ui.label(format!("{}. {}", index + 1, mv));
                                        }
                                    });
                                });
                                egui::CollapsingHeader::new("Online game & chat").default_open(true).show(ui, |ui| {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label("Server");
                                        ui.text_edit_singleline(&mut self.chat.url);
                                        if ui.button("Connect").clicked() { self.chat.connect(); }
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Name");
                                        ui.add(egui::TextEdit::singleline(&mut self.chat.name).desired_width(110.0));
                                        if ui.button("Create room").clicked() { self.chat.create_room(); }
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Room");
                                        ui.add(egui::TextEdit::singleline(&mut self.chat.room_code).desired_width(100.0));
                                        if ui.button("Join").clicked() { self.chat.join_room(); }
                                    });
                                    ui.horizontal(|ui| {
                                        if ui.selectable_label(self.chat.tab == ChatTab::Global, "Global").clicked() { self.chat.tab = ChatTab::Global; }
                                        if ui.selectable_label(self.chat.tab == ChatTab::Match, "Match").clicked() { self.chat.tab = ChatTab::Match; }
                                    });
                                    let scope = if self.chat.tab == ChatTab::Global { "global" } else { "match" };
                                    egui::ScrollArea::vertical().max_height(140.0).stick_to_bottom(true).show(ui, |ui| {
                                        let mut shown = false;
                                        for line in &self.chat.lines {
                                            if line.scope == scope {
                                                shown = true;
                                                ui.horizontal_wrapped(|ui| {
                                                    ui.strong(format!("{}:", line.name));
                                                    ui.label(&line.message);
                                                });
                                            }
                                        }
                                        if !shown { ui.weak(if scope == "global" { "No global messages yet." } else { "No match messages yet." }); }
                                    });
                                    ui.horizontal(|ui| {
                                        let response = ui.add(egui::TextEdit::singleline(&mut self.chat.input).hint_text("Write a message...").desired_width(ui.available_width() - 58.0));
                                        let enter = response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
                                        if ui.button("Send").clicked() || enter { self.chat.send_chat(); }
                                    });
                                    ui.small(&self.chat.status);
                                });
                            });
                        });
                    }
                    AppPage::Puzzles => {
                        ui.heading("Chess puzzles");
                        ui.label("Tactical training is coming next.");
                        ui.group(|ui| {
                            ui.heading("Puzzle modes planned");
                            ui.label("• Find the best move");
                            ui.label("• Checkmate in one, two and three");
                            ui.label("• Daily puzzle streaks");
                            ui.label("• Puzzle rating and progress");
                        });
                        if ui.button("Back to play").clicked() { self.page = AppPage::Play; }
                    }
                    AppPage::Learn => {
                        ui.heading("Learn chess");
                        ui.label("Build your understanding one lesson at a time.");
                        ui.group(|ui| {
                            ui.heading("Learning library");
                            ui.label("• How each piece moves");
                            ui.label("• Opening principles");
                            ui.label("• Tactics and checkmates");
                            ui.label("• Endgame fundamentals");
                        });
                        ui.weak("Interactive lessons are coming soon.");
                    }
                    AppPage::Watch => {
                        ui.heading("Watch chess");
                        ui.label("A home for live games, featured matches and spectators.");
                        ui.weak("Live game listings and spectator boards are coming soon.");
                    }
                    AppPage::Profile => {
                        ui.heading("Player profile");
                        ui.label(&self.chat.name);
                        ui.group(|ui| {
                            ui.heading("Your stats");
                            ui.label(format!("Games played on this device: {}", self.history.len().saturating_sub(1)));
                            ui.label("Online rating: not available yet");
                            ui.label("Game archive: coming soon");
                        });
                    }
                }
                ui.add_space(24.0);
            });
        });
        ctx.request_repaint();
    }
}
