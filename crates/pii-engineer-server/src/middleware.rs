//! HTTP middleware: request ID, per-IP rate limit, error handling.

use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use axum::{
    body::Body,
    extract::{ConnectInfo, Request, State},
    http::{HeaderName, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::state::AppState;

const PROTECTED_PATHS: &[&str] = &["/api/detect"];
const REQUEST_ID_HEADER: &str = "x-request-id";

fn random_id() -> String {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{nanos:016x}")
}

pub async fn request_id(mut req: Request, next: Next) -> Response {
    let id = req
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(random_id);
    req.extensions_mut().insert(RequestId(id.clone()));
    let mut resp = next.run(req).await;
    if let Ok(v) = HeaderValue::from_str(&id) {
        resp.headers_mut()
            .insert(HeaderName::from_static(REQUEST_ID_HEADER), v);
    }
    resp
}

#[derive(Clone)]
pub struct RequestId(pub String);

fn is_protected(path: &str) -> bool {
    PROTECTED_PATHS.contains(&path)
}


#[derive(Default)]
pub struct RateLimiter {
    inner: Mutex<HashMap<String, VecDeque<Instant>>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self::default()
    }

    fn check(&self, ip: &str, rpm: u32, now: Instant) -> bool {
        if rpm == 0 {
            return true;
        }
        let mut map = self.inner.lock().expect("rate map");
        let q = map.entry(ip.to_string()).or_default();
        let cutoff = now.checked_sub(Duration::from_secs(60)).unwrap_or(now);
        while let Some(&front) = q.front() {
            if front < cutoff {
                q.pop_front();
            } else {
                break;
            }
        }
        if q.len() as u32 >= rpm {
            return false;
        }
        q.push_back(now);
        true
    }
}

pub async fn rate_limit(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request,
    next: Next,
) -> Response {
    if !is_protected(req.uri().path()) {
        return next.run(req).await;
    }
    let ip = addr.ip().to_string();
    if !state.limiter.check(&ip, state.settings.rate_limit_rpm, Instant::now()) {
        let mut resp = (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({"error":"rate_limit_exceeded","detail":format!("max {} req/min", state.settings.rate_limit_rpm)})),
        )
            .into_response();
        resp.headers_mut()
            .insert("retry-after", HeaderValue::from_static("60"));
        resp
    } else {
        next.run(req).await
    }
}

pub async fn global_error(req: Request, next: Next) -> Response {
    let id = req
        .extensions()
        .get::<RequestId>()
        .map(|r| r.0.clone())
        .unwrap_or_default();
    let path = req.uri().path().to_string();
    let resp = next.run(req).await;
    if resp.status().is_server_error() {
        tracing::error!(request_id = %id, path = %path, status = %resp.status(), "server error");
    }
    let _ = Body::from("");
    resp
}
