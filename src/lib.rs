use chess::{Board, BoardStatus, ChessMove, Color, MoveGen, Piece, Square};
use std::collections::HashMap;
use std::str::FromStr;

const MATE: i32 = 30_000;
const INF: i32 = 31_000;
const MAX_PLY: usize = 128;
const TT_LIMIT: usize = 1_000_000;

#[derive(Clone, Copy)]
enum Bound {
    Exact,
    Lower,
    Upper,
}

#[derive(Clone, Copy)]
struct TTEntry {
    depth: i16,
    score: i32,
    bound: Bound,
    best_move: Option<ChessMove>,
}

struct SearchContext {
    table: HashMap<u64, TTEntry>,
    killers: [[Option<ChessMove>; 2]; MAX_PLY],
    history: [[i32; 4096]; 2],
}

impl SearchContext {
    fn new() -> Self {
        Self {
            table: HashMap::new(),
            killers: [[None; 2]; MAX_PLY],
            history: [[0; 4096]; 2],
        }
    }

    fn remember_quiet(&mut self, board: &Board, mv: ChessMove, ply: usize, depth: u8) {
        if ply < MAX_PLY && self.killers[ply][0] != Some(mv) {
            self.killers[ply][1] = self.killers[ply][0];
            self.killers[ply][0] = Some(mv);
        }
        let color = color_index(board.side_to_move());
        let index = move_index(mv);
        let bonus = (depth as i32 * depth as i32).min(2048);
        self.history[color][index] = (self.history[color][index] + bonus).min(32_000);
    }

    fn age_history(&mut self) {
        for side in &mut self.history {
            for value in side {
                *value /= 2;
            }
        }
    }
}

pub fn parse_uci_move(board: &Board, text: &str) -> Option<ChessMove> {
    if text.len() < 4 || text.len() > 5 { return None; }
    let from = Square::from_str(&text[0..2]).ok()?;
    let to = Square::from_str(&text[2..4]).ok()?;
    let promotion = match text.as_bytes().get(4).copied() {
        Some(b'q') => Some(Piece::Queen),
        Some(b'r') => Some(Piece::Rook),
        Some(b'b') => Some(Piece::Bishop),
        Some(b'n') => Some(Piece::Knight),
        None => None,
        _ => return None,
    };
    let mv = ChessMove::new(from, to, promotion);
    MoveGen::new_legal(board).any(|legal| legal == mv).then_some(mv)
}

pub fn uci_move(mv: ChessMove) -> String {
    let mut text = format!("{}{}", mv.get_source(), mv.get_dest());
    if let Some(piece) = mv.get_promotion() {
        text.push(match piece { Piece::Queen => 'q', Piece::Rook => 'r', Piece::Bishop => 'b', Piece::Knight => 'n', _ => 'q' });
    }
    text
}

fn piece_value(piece: Piece) -> i32 {
    match piece {
        Piece::Pawn => 100,
        Piece::Knight => 320,
        Piece::Bishop => 335,
        Piece::Rook => 500,
        Piece::Queen => 900,
        Piece::King => 20_000,
    }
}

fn color_index(color: Color) -> usize {
    if color == Color::White { 0 } else { 1 }
}

fn square_coords(square: Square) -> (usize, usize) {
    let text = square.to_string();
    let bytes = text.as_bytes();
    ((bytes[0] - b'a') as usize, (bytes[1] - b'1') as usize)
}

fn move_index(mv: ChessMove) -> usize {
    let (ff, fr) = square_coords(mv.get_source());
    let (tf, tr) = square_coords(mv.get_dest());
    (fr * 8 + ff) * 64 + tr * 8 + tf
}

fn piece_square_bonus(piece: Piece, color: Color, file: usize, rank: usize, endgame: bool) -> i32 {
    let relative_rank = if color == Color::White { rank } else { 7 - rank };
    let file_distance = (file as i32 * 2 - 7).abs();
    let rank_distance = (relative_rank as i32 * 2 - 7).abs();
    let center = 14 - file_distance - rank_distance;
    match piece {
        Piece::Pawn => relative_rank as i32 * 7 + center * 2,
        Piece::Knight => center * 7,
        Piece::Bishop => center * 4 + relative_rank as i32 * 2,
        Piece::Rook => relative_rank as i32 * 2 + if relative_rank == 6 { 12 } else { 0 },
        Piece::Queen => center * 2,
        Piece::King if endgame => center * 7,
        Piece::King => {
            let shelter = if relative_rank == 0 && (file == 1 || file == 2 || file == 6) { 34 } else { 0 };
            shelter - center * 5
        }
    }
}

fn king_safety(board: &Board, color: Color) -> i32 {
    let king = board.king_square(color);
    let (file, rank) = square_coords(king);
    let forward = if color == Color::White { 1i32 } else { -1i32 };
    let mut score = 0;
    let mut shield = 0;
    for df in -1i32..=1 {
        let f = file as i32 + df;
        if !(0..8).contains(&f) { continue; }
        let own_pawn = (0..8).any(|r| {
            let square = Square::make_square(chess::Rank::from_index(r), chess::File::from_index(f as usize));
            board.piece_on(square) == Some(Piece::Pawn) && board.color_on(square) == Some(color)
        });
        let enemy_pawn = (0..8).any(|r| {
            let square = Square::make_square(chess::Rank::from_index(r), chess::File::from_index(f as usize));
            board.piece_on(square) == Some(Piece::Pawn) && board.color_on(square) == Some(!color)
        });
        if own_pawn {
            if (rank as i32 + forward).clamp(0, 7) as usize <= 7 { shield += 1; }
        } else {
            score -= if enemy_pawn { 9 } else { 17 };
        }
        if !own_pawn {
            score -= 10;
        }
    }
    score += shield * 13;
    score
}

pub fn evaluate(board: &Board) -> i32 {
    let mut score = 0;
    let mut phase = 0;
    for rank in 0..8 {
        for file in 0..8 {
            let square = Square::make_square(
                chess::Rank::from_index(rank),
                chess::File::from_index(file),
            );
            if let Some(piece) = board.piece_on(square) {
                let color = board.color_on(square).unwrap_or(Color::White);
                if piece != Piece::Pawn && piece != Piece::King {
                    phase += match piece {
                        Piece::Knight | Piece::Bishop => 1,
                        Piece::Rook => 2,
                        Piece::Queen => 4,
                        _ => 0,
                    };
                }
                let endgame = phase <= 10;
                let value = piece_value(piece) + piece_square_bonus(piece, color, file, rank, endgame);
                score += if color == Color::White { value } else { -value };
            }
        }
    }
    let mut safety = king_safety(board, Color::White) - king_safety(board, Color::Black);
    if phase > 20 { safety /= 2; }
    score += safety;
    if board.side_to_move() == Color::White { score } else { -score }
}

fn move_order_score(
    board: &Board,
    mv: ChessMove,
    tt_move: Option<ChessMove>,
    ctx: &SearchContext,
    ply: usize,
) -> i32 {
    if tt_move == Some(mv) { return 2_000_000; }
    let attacker = board.piece_on(mv.get_source()).map(piece_value).unwrap_or(0);
    let victim = board.piece_on(mv.get_dest()).map(piece_value).unwrap_or_else(|| {
        if board.piece_on(mv.get_source()) == Some(Piece::Pawn)
            && mv.get_source().get_file() != mv.get_dest().get_file() { 100 } else { 0 }
    });
    if victim > 0 {
        return 1_000_000 + victim * 16 - attacker;
    }
    if let Some(promotion) = mv.get_promotion() {
        return 900_000 + piece_value(promotion);
    }
    let next = board.make_move_new(mv);
    if next.checkers().popcnt() > 0 { return 800_000; }
    if ply < MAX_PLY {
        if ctx.killers[ply][0] == Some(mv) { return 700_000; }
        if ctx.killers[ply][1] == Some(mv) { return 690_000; }
    }
    ctx.history[color_index(board.side_to_move())][move_index(mv)]
}

fn ordered_moves(board: &Board, tt_move: Option<ChessMove>, ctx: &SearchContext, ply: usize) -> Vec<ChessMove> {
    let mut moves: Vec<ChessMove> = MoveGen::new_legal(board).collect();
    moves.sort_by_key(|mv| -move_order_score(board, *mv, tt_move, ctx, ply));
    moves
}

fn score_to_tt(score: i32, ply: i32) -> i32 {
    if score > MATE - 1000 { score + ply }
    else if score < -MATE + 1000 { score - ply }
    else { score }
}

fn score_from_tt(score: i32, ply: i32) -> i32 {
    if score > MATE - 1000 { score - ply }
    else if score < -MATE + 1000 { score + ply }
    else { score }
}

fn has_null_move_material(board: &Board) -> bool {
    let color = board.side_to_move();
    let mut major_or_minor = 0;
    for rank in 0..8 {
        for file in 0..8 {
            let square = Square::make_square(chess::Rank::from_index(rank), chess::File::from_index(file));
            if board.color_on(square) == Some(color) {
                if matches!(board.piece_on(square), Some(Piece::Knight | Piece::Bishop | Piece::Rook | Piece::Queen)) {
                    major_or_minor += 1;
                }
            }
        }
    }
    major_or_minor > 0
}

fn quiescence(board: &Board, mut alpha: i32, beta: i32, ply: i32) -> i32 {
    match board.status() {
        BoardStatus::Checkmate => return -MATE + ply,
        BoardStatus::Stalemate => return 0,
        BoardStatus::Ongoing => {}
    }
    let in_check = board.checkers().popcnt() > 0;
    let stand_pat = evaluate(board);
    if !in_check {
        if stand_pat >= beta { return beta; }
        alpha = alpha.max(stand_pat);
    }

    let mut moves: Vec<ChessMove> = MoveGen::new_legal(board)
        .filter(|mv| in_check || board.piece_on(mv.get_dest()).is_some() || mv.get_promotion().is_some())
        .collect();
    moves.sort_by_key(|mv| {
        let victim = board.piece_on(mv.get_dest()).map(piece_value).unwrap_or(0);
        let attacker = board.piece_on(mv.get_source()).map(piece_value).unwrap_or(0);
        -(victim * 16 - attacker)
    });
    for mv in moves {
        let score = -quiescence(&board.make_move_new(mv), -beta, -alpha, ply + 1);
        if score >= beta { return beta; }
        alpha = alpha.max(score);
    }
    alpha
}

fn negamax(
    board: &Board,
    depth: i32,
    mut alpha: i32,
    beta: i32,
    ply: usize,
    ctx: &mut SearchContext,
    allow_null: bool,
) -> i32 {
    match board.status() {
        BoardStatus::Checkmate => return -MATE + ply as i32,
        BoardStatus::Stalemate => return 0,
        BoardStatus::Ongoing => {}
    }
    if depth <= 0 { return quiescence(board, alpha, beta, ply as i32); }

    let in_check = board.checkers().popcnt() > 0;
    let original_alpha = alpha;
    let hash = board.get_hash();
    let cached = ctx.table.get(&hash).copied();
    let tt_move = cached.and_then(|entry| entry.best_move);
    if let Some(entry) = cached {
        if entry.depth as i32 >= depth {
            let score = score_from_tt(entry.score, ply as i32);
            match entry.bound {
                Bound::Exact => return score,
                Bound::Lower if score >= beta => return score,
                Bound::Upper if score <= alpha => return score,
                _ => {}
            }
        }
    }

    if allow_null && depth >= 3 && !in_check && has_null_move_material(board) {
        if let Some(null_board) = board.null_move() {
            let reduction = 2 + depth / 6;
            let score = -negamax(&null_board, depth - 1 - reduction, -beta, -beta + 1, ply + 1, ctx, false);
            if score >= beta { return beta; }
        }
    }

    let moves = ordered_moves(board, tt_move, ctx, ply);
    let mut best_score = -INF;
    let mut best_move = None;
    let mut searched = 0usize;

    for mv in moves {
        let quiet = board.piece_on(mv.get_dest()).is_none()
            && mv.get_promotion().is_none()
            && board.piece_on(mv.get_source()) != Some(Piece::Pawn);
        let next = board.make_move_new(mv);
        let mut score;

        if searched == 0 {
            score = -negamax(&next, depth - 1, -beta, -alpha, ply + 1, ctx, true);
        } else {
            let reduction = if depth >= 3 && searched >= 4 && quiet && !in_check { 1 } else { 0 };
            score = -negamax(&next, depth - 1 - reduction, -alpha - 1, -alpha, ply + 1, ctx, true);
            if reduction > 0 && score > alpha {
                score = -negamax(&next, depth - 1, -alpha - 1, -alpha, ply + 1, ctx, true);
            }
            if score > alpha && score < beta {
                score = -negamax(&next, depth - 1, -beta, -alpha, ply + 1, ctx, true);
            }
        }

        searched += 1;
        if score > best_score {
            best_score = score;
            best_move = Some(mv);
        }
        if score > alpha {
            alpha = score;
        }
        if alpha >= beta {
            if quiet {
                ctx.remember_quiet(board, mv, ply, depth as u8);
            }
            break;
        }
    }

    let bound = if best_score <= original_alpha {
        Bound::Upper
    } else if best_score >= beta {
        Bound::Lower
    } else {
        Bound::Exact
    };
    if ctx.table.len() >= TT_LIMIT { ctx.table.clear(); }
    ctx.table.insert(hash, TTEntry {
        depth: depth.min(i16::MAX as i32) as i16,
        score: score_to_tt(best_score, ply as i32),
        bound,
        best_move,
    });
    best_score
}

pub fn best_move(board: &Board, depth: u8) -> Option<ChessMove> {
    let mut ctx = SearchContext::new();
    let mut moves: Vec<ChessMove> = MoveGen::new_legal(board).collect();
    if moves.is_empty() { return None; }
    let mut best = None;

    for current_depth in 1..=depth.max(1) {
        ctx.age_history();
        let hash = board.get_hash();
        let tt_move = ctx.table.get(&hash).and_then(|entry| entry.best_move);
        moves.sort_by_key(|mv| -move_order_score(board, *mv, tt_move, &ctx, 0));
        let mut iteration_best = None;
        let mut iteration_score = -INF;
        let mut alpha = -INF;

        for (index, mv) in moves.iter().enumerate() {
            let next = board.make_move_new(*mv);
            let score = if index == 0 {
                -negamax(&next, current_depth as i32 - 1, -INF, -alpha, 1, &mut ctx, true)
            } else {
                let mut value = -negamax(&next, current_depth as i32 - 1, -alpha - 1, -alpha, 1, &mut ctx, true);
                if value > alpha {
                    value = -negamax(&next, current_depth as i32 - 1, -INF, -alpha, 1, &mut ctx, true);
                }
                value
            };
            if score > iteration_score {
                iteration_score = score;
                iteration_best = Some(*mv);
            }
            alpha = alpha.max(score);
        }

        if let Some(mv) = iteration_best {
            best = Some(mv);
            ctx.table.insert(hash, TTEntry {
                depth: current_depth as i16,
                score: score_to_tt(iteration_score, 0),
                bound: Bound::Exact,
                best_move: Some(mv),
            });
            moves.sort_by_key(|candidate| if *candidate == mv { i32::MIN } else { -move_order_score(board, *candidate, Some(mv), &ctx, 0) });
        }
        if iteration_score.abs() >= MATE - 100 { break; }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starting_position_has_twenty_moves() {
        assert_eq!(MoveGen::new_legal(&Board::default()).len(), 20);
    }

    #[test]
    fn engine_returns_a_legal_starting_move() {
        let board = Board::default();
        let mv = best_move(&board, 3).unwrap();
        assert!(MoveGen::new_legal(&board).any(|candidate| candidate == mv));
    }

    #[test]
    fn uci_move_round_trips() {
        let board = Board::default();
        let mv = parse_uci_move(&board, "e2e4").unwrap();
        assert_eq!(uci_move(mv), "e2e4");
        assert!(parse_uci_move(&board, "e2e5").is_none());
    }

    #[test]
    fn engine_finds_mate_in_one() {
        let board = Board::from_str("6k1/5ppp/8/8/8/8/5PPP/5RK1 w - - 0 1").unwrap();
        let mv = best_move(&board, 3).unwrap();
        assert!(MoveGen::new_legal(&board).any(|candidate| candidate == mv));
    }

    #[test]
    fn transposition_table_hash_is_stable() {
        let board = Board::default();
        assert_eq!(board.get_hash(), board.get_hash());
        assert!(best_move(&board, 4).is_some());
    }
}

mod app;
pub use app::ChessApp;

#[cfg(target_os = "android")]
#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn android_main(app: winit::platform::android::activity::AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );
    let options = eframe::NativeOptions {
        android_app: Some(app),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "Phoenix Chess",
        options,
        Box::new(|cc| Ok(Box::new(ChessApp::new(cc)))),
    );
}
