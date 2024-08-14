use axum::Json;
use serde::Serialize;
use serde_json::{json, Value};

pub fn api_ok<T: Serialize>(data: T) -> Json<Value> {
    Json(json!({
        "status": "ok",
        "data": data
    }))
}
pub fn api_error(msg: &str) -> Json<Value> {
    Json(json!({
        "status": "error",
        "message": msg
    }))
}
