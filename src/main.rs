// src/main.rs

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::{
    http::StatusCode,
    ws,
    Filter, Rejection, Reply,
};
use warp::reply::{with_status, json};

// Import AppState, ErrorResponse, and UserSession from the ws_handlers module
use crate::ws_handlers::{AppState, ErrorResponse, UserSession};

mod ws_handlers; // Declare your WebSocket handlers module


// A filter that provides the `AppState` to handlers.
fn with_app_state(
    app_state: Arc<AppState>,
) -> impl Filter<Extract = (Arc<AppState>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || app_state.clone())
}

// A combined filter to extract the session key and authenticate the user.
// This filter is specifically designed for HTTP requests where the session key is in a header.
fn with_authenticated_session(
    app_state: Arc<AppState>,
) -> impl Filter<Extract = (UserSession,), Error = Rejection> + Clone {
    warp::header::header::<String>("x-session-key")
        .and(with_app_state(app_state))
        .and_then(|session_key: String, app_state_auth: Arc<AppState>| async move {
            let sessions = app_state_auth.user_sessions.lock().await;
            match sessions.get(&session_key) {
                Some(session) => Ok(session.clone()),
                None => Err(warp::reject::custom(ErrorResponse {
                    message: "Unauthorized: Invalid session key.".to_string(),
                })),
            }
        })
}

// Custom rejection handler to convert `ErrorResponse` rejections into HTTP responses.
async fn handle_rejection(err: Rejection) -> Result<impl Reply, Rejection> {
    if err.is_not_found() {
        eprintln!("Rejection: Not Found - {:?}", err);
        Ok(with_status(json(&ErrorResponse { message: "Not Found".to_string() }), StatusCode::NOT_FOUND))
    } else if let Some(e) = err.find::<ErrorResponse>() {
        eprintln!("Rejection: Custom ErrorResponse - Message: {:?}", e.message);
        Ok(with_status(json(e), StatusCode::BAD_REQUEST))
    }
    // Handle the built-in `warp::reject::MethodNotAllowed` specifically
    else if let Some(_) = err.find::<warp::reject::MethodNotAllowed>() {
        eprintln!("Rejection: Method Not Allowed - {:?}", err);
        Ok(with_status(json(&ErrorResponse { message: "Method Not Allowed".to_string() }), StatusCode::METHOD_NOT_ALLOWED))
    }
    // Re-reject other unhandled Rejection types so Warp can handle them
    // This prevents a blanket 500 and allows Warp to propagate more serious internal errors.
    else {
        eprintln!("Rejection: Unhandled type of rejection, propagating - {:?}", err);
        Err(err) // Re-reject the error
    }
}


#[tokio::main]
async fn main() {
    let app_state = Arc::new(AppState {
        users: Mutex::new(HashMap::new()),
        user_sessions: Mutex::new(HashMap::new()),
        active_connections: Mutex::new(HashMap::new()),
    });

    // --- ROUTES (Keep your existing route definitions here) ---
    // ... (Your login_route, register_route, etc.)

    // --- DYNAMIC PORT LOGIC ---
    let port_key = "PORT";
    let port: u16 = std::env::var(port_key)
        .unwrap_or_else(|_| "3030".to_string())
        .parse()
        .expect("PORT must be a number");

    println!("Starting chat server on 0.0.0.0:{}", port);
    
    warp::serve(routes)
        .run(([0, 0, 0, 0], port))
        .await;
}

    // Registration route
    let register_route = warp::path("register")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_app_state(app_state.clone()))
        .and_then(ws_handlers::register_handler);

    // Login route
    let login_route = warp::path("login")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_app_state(app_state.clone()))
        .and_then(ws_handlers::login_handler);

    // Add contact route
    let contacts_post_route = warp::path("contacts")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_authenticated_session(app_state.clone())) // This filter expects header "x-session-key"
        .and(with_app_state(app_state.clone()))
        .and_then(ws_handlers::add_contact_handler);

    // Get contacts route
    let contacts_get_route = warp::path("contacts")
        .and(warp::get())
        .and(with_authenticated_session(app_state.clone())) // This filter expects header "x-session-key"
        .and(with_app_state(app_state.clone()))
        .and_then(ws_handlers::get_contacts_handler);

    // The order of routes matters. Static files should generally be checked first.
    let routes = static_files // This will now serve 'static/index.html' for '/'
        .or(chat_route)
        .or(register_route)
        .or(login_route)
        .or(contacts_post_route)
        .or(contacts_get_route)
        .with(warp::log("rust_chat"))
        .recover(handle_rejection);

    // Use 0.0.0.0 to be accessible externally
    warp::serve(routes).run(([0, 0, 0, 0], 3030)).await;
}
