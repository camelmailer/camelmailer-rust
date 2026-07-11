//! Shared helpers for the wiremock-based test suites.
#![allow(dead_code)] // not every suite uses every helper

use camelmailer_rs::CamelMailer;
use serde_json::{json, Value};
use wiremock::MockServer;

pub const API_KEY: &str = "cm_test_key";

/// A client pointed at the given mock server.
pub fn client(server: &MockServer) -> CamelMailer {
    CamelMailer::builder(API_KEY)
        .base_url(server.uri())
        .build()
        .expect("valid test configuration")
}

/// Wrap `data` in the standard success envelope.
pub fn success(data: Value) -> Value {
    json!({ "status": "success", "time": 0.003, "data": data })
}

/// The standard error envelope.
pub fn error(code: &str, message: &str) -> Value {
    json!({
        "status": "error",
        "time": 0.003,
        "error": { "code": code, "message": message }
    })
}
