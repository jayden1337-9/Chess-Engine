use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use eframe::{WebOptions, WebRunner};

use crate::ChessApp;

#[wasm_bindgen(start)]
pub async fn start() -> Result<(), JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("window unavailable"))?;
    let document = window.document().ok_or_else(|| JsValue::from_str("document unavailable"))?;
    let body = document.body().ok_or_else(|| JsValue::from_str("document body unavailable"))?;

    body.style().set_property("margin", "0")?;
    body.style().set_property("background", "#101318")?;
    body.style().set_property("overflow", "hidden")?;

    let canvas = document
        .create_element("canvas")?
        .dyn_into::<web_sys::HtmlCanvasElement>()?;
    canvas.set_id("chess-engine-canvas");
    canvas.style().set_property("width", "100vw")?;
    canvas.style().set_property("height", "100vh")?;
    canvas.style().set_property("display", "block")?;
    canvas.style().set_property("touch-action", "none")?;
    body.append_child(&canvas)?;

    WebRunner::new()
        .start(
            canvas,
            WebOptions::default(),
            Box::new(|cc| Ok(Box::new(ChessApp::new(cc)))),
        )
        .await
}
