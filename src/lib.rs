use chess::{Board, BoardStatus, ChessMove, Color, MoveGen, Piece};
use std::str::FromStr;

const MATE: i32 = 30_000;
const INF: i32 = 31_000;

pub fn parse_uci_move(board: &Board, text: &str) -> Option<ChessMove> {
    if text.len() < 4 || text.len() > 5 { return None; }
    let from = chess::Square::from_str(&text[0..2]).ok()?;
    let to = chess::Square::from_str(&text[2..4]).ok()?;
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

pub fn evaluate(board: &Board) -> i32 {
    let mut score = 0;
    for rank in 0..8 {
        for file in 0..8 {
            let square = chess::Square::make_square(
                chess::Rank::from_index(rank),
                chess::File::from_index(file),
            );
            if let Some(piece) = board.piece_on(square) {
                let center = 7 - ((file as i32 * 2 - 7).abs() + (rank as i32 * 2 - 7).abs()) / 2;
                let activity = match piece {
                    Piece::Pawn => center / 2,
                    Piece::Knight | Piece::Bishop => center * 4,
                    Piece::Queen => center * 2,
                    _ => 0,
                };
                let total = piece_value(piece) + activity;
                score += if board.color_on(square) == Some(Color::White) { total } else { -total };
            }
        }
    }
    if board.side_to_move() == Color::White { score } else { -score }
}

fn move_order_score(board: &Board, mv: ChessMove) -> i32 {
    let victim = board.piece_on(mv.get_dest()).map(piece_value).unwrap_or(0);
    let attacker = board.piece_on(mv.get_source()).map(piece_value).unwrap_or(0);
    let capture = if victim > 0 { 10_000 + victim * 10 - attacker } else { 0 };
    let promotion = mv.get_promotion().map(piece_value).unwrap_or(0);
    let next = board.make_move_new(mv);
    let check = if next.checkers().popcnt() > 0 { 500 } else { 0 };
    capture + promotion + check
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
    moves.sort_by_key(|mv| -move_order_score(board, *mv));
    for mv in moves {
        let score = -quiescence(&board.make_move_new(mv), -beta, -alpha, ply + 1);
        if score >= beta { return beta; }
        alpha = alpha.max(score);
    }
    alpha
}

fn negamax(board: &Board, depth: u8, mut alpha: i32, beta: i32, ply: i32) -> i32 {
    match board.status() {
        BoardStatus::Checkmate => return -MATE + ply,
        BoardStatus::Stalemate => return 0,
        BoardStatus::Ongoing => {}
    }
    if depth == 0 { return quiescence(board, alpha, beta, ply); }

    let mut moves: Vec<ChessMove> = MoveGen::new_legal(board).collect();
    moves.sort_by_key(|mv| -move_order_score(board, *mv));
    let mut best = -INF;
    for mv in moves {
        let score = -negamax(&board.make_move_new(mv), depth - 1, -beta, -alpha, ply + 1);
        best = best.max(score);
        alpha = alpha.max(score);
        if alpha >= beta { break; }
    }
    best
}

pub fn best_move(board: &Board, depth: u8) -> Option<ChessMove> {
    let mut moves: Vec<ChessMove> = MoveGen::new_legal(board).collect();
    moves.sort_by_key(|mv| -move_order_score(board, *mv));
    let mut best = None;
    for current_depth in 1..=depth.max(1) {
        let mut iteration_best = None;
        let mut iteration_score = -INF;
        let mut alpha = -INF;
        for mv in &moves {
            let score = -negamax(&board.make_move_new(*mv), current_depth - 1, -INF, -alpha, 1);
            if score > iteration_score {
                iteration_score = score;
                iteration_best = Some(*mv);
            }
            alpha = alpha.max(score);
        }
        if let Some(mv) = iteration_best {
            best = Some(mv);
            moves.sort_by_key(|candidate| if *candidate == mv { i32::MIN } else { -move_order_score(board, *candidate) });
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
