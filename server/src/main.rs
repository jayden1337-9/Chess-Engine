use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use chess::{Board, ChessMove, MoveGen};
use futures_util::{SinkExt, StreamExt};
use rand::{distributions::Alphanumeric, Rng};
use serde::Deserialize;
use serde_json::json;
use std::{collections::HashMap, net::SocketAddr, path::Path, str::FromStr, sync::{Arc, atomic::{AtomicU64, Ordering}}, time::{SystemTime, UNIX_EPOCH}};
use tokio::sync::{mpsc, Mutex};

type Tx = mpsc::UnboundedSender<Message>;

#[derive(Clone)]
struct AppState {
    rooms: Arc<Mutex<HashMap<String, Room>>>,
    clients: Arc<Mutex<HashMap<u64, Tx>>>,
    persistence_lock: Arc<Mutex<()>>,
}

static NEXT_CLIENT_ID: AtomicU64 = AtomicU64::new(1);

// The browser shell is returned by the Rust server; the JavaScript glue and
// WebAssembly binary are generated from the Rust app during the release build.
const WEB_JS: &[u8] = include_bytes!("../../web/pkg/chess_engine.js");
const WEB_WASM: &[u8] = include_bytes!("../../web/pkg/chess_engine_bg.wasm");

async fn index() -> Html<&'static str> {
    Html(r##"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1,maximum-scale=1,user-scalable=no"><meta name="theme-color" content="#101318"><title>Chess Engine</title></head><body><script type="module">import init from '/app/chess_engine.js'; init();</script></body></html>"##)
}

async fn wasm_javascript() -> Response {
    Response::builder()
        .header(axum::http::header::CONTENT_TYPE, "text/javascript; charset=utf-8")
        .header(axum::http::header::CACHE_CONTROL, "public, max-age=300")
        .body(axum::body::Body::from(WEB_JS))
        .expect("valid JavaScript response")
}

async fn wasm_binary() -> Response {
    Response::builder()
        .header(axum::http::header::CONTENT_TYPE, "application/wasm")
        .header(axum::http::header::CACHE_CONTROL, "public, max-age=300")
        .body(axum::body::Body::from(WEB_WASM))
        .expect("valid WebAssembly response")
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
    Chat { scope: String, name: String, message: String },
}

#[tokio::main]
async fn main() {
    let state = AppState {
        rooms: Arc::new(Mutex::new(HashMap::new())),
        clients: Arc::new(Mutex::new(HashMap::new())),
        persistence_lock: Arc::new(Mutex::new(())),
    };
    let app = Router::new()
        .route("/", get(index))
        .route("/app/chess_engine.js", get(wasm_javascript))
        .route("/app/chess_engine_bg.wasm", get(wasm_binary))
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

// Append-only JSONL storage. Existing records are never truncated or removed.
async fn persist_event(state: &AppState, event: serde_json::Value) {
    let _guard = state.persistence_lock.lock().await;
    let path = std::env::var("CHESS_DATA_FILE")
        .unwrap_or_else(|_| "data/chess_events.jsonl".to_owned());
    let path = Path::new(&path);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(error) = tokio::fs::create_dir_all(parent).await {
                eprintln!("Could not create persistence directory: {error}");
                return;
            }
        }
    }
    let mut record = event;
    record["saved_at_unix"] = json!(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs());
    let mut line = match serde_json::to_vec(&record) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("Could not encode persistence event: {error}");
            return;
        }
    };
    line.push(b'\\n');
    use tokio::io::AsyncWriteExt;
    match tokio::fs::OpenOptions::new().create(true).append(true).open(path).await {
        Ok(mut file) => {
            if let Err(error) = file.write_all(&line).await {
                eprintln!("Could not append persistence event: {error}");
            }
            if let Err(error) = file.flush().await {
                eprintln!("Could not flush persistence event: {error}");
            }
        }
        Err(error) => eprintln!("Could not open persistence file: {error}"),
    }
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

    let client_id = NEXT_CLIENT_ID.fetch_add(1, Ordering::Relaxed);
    state.clients.lock().await.insert(client_id, tx.clone());
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
                persist_event(&state, json!({"event":"room_created","room":code,"initial_fen":Board::default().to_string()})).await;
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
                    persist_event(&state, json!({"event":"room_joined","room":code})).await;
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
                if room.black.is_none() {
                    send(&tx, json!({"type":"error","message":"Waiting for an opponent to join"})).await;
                    continue;
                }
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
                persist_event(&state, json!({"event":"move","room":code,"uci":uci.clone(),"fen":room.board.to_string()})).await;
                let update = json!({"type":"move","uci":uci,"fen":room.board.to_string()});
                if is_white {
                    if let Some(black) = &room.black { send(black, update).await; }
                } else {
                    send(&room.white, update).await;
                }
                match room.board.status() {
                    chess::BoardStatus::Checkmate => {
                        let winner = if room.board.side_to_move() == chess::Color::White {"black_wins"} else {"white_wins"};
                        persist_event(&state, json!({"event":"game_over","room":code,"result":winner,"reason":"checkmate"})).await;
                        let result = json!({"type":"game_over","result":if room.board.side_to_move() == chess::Color::White {"black_wins"} else {"white_wins"},"reason":"checkmate"});
                        send(&room.white, result.clone()).await;
                        if let Some(black) = &room.black { send(black, result).await; }
                    }
                    chess::BoardStatus::Stalemate => {
                        persist_event(&state, json!({"event":"game_over","room":code,"result":"draw","reason":"stalemate"})).await;
                        let result = json!({"type":"game_over","result":"draw","reason":"stalemate"});
                        send(&room.white, result.clone()).await;
                        if let Some(black) = &room.black { send(black, result).await; }
                    }
                    chess::BoardStatus::Ongoing => {}
                }
            }
            ClientMessage::Resign => {
                let Some(code) = room_code.as_ref() else { continue };
                let rooms = state.rooms.lock().await;
                if let Some(room) = rooms.get(code) {
                    let result = json!({"type":"game_over","result":if is_white {"black_wins"} else {"white_wins"},"reason":"resignation"});
                    persist_event(&state, json!({"event":"game_over","room":code,"result":if is_white {"black_wins"} else {"white_wins"},"reason":"resignation"})).await;
                    send(&room.white, result.clone()).await;
                    if let Some(black) = &room.black { send(black, result).await; }
                }
            }
            ClientMessage::Chat { scope, name, message } => {
                let message = message.trim();
                let name = name.trim();
                if message.is_empty() || message.chars().count() > 500 || name.is_empty() || name.chars().count() > 24 {
                    send(&tx, json!({"type":"error","message":"Chat name or message is invalid (message max 500 characters, name max 24)"})).await;
                    continue;
                }
                let payload = json!({
                    "type":"chat",
                    "scope":scope.clone(),
                    "name":name,
                    "message":message
                });
                persist_event(&state, json!({"event":"chat","scope":scope.clone(),"room":room_code.clone(),"name":name,"message":message})).await;
                match scope.as_str() {
                    "global" => {
                        let clients = state.clients.lock().await;
                        for client in clients.values() {
                            send(client, payload.clone()).await;
                        }
                    }
                    "match" => {
                        let Some(code) = room_code.as_ref() else {
                            send(&tx, json!({"type":"error","message":"Join a match room before using match chat"})).await;
                            continue;
                        };
                        let rooms = state.rooms.lock().await;
                        if let Some(room) = rooms.get(code) {
                            send(&room.white, payload.clone()).await;
                            if let Some(black) = &room.black {
                                send(black, payload).await;
                            }
                        } else {
                            send(&tx, json!({"type":"error","message":"Match room not found"})).await;
                        }
                    }
                    _ => send(&tx, json!({"type":"error","message":"Unknown chat scope"})).await,
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

    state.clients.lock().await.remove(&client_id);
    if let Some(code) = room_code {
        let mut rooms = state.rooms.lock().await;
        if is_white {
            if let Some(room) = rooms.remove(&code) {
                let notice = json!({"type":"opponent_disconnected"});
                if let Some(black) = room.black { send(&black, notice).await; }
            }
        } else if let Some(room) = rooms.get_mut(&code) {
            send(&room.white, json!({"type":"opponent_disconnected"})).await;
            room.black = None;
        }
    }
    writer.abort();
}
