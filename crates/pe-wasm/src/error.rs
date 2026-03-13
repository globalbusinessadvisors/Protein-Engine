//! Error types and helpers for WASM boundary conversion.

use serde::Serialize;
use wasm_bindgen::JsValue;

/// Lightweight error wrapper for WASM-exported functions.
#[derive(Debug, Serialize)]
pub struct WasmErrorResponse {
    pub error: String,
}

/// Convert any error message into a JsValue containing `{error: "..."}` JSON.
pub fn error_js(msg: impl Into<String>) -> JsValue {
    let resp = WasmErrorResponse {
        error: msg.into(),
    };
    JsValue::from_str(&serde_json::to_string(&resp).unwrap_or_else(|_| {
        r#"{"error":"serialization failed"}"#.to_string()
    }))
}

/// Convert a Rust Result<T: Serialize, String> into Result<JsValue, JsValue>.
pub fn to_js_result<T: Serialize>(result: Result<T, String>) -> Result<JsValue, JsValue> {
    match result {
        Ok(val) => serde_json::to_string(&val)
            .map(|s| JsValue::from_str(&s))
            .map_err(|e| error_js(e.to_string())),
        Err(e) => Err(error_js(e)),
    }
}
