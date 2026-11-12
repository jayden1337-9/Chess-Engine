use chess_engine::ChessApp;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Chess Engine",
        options,
        Box::new(|cc| Ok(Box::new(ChessApp::new(cc)))),
    )
}
