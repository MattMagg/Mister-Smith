# HTTP Transport Layer with Axum v0.8

This document shows how to wire up an HTTP transport in MisterSmith using Axum v0.8.  
We’ll cover router setup (in pseudocode), middleware chains (auth, metrics/tracing, error handling), REST patterns, WebSockets, extractors, and error handling.  
For concrete implementations, we draw from Axum’s real examples; route definitions are presented in pseudocode to focus on the framework integration.

---

## 1. Dependencies

Add to your `Cargo.toml`:

```toml
[dependencies]
axum = { version = "0.8", features = ["ws"] }
tower = "0.4"
tower-http = { version = "0.3", features = ["trace", "auth", "error-handling"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
```

---

## 2. Shared Application State

```rust
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    // e.g. DB pool, config, etc.
    pub db: sqlx::PgPool,
    // ...other shared resources
}

type SharedState = Arc<AppState>;
```

---

## 3. Router Setup (Pseudocode)

We define routes in pseudocode but use real handler signatures and middleware layering.

```rust
use axum::{Router, routing::{get, post, put, delete}};
use tower::ServiceBuilder;
use tower_http::{
    trace::TraceLayer,
    auth::RequireAuthorizationLayer,
    error_handling::HandleErrorLayer,
};
use crate::{handlers, middleware::{AuthLayer, ErrorHandler}, AppState};

pub fn create_app(state: SharedState) -> Router {
    // Pseudocode for resource routes
    let api = Router::new()
        .route("/items", 
            get(handlers::list_items)
            .post(handlers::create_item)
        )
        .route("/items/:id", 
            get(handlers::get_item)
            .put(handlers::update_item)
            .delete(handlers::delete_item)
        )
        .route("/ws", get(handlers::ws_handler))
        // ... add more routes

    // Build middleware stack
    let middleware_stack = ServiceBuilder::new()
        // Observability: HTTP tracing for metrics & logs
        .layer(TraceLayer::new_for_http())
        // Attach shared state as an extractor
        .layer(axum::Extension(state.clone()))
        // Security: require valid auth (see security-framework.md)
        .layer(AuthLayer::new())
        // Error handling: maps internal errors to HTTP responses
        .layer(HandleErrorLayer::new(ErrorHandler::handle_rejection));

    api.layer(middleware_stack)
}
```

---

## 4. Handlers & Extractors

### Typical REST Handler

```rust
use axum::{
    extract::{Path, Query, Json, Extension},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct CreateItem {
    name: String,
    price: f64,
}

#[derive(Serialize)]
pub struct Item {
    id: i64,
    name: String,
    price: f64,
}

pub async fn create_item(
    Extension(state): Extension<SharedState>,
    Json(payload): Json<CreateItem>,
) -> Result<(StatusCode, Json<Item>), AppError> {
    // Real DB call, business logic, etc.
    let saved = state.db
        .fetch_one(/* ... */)
        .await
        .map_err(AppError::from)?;
    Ok((StatusCode::CREATED, Json(Item { /*...*/ })))
}

pub async fn get_item(
    Extension(state): Extension<SharedState>,
    Path(id): Path<i64>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Item>, AppError> {
    // Use params.get("verbose") etc.
    let item = /* fetch by id */.await.map_err(AppError::from)?;
    Ok(Json(item))
}
```

### Authentication Middleware (from `security-framework.md`)

```rust
use tower::{Layer, Service};
use axum::extract::RequestParts;
use axum::http::{Request, StatusCode};
use std::task::{Context, Poll};
use futures_util::future::BoxFuture;

pub struct AuthLayer;

impl AuthLayer {
    pub fn new() -> Self { Self }
}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthMiddleware { inner }
    }
}

pub struct AuthMiddleware<S> {
    inner: S,
}

impl<S, ReqBody> Service<Request<ReqBody>> for AuthMiddleware<S>
where
    S: Service<Request<ReqBody>, Response = axum::response::Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        // Real token validation logic:
        // If invalid: return early with 401
        // Else: forward to inner
        let mut svc = self.inner.clone();
        Box::pin(async move {
            // validate_auth(&req).await?;
            svc.call(req).await
        })
    }
}
```

---

## 5. WebSocket Support

```rust
use axum::{
    extract::ws::{WebSocket, WebSocketUpgrade},
    response::IntoResponse,
};
use crate::AppError;

pub async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    while let Some(msg) = socket.recv().await {
        match msg {
            Ok(axum::extract::ws::Message::Text(text)) => {
                let reply = format!("Echo: {}", text);
                let _ = socket.send(axum::extract::ws::Message::Text(reply)).await;
            }
            Ok(axum::extract::ws::Message::Close(_)) => break,
            _ => (),
        }
    }
}
```

---

## 6. Error Handling

Define a custom error type that implements `IntoResponse`. Use it in handlers:

```rust
use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;

#[derive(Debug)]
pub enum AppError {
    DbError(sqlx::Error),
    NotFound(String),
    Unauthorized,
    // ...
}

#[derive(Serialize)]
struct ErrorPayload {
    error: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, msg) = match &self {
            AppError::DbError(_)    => (StatusCode::INTERNAL_SERVER_ERROR, "Database error"),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::Unauthorized  => (StatusCode::UNAUTHORIZED, "Unauthorized"),
        };
        let body = Json(ErrorPayload { error: msg.to_string() });
        (status, body).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::DbError(err)
    }
}
```

You can also register a global rejection handler:

```rust
pub struct ErrorHandler;

impl ErrorHandler {
    pub async fn handle_rejection(err: Box<dyn std::error::Error + Send + Sync>) 
        -> impl IntoResponse 
    {
        // Map unexpected errors to 500
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Unhandled error: {}", err),
        )
    }
}
```

---

## 7. Observability & Monitoring

From `observability-monitoring-framework.md`, we use `tower_http::trace::TraceLayer`:

```rust
use tower_http::trace::{TraceLayer, DefaultOnResponse, DefaultMakeSpan};

let trace_layer = TraceLayer::new_for_http()
    .make_span_with(DefaultMakeSpan::new().include_headers(true))
    .on_response(DefaultOnResponse::new().include_headers(true));
```

This integrates with `tracing` and your monitoring backends.

---

## 8. Putting It All Together

```rust
#[tokio::main]
async fn main() {
    // Init tracing subscriber, metrics, etc.
    tracing_subscriber::fmt::init();

    // Build shared state
    let state = Arc::new(AppState {
        db: init_db_pool().await,
        // ...
    });

    // Create Axum app
    let app = create_app(state);

    // Run server
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
```

---

# References

- Axum docs: https://docs.rs/axum/0.8
- Tower HTTP: https://docs.rs/tower-http
- security-framework.md
- observability-monitoring-framework.md
