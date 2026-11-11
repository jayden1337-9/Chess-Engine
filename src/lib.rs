use chess::{Board, BoardStatus, ChessMove, Color, MoveGen, Piece};
use std::str::FromStr;

const MATE: i32 = 30_000;

pub fn parse_uci_move(board: &Board, text: &str) -> Option<ChessMove> {
    if text.len() < 4 || text.len() > 5 {
        return None;
    }
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
        text.push(match piece {
            Piece::Queen => 'q',
            Piece::Rook => 'r',
            Piece::Bishop => 'b',
            Piece::Knight => 'n',
            _ => 'q',
        });
    }
    text
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
                let value = match piece {
                    Piece::Pawn => 100,
                    Piece::Knight => 320,
                    Piece::Bishop => 335,
                    Piece::Rook => 500,
                    Piece::Queen => 900,
                    Piece::King => 0,
                };
                let center = 7 - ((file as i32 * 2 - 7).abs() + (rank as i32 * 2 - 7).abs()) / 2;
                let activity = match piece {
                    Piece::Pawn => center / 2,
                    Piece::Knight | Piece::Bishop => center * 4,
                    Piece::Queen => center * 2,
                    _ => 0,
                };
                let total = value + activity;
                score += if board.color_on(square) == Some(Color::White) { total } else { -total };
            }
        }
    }
    if board.side_to_move() == Color::White { score } else { -score }
}

fn negamax(board: &Board, depth: u8, mut alpha: i32, beta: i32, ply: i32) -> i32 {
    match board.status() {
        BoardStatus::Checkmate => return -MATE + ply,
        BoardStatus::Stalemate => return 0,
        BoardStatus::Ongoing => {}
    }
    if depth == 0 {
        return evaluate(board);
    }

    let mut moves: Vec<ChessMove> = MoveGen::new_legal(board).collect();
    moves.sort_by_key(|mv| if board.piece_on(mv.get_dest()).is_some() { 0 } else { 1 });
    let mut best = -MATE;
    for mv in moves {
        let next = board.make_move_new(mv);
        let score = -negamax(&next, depth - 1, -beta, -alpha, ply + 1);
        best = best.max(score);
        alpha = alpha.max(score);
        if alpha >= beta {
            break;
        }
    }
    best
}

pub fn best_move(board: &Board, depth: u8) -> Option<ChessMove> {
    let mut moves: Vec<ChessMove> = MoveGen::new_legal(board).collect();
    moves.sort_by_key(|mv| if board.piece_on(mv.get_dest()).is_some() { 0 } else { 1 });
    let mut best = None;
    let mut best_score = -MATE;
    let mut alpha = -MATE;
    for mv in moves {
        let next = board.make_move_new(mv);
        let score = -negamax(&next, depth.max(1) - 1, -MATE, -alpha, 1);
        if score > best_score {
            best_score = score;
            best = Some(mv);
        }
        alpha = alpha.max(score);
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
}

mod app;
pub use app::ChessApp;

#[cfg(target_os = "android")]
#[no_mangle]
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
