mod config;
mod download;
mod middleware;
mod routes;
mod state;

use std::net::SocketAddr;

use anyhow::Result;
use axum::{middleware as axmw, Router};
use tower_http::services::ServeDir;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[cfg(unix)]
fn lock_memory() {
    unsafe {
        // MCL_CURRENT only — lock model weights already in RAM.
        // MCL_FUTURE would also lock temporary inference buffers, bloating RSS.
        if libc::mlockall(libc::MCL_CURRENT) == 0 {
            tracing::info!("mlockall: model weights locked in RAM");
        } else {
            tracing::warn!("mlockall failed — run with CAP_IPC_LOCK or as root to prevent swap");
        }
    }
}

use crate::config::Settings;
use crate::state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let settings = Settings::from_env();
    let addr: SocketAddr = format!("{}:{}", settings.host, settings.port).parse()?;
    info!(?addr, "pii-engineer starting");

    if std::env::var("ORT_DYLIB_PATH").is_err() {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()));
        let candidates = [
            exe_dir.as_ref().map(|d| d.join("libonnxruntime.dylib")),
            exe_dir.as_ref().map(|d| d.join("libonnxruntime.so")),
            Some(std::path::PathBuf::from("lib/libonnxruntime.dylib")),
            Some(std::path::PathBuf::from("lib/libonnxruntime.so")),
        ];
        for candidate in candidates.iter().flatten() {
            if candidate.exists() {
                std::env::set_var("ORT_DYLIB_PATH", candidate);
                info!(path = %candidate.display(), "auto-detected ORT_DYLIB_PATH");
                break;
            }
        }
    }

    let state = AppState::new(settings)?;

    // Lock model weights in RAM after loading + warmup, before serving
    #[cfg(unix)]
    lock_memory();

    let static_path = std::env::var("PII_ENGINEER_STATIC_DIR").unwrap_or_else(|_| "static".into());

    let app: Router = routes::router(state.clone())
        .nest_service("/static", ServeDir::new(&static_path))
        .layer(axmw::from_fn_with_state(state.clone(), middleware::rate_limit))
        .layer(axmw::from_fn(middleware::request_id))
        .layer(axmw::from_fn(middleware::global_error));

    // Periodic warmup to keep ONNX models in CPU cache / OS page tables
    {
        let st = state.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            interval.tick().await; // skip first (already warmed up at init)
            loop {
                interval.tick().await;
                let st2 = st.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    if let Some(ref g) = st2.gliner {
                        g.warm_up();
                    }
                    if let Some(ref c) = st2.chinese {
                        c.warm_up();
                    }
                }).await;
            }
        });
    }

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await?;
    Ok(())
}
