use ewebsock::{WsEvent, WsMessage, WsReceiver, WsSender};
use serde_json::{json, Value};
use chess::Color;
use std::collections::VecDeque;

#[derive(Clone)]
pub struct ChatLine {
    pub scope: String,
    pub name: String,
    pub message: String,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum ChatTab {
    Global,
    Match,
}

pub struct ChatClient {
    pub url: String,
    pub name: String,
    pub room_code: String,
    pub status: String,
    pub input: String,
    pub tab: ChatTab,
    sender: Option<WsSender>,
    receiver: Option<WsReceiver>,
    opened: bool,
    pub lines: VecDeque<ChatLine>,
    pub player_color: Option<Color>,
    pub remote_moves: VecDeque<(String, String)>,
}

impl Default for ChatClient {
    fn default() -> Self {
        Self {
            url: "wss://chessengine.freesrv.com/ws".to_owned(),
            name: "Player".to_owned(),
            room_code: String::new(),
            status: "Not connected".to_owned(),
            input: String::new(),
            tab: ChatTab::Global,
            sender: None,
            receiver: None,
            opened: false,
            lines: VecDeque::new(),
            player_color: None,
            remote_moves: VecDeque::new(),
        }
    }
}

impl ChatClient {
    pub fn connect(&mut self) {
        let url = self.url.trim();
        if url.is_empty() {
            self.status = "Enter your multiplayer server address".to_owned();
            return;
        }
        self.sender = None;
        self.receiver = None;
        self.opened = false;
        match ewebsock::connect(url.to_owned(), ewebsock::Options::default()) {
            Ok((sender, receiver)) => {
                self.sender = Some(sender);
                self.receiver = Some(receiver);
                self.status = "Connecting...".to_owned();
            }
            Err(error) => self.status = format!("Connection failed: {error}"),
        }
    }

    pub fn poll(&mut self) {
        let mut events = Vec::new();
        if let Some(receiver) = &self.receiver {
            while let Some(event) = receiver.try_recv() {
                events.push(event);
            }
        }
        for event in events {
            match event {
                WsEvent::Opened => {
                    self.opened = true;
                    self.status = "Connected".to_owned();
                }
                WsEvent::Closed => {
                    self.opened = false;
                    self.status = "Disconnected".to_owned();
                    self.room_code.clear();
                    self.player_color = None;
                }
                WsEvent::Error(error) => self.status = format!("Connection error: {error}"),
                WsEvent::Message(WsMessage::Text(text)) => {
                    if let Ok(value) = serde_json::from_str::<Value>(&text) {
                        match value["type"].as_str().unwrap_or_default() {
                            "chat" => {
                                let scope = value["scope"].as_str().unwrap_or_default().to_owned();
                                let name = value["name"].as_str().unwrap_or("Player").to_owned();
                                let message = value["message"].as_str().unwrap_or_default().to_owned();
                                if scope == "global" || scope == "match" {
                                    self.lines.push_back(ChatLine { scope, name, message });
                                    while self.lines.len() > 100 { self.lines.pop_front(); }
                                }
                            }
                            "created" | "joined" => {
                                self.room_code = value["code"].as_str().unwrap_or_default().to_owned();
                                self.player_color = match value["color"].as_str() { Some("white") => Some(Color::White), Some("black") => Some(Color::Black), _ => self.player_color };
                                self.status = format!("Connected · room {}", self.room_code);
                            }
                            "error" => {
                                self.status = value["message"].as_str().unwrap_or("Server error").to_owned();
                            }
                            "move" => {
                                let uci = value["uci"].as_str().unwrap_or_default().to_owned();
                                let fen = value["fen"].as_str().unwrap_or_default().to_owned();
                                if !uci.is_empty() && !fen.is_empty() { self.remote_moves.push_back((uci, fen)); }
                            }
                            "opponent_joined" => { self.status = "Opponent connected · game ready".to_owned(); }
                            "game_over" => { self.status = format!("Game over: {}", value["reason"].as_str().unwrap_or("finished")); }
                            "draw_offer" => { self.status = "Opponent offered a draw".to_owned(); }
                            "opponent_disconnected" => {
                                self.status = "Opponent disconnected".to_owned();
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn send_value(&mut self, value: Value) {
        if !self.opened {
            self.status = "Connect to a server first".to_owned();
            return;
        }
        if let Some(sender) = &mut self.sender {
            sender.send(WsMessage::Text(value.to_string()));
        }
    }

    pub fn send_move(&mut self, uci: &str) {
        self.send_value(json!({"type":"move","uci":uci}));
    }

    pub fn take_remote_moves(&mut self) -> Vec<(String, String)> {
        self.remote_moves.drain(..).collect()
    }

    pub fn create_room(&mut self) {
        self.send_value(json!({"type":"create"}));
    }

    pub fn join_room(&mut self) {
        let code = self.room_code.trim().to_ascii_uppercase();
        if code.is_empty() {
            self.status = "Enter a room code".to_owned();
            return;
        }
        self.send_value(json!({"type":"join","code":code}));
    }

    pub fn send_chat(&mut self) {
        let message = self.input.trim().to_owned();
        if message.is_empty() { return; }
        if message.chars().count() > 500 {
            self.status = "Messages are limited to 500 characters".to_owned();
            return;
        }
        let scope = match self.tab {
            ChatTab::Global => "global",
            ChatTab::Match => "match",
        };
        if scope == "match" && self.room_code.is_empty() {
            self.status = "Create or join a room to use match chat".to_owned();
            return;
        }
        let name = self.name.trim();
        if name.is_empty() || name.chars().count() > 24 {
            self.status = "Name must be 1–24 characters".to_owned();
            return;
        }
        self.send_value(json!({
            "type":"chat",
            "scope":scope,
            "name":name,
            "message":message
        }));
        self.input.clear();
    }
}
