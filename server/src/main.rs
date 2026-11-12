use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use chess::{Board, ChessMove, MoveGen};
use futures_util::{SinkExt, StreamExt};
use rand::{distributions::Alphanumeric, Rng};
use serde::Deserialize;
use serde_json::json;
use std::{collections::HashMap, net::SocketAddr, str::FromStr, sync::Arc};
use tokio::sync::{mpsc, Mutex};

type Tx = mpsc::UnboundedSender<Message>;

#[derive(Clone)]
struct AppState {
    rooms: Arc<Mutex<HashMap<String, Room>>>,
}

struct Room {
    board: Board,
    white: Tx,
    black: Option<Tx>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMessage {
    Create,
    Join { code: String },
    Move { uci: String },
    Resign,
    DrawOffer,
}

#[tokio::main]
async fn main() {
    let state = AppState {
        rooms: Arc::new(Mutex::new(HashMap::new())),
    };
    let app = Router::new()
        .route("/health", get(|| async { "Chess Engine multiplayer server online" }))
        .route("/ws", get(ws_handler))
        .with_state(state);
    let addr: SocketAddr = std::env::var("BIND_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:8080".to_owned())
        .parse()
        .expect("BIND_ADDR must be a valid socket address");
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind server");
    println!("Chess Engine multiplayer server listening on {addr}");
    axum::serve(listener, app).await.expect("serve");
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn send(tx: &Tx, value: serde_json::Value) {
    let _ = tx.send(Message::Text(value.to_string()));
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sink, mut stream) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
    let writer = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            if sink.send(message).await.is_err() {
                break;
            }
        }
    });

    let mut room_code: Option<String> = None;
    let mut is_white = false;

    while let Some(Ok(message)) = stream.next().await {
        let Message::Text(text) = message else { continue };
        let Ok(request) = serde_json::from_str::<ClientMessage>(&text) else {
            send(&tx, json!({"type":"error","message":"Invalid message"})).await;
            continue;
        };

        match request {
            ClientMessage::Create => {
                if room_code.is_some() {
                    send(&tx, json!({"type":"error","message":"Already in a room"})).await;
                    continue;
                }
                let mut rooms = state.rooms.lock().await;
                let code = loop {
                    let candidate: String = rand::thread_rng()
                        .sample_iter(&Alphanumeric)
                        .take(6)
                        .map(char::from)
                        .map(|c| c.to_ascii_uppercase())
                        .collect();
                    if !rooms.contains_key(&candidate) { break candidate; }
                };
                rooms.insert(code.clone(), Room {
                    board: Board::default(),
                    white: tx.clone(),
                    black: None,
                });
                room_code = Some(code.clone());
                is_white = true;
                send(&tx, json!({"type":"created","code":code,"color":"white","fen":Board::default().to_string()})).await;
            }
            ClientMessage::Join { code } => {
                if room_code.is_some() {
                    send(&tx, json!({"type":"error","message":"Already in a room"})).await;
                    continue;
                }
                let code = code.trim().to_ascii_uppercase();
                let mut rooms = state.rooms.lock().await;
                if let Some(room) = rooms.get_mut(&code) {
                    if room.black.is_some() {
                        send(&tx, json!({"type":"error","message":"Room is full"})).await;
                        continue;
                    }
                    room.black = Some(tx.clone());
                    let fen = room.board.to_string();
                    let white = room.white.clone();
                    room_code = Some(code.clone());
                    is_white = false;
                    send(&tx, json!({"type":"joined","code":code,"color":"black","fen":fen})).await;
                    send(&white, json!({"type":"opponent_joined","fen":fen})).await;
                } else {
                    send(&tx, json!({"type":"error","message":"Room not found"})).await;
                }
            }
            ClientMessage::Move { uci } => {
                let Some(code) = room_code.as_ref() else {
                    send(&tx, json!({"type":"error","message":"Join a room first"})).await;
                    continue;
                };
                let mut rooms = state.rooms.lock().await;
                let Some(room) = rooms.get_mut(code) else { continue };
                let expected = if is_white { chess::Color::White } else { chess::Color::Black };
                if room.board.side_to_move() != expected {
                    send(&tx, json!({"type":"error","message":"Not your turn"})).await;
                    continue;
                }
                let Ok(mv) = ChessMove::from_str(&uci) else {
                    send(&tx, json!({"type":"error","message":"Invalid move format"})).await;
                    continue;
                };
                if !MoveGen::new_legal(&room.board).any(|legal| legal == mv) {
                    send(&tx, json!({"type":"error","message":"Illegal move"})).await;
                    continue;
                }
                room.board = room.board.make_move_new(mv);
                let update = json!({"type":"move","uci":uci,"fen":room.board.to_string()});
                send(&room.white, update.clone()).await;
                if let Some(black) = &room.black { send(black, update).await; }
            }
            ClientMessage::Resign => {
                let Some(code) = room_code.as_ref() else { continue };
                let rooms = state.rooms.lock().await;
                if let Some(room) = rooms.get(code) {
                    let result = json!({"type":"game_over","result":if is_white {"black_wins"} else {"white_wins"},"reason":"resignation"});
                    send(&room.white, result.clone()).await;
                    if let Some(black) = &room.black { send(black, result).await; }
                }
            }
            ClientMessage::DrawOffer => {
                let Some(code) = room_code.as_ref() else { continue };
                let rooms = state.rooms.lock().await;
                if let Some(room) = rooms.get(code) {
                    let offer = json!({"type":"draw_offer"});
                    if is_white {
                        if let Some(black) = &room.black { send(black, offer).await; }
                    } else {
                        send(&room.white, offer).await;
                    }
                }
            }
        }
    }

    if let Some(code) = room_code {
        let mut rooms = state.rooms.lock().await;
        if let Some(room) = rooms.get_mut(&code) {
            let notice = json!({"type":"opponent_disconnected"});
            if is_white {
                if let Some(black) = &room.black { send(black, notice).await; }
                rooms.remove(&code);
            } else {
                send(&room.white, notice).await;
                if let Some(room) = rooms.get_mut(&code) { room.black = None; }
            }
        }
    }
    writer.abort();
}
