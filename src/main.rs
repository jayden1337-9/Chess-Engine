use chess::{Board, ChessMove};
use phoenix_chess::{best_move, parse_uci_move, uci_move};
use std::io::{self, BufRead, Write};
use std::str::FromStr;

fn main() {
    let stdin = io::stdin();
    let mut board = Board::default();

    for input in stdin.lock().lines() {
        let Ok(line) = input else { break };
        let mut parts = line.split_whitespace();
        match parts.next().unwrap_or("") {
            "uci" => {
                println!("id name Phoenix Chess");
                println!("id author Jayden");
                println!("uciok");
            }
            "isready" => println!("readyok"),
            "ucinewgame" => board = Board::default(),
            "position" => {
                let args: Vec<&str> = parts.collect();
                let mut index = 0;
                if args.first() == Some(&"startpos") {
                    board = Board::default();
                    index = 1;
                } else if args.first() == Some(&"fen") && args.len() >= 7 {
                    let fen = args[1..7].join(" ");
                    if let Ok(parsed) = Board::from_str(&fen) {
                        board = parsed;
                    }
                    index = 7;
                }
                if args.get(index) == Some(&"moves") {
                    for token in &args[index + 1..] {
                        if let Some(mv) = parse_uci_move(&board, token) {
                            board = board.make_move_new(mv);
                        }
                    }
                }
            }
            "go" => {
                let args: Vec<&str> = parts.collect();
                let depth = args.windows(2)
                    .find(|pair| pair[0] == "depth")
                    .and_then(|pair| pair[1].parse::<u8>().ok())
                    .unwrap_or(4)
                    .clamp(1, 12);
                let best = best_move(&board, depth)
                    .map(uci_move)
                    .unwrap_or_else(|| "0000".to_string());
                println!("bestmove {best}");
            }
            "quit" => break,
            _ => {}
        }
        let _ = io::stdout().flush();
    }
}
