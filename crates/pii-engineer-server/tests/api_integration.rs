use axum::http::StatusCode;
use axum::body::Body;
use axum::http::Request;
use tower::ServiceExt;
use serde_json::Value;

fn build_app() -> axum::Router {
    use axum::{routing::{get, post}, Json, Router};
    use serde_json::json;

    async fn health() -> Json<Value> {
        Json(json!({
            "status": "ok",
            "version": env!("CARGO_PKG_VERSION"),
            "gliner_loaded": false,
            "chinese_loaded": false,
        }))
    }

    async fn detect_no_model() -> (StatusCode, Json<Value>) {
        (StatusCode::SERVICE_UNAVAILABLE, Json(json!({"error": "model_unavailable"})))
    }

    Router::new()
        .route("/api/health", get(health))
        .route("/api/detect", post(detect_no_model))
}

#[tokio::test]
async fn health_endpoint() {
    let app = build_app();
    let req = Request::builder()
        .uri("/api/health")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");
}

#[tokio::test]
async fn detect_without_model_returns_503() {
    let app = build_app();
    let req = Request::builder()
        .method("POST")
        .uri("/api/detect")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"text":"John Doe"}"#))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn unknown_route_returns_404() {
    let app = build_app();
    let req = Request::builder()
        .uri("/api/nonexistent")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
