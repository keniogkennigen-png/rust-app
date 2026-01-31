use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::{Filter, Rejection, Reply};
use crate::ws_handlers::{AppState, UserSession};

mod ws_handlers;

fn with_app_state(
    app_state: Arc<AppState>,
) -> impl Filter<Extract = (Arc<AppState>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || app_state.clone())
}

fn with_authenticated_session(
    app_state: Arc<AppState>,
) -> impl Filter<Extract = (UserSession,), Error = Rejection> + Clone {
    warp::header::header::<String>("x-session-key")
        .and(with_app_state(app_state))
        .and_then(|session_key: String, app_state_auth: Arc<AppState>| async move {
            let sessions = app_state_auth.user_sessions.lock().await;
            match sessions.get(&session_key) {
                Some(session) => Ok(session.clone()),
                None => Err(warp::reject::custom(ws_handlers::AuthError)),
            }
        })
}

#[tokio::main]
use warp::http::StatusCode;
use std::convert::Infallible;
// Add this to your imports at the top
// use crate::ws_handlers::ErrorResponse; 

async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    let code;
    let message;

    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        message = "Route not found";
    } else if let Some(e) = err.find::<ws_handlers::ErrorResponse>() {
        code = StatusCode::BAD_REQUEST;
        message = &e.message;
    } else if let Some(_) = err.find::<ws_handlers::AuthError>() {
        code = StatusCode::UNAUTHORIZED;
        message = "Authentication failed";
    } else {
        eprintln!("unhandled rejection: {:?}", err);
        code = StatusCode::INTERNAL_SERVER_ERROR;
        message = "Internal Server Error";
    }

    let json = warp::reply::json(&ws_handlers::ErrorResponse {
        message: message.into(),
    });

    Ok(warp::reply::with_status(json, code))
}
async fn main() {
    let app_state = Arc::new(AppState {
        users: Mutex::new(HashMap::new()),
        user_sessions: Mutex::new(HashMap::new()),
        active_connections: Mutex::new(HashMap::new()),
    });

    let static_files = warp::fs::dir("static")
        .or(warp::get().and(warp::path::end()).and(warp::fs::file("static/index.html")));

    let chat_route = warp::path("chat")
        .and(warp::ws())
        .and(warp::path::param())
        .and(with_app_state(app_state.clone()))
        .and_then(ws_handlers::chat_handler);

    let register_route = warp::path("register")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_app_state(app_state.clone()))
        .and_then(ws_handlers::register_handler);

    let login_route = warp::path("login")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_app_state(app_state.clone()))
        .and_then(ws_handlers::login_handler);

    let contacts_post_route = warp::path("contacts")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_authenticated_session(app_state.clone()))
        .and(with_app_state(app_state.clone()))
        .and_then(ws_handlers::add_contact_handler);

    let contacts_get_route = warp::path("contacts")
        .and(warp::get())
        .and(with_authenticated_session(app_state.clone()))
        .and(with_app_state(app_state.clone()))
        .and_then(ws_handlers::get_contacts_handler);

    let routes = static_files
        .or(chat_route)
        .or(register_route)
        .or(login_route)
        .or(contacts_post_route)
        .or(contacts_get_route);

    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "3030".to_string())
        .parse()
        .expect("PORT must be a number");

    println!("Starting chat server on 0.0.0.0:{}", port);

    warp::serve(routes)
        .run(([0, 0, 0, 0], port))
        .await;
}
