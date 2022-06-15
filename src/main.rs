//! Simple in-memory key/value store showing features of axum.
//!
//! Run with:
//!
//! ```not_rust
//! cd examples && cargo run -p example-key-value-store
//! ```
//!

#[allow(unused_imports)]
use axum::{
    body::Bytes,
    error_handling::HandleErrorLayer,
    extract::{ContentLengthLimit, Extension, Path},
    handler::Handler,
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Router,
};

use std::{
    borrow::Cow,
    collections::HashMap,
    net::SocketAddr,
    sync::{mpsc::SyncSender, Arc, RwLock},
    time::Duration,
};
use tower::{BoxError, ServiceBuilder};
#[allow(unused_imports)]
use tower_http::{auth::RequireAuthorizationLayer, compression::CompressionLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use std::sync::mpsc::sync_channel;
//use oneshot::{Receiver, Sender};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "example_key_value_store=debug,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Build our application by composing routes
    let app = Router::new()
        .route(
            "/:key",
            post(kv_set),
            // Add compression to `kv_get`
            //get(kv_get.layer(CompressionLayer::new()))
            // But don't compress `kv_set`
        )
        .route("/keys", get(list_keys))
        // Nest our admin routes under `/admin`
        .nest("/admin", admin_routes())
        // Add middleware to all routes
        .layer(
            ServiceBuilder::new()
                // Handle errors from middleware
                .layer(HandleErrorLayer::new(handle_error))
                .load_shed()
                .concurrency_limit(1024)
                .timeout(Duration::from_secs(10))
                .layer(TraceLayer::new_for_http())
                .layer(Extension(SharedState::default()))
                .into_inner(),
        );

    // Run our app with hyper
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr).serve(app.into_make_service()).await.unwrap();
}

type SharedState = Arc<RwLock<State>>;

#[derive(Default)]
struct State {
    db: HashMap<String, StateVal>,
}

//#[derive(Default)]
struct StateVal {
    first_offer: String,
    sync_sender: SyncSender<String>,
    // sender: Mutex<Sender<Bytes>>,
}

// impl Default for StateVal {
//     fn default() -> Self {
//         let (s, r) = oneshot::channel();

//         Self {
//             sender: Mutex::new(s),
//             receiver: Mutex::new(r),
//         }
//         //Self { rx:xrx }
//     }
// }

// async fn kv_get(
//     Path(key): Path<String>,
//     Extension(state): Extension<SharedState>,
// ) -> Result<Bytes, StatusCode> {
//     let db = &state.read().unwrap().db;

//     if let Some(value) = db.get(&key) {
//         Ok(value.clone())
//     } else {
//         Err(StatusCode::NOT_FOUND)
//     }
// }

async fn kv_set(
    Path(key): Path<String>,
    ContentLengthLimit(sdp): ContentLengthLimit<String, { 1024 * 5_000 }>, // ~5mb
    Extension(state): Extension<SharedState>,
) -> Result<String, StatusCode> {
    if let Some(x) = state.write().unwrap().db.remove(&key) {
        //state.write().unwrap().db.remove(&key);

        // let b_sdp = b.localDescription.sdp.replace("a=setup:actpass", "a=setup:active")
        // let a_sdp = a.localDescription.sdp.replace(  "a=setup:actpass",  "a=setup:passive")

        let a_sdp = x.first_offer.clone().replace("a=setup:actpass", "a=setup:active");
        let b_sdp = sdp.replace("a=setup:actpass", "a=setup:passive");

        match x.sync_sender.try_send(b_sdp) {
            Ok(()) => (),
            Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
        }

        return Ok(a_sdp);
    }

    let (sync_sender, receiver) = sync_channel(1);

    //let x: StateVal = Default::default();
    let x: StateVal = StateVal {
        first_offer: sdp,
        sync_sender: sync_sender,
    };

    state.write().unwrap().db.insert(key, x);

    let msg;
    msg = receiver.recv().unwrap();
    println!("message {msg} received");
    return Ok(msg);
}

async fn list_keys(Extension(state): Extension<SharedState>) -> String {
    let db = &state.read().unwrap().db;

    db.keys().map(|key| key.to_string()).collect::<Vec<String>>().join("\n")
}

fn admin_routes() -> Router {
    async fn delete_all_keys(Extension(state): Extension<SharedState>) {
        state.write().unwrap().db.clear();
    }

    async fn remove_key(Path(key): Path<String>, Extension(state): Extension<SharedState>) {
        state.write().unwrap().db.remove(&key);
    }

    Router::new()
        .route("/keys", delete(delete_all_keys))
        .route("/key/:key", delete(remove_key))
        // Require bearer auth for all admin routes
        .layer(RequireAuthorizationLayer::bearer("secret-token"))
}

async fn handle_error(error: BoxError) -> impl IntoResponse {
    if error.is::<tower::timeout::error::Elapsed>() {
        return (StatusCode::REQUEST_TIMEOUT, Cow::from("request timed out"));
    }

    if error.is::<tower::load_shed::error::Overloaded>() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Cow::from("service is overloaded, try again later"),
        );
    }

    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Cow::from(format!("Unhandled internal error: {}", error)),
    )
}
