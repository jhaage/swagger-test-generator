use axum::{
    routing::{get, post, put, delete},
    Router,
    extract::{Path, Json, State},
    http::StatusCode,
    response::IntoResponse,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use std::net::SocketAddr;
use tower_http::cors::{CorsLayer, Any};
use std::env;

// Models that match our Swagger specification
#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: i64,
    name: String,
    email: String,
    created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize)]
struct UserCreate {
    name: String,
    email: String,
}

#[derive(Debug, Clone, Deserialize)]
struct UserUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<String>,
}

// In-memory database
type Database = Arc<Mutex<HashMap<i64, User>>>;

// Global counter for ID generation
static NEXT_ID: Lazy<Mutex<i64>> = Lazy::new(|| Mutex::new(1));

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Parse command-line arguments for port
    let args: Vec<String> = env::args().collect();
    let mut port = 3000; // Default port
    
    for i in 1..args.len() {
        if args[i] == "--port" && i + 1 < args.len() {
            if let Ok(p) = args[i + 1].parse::<u16>() {
                port = p;
            }
        }
    }

    // Setup our in-memory database
    let db: Database = Arc::new(Mutex::new(HashMap::new()));

    // Add some sample users
    {
        let mut db_lock = db.lock().unwrap();
        let user1 = User {
            id: 1,
            name: "John Doe".to_string(),
            email: "john.doe@example.com".to_string(),
            created_at: Utc::now(),
            updated_at: None,
        };
        let user2 = User {
            id: 2,
            name: "Jane Smith".to_string(),
            email: "jane.smith@example.com".to_string(),
            created_at: Utc::now(),
            updated_at: None,
        };

        db_lock.insert(user1.id, user1);
        db_lock.insert(user2.id, user2);
        
        // Update next ID
        let mut next_id = NEXT_ID.lock().unwrap();
        *next_id = 3;
    }

    // Configure CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build our application with routes
    let app = Router::new()
        .route("/v1/users", get(get_users).post(create_user))
        .route("/v1/users/:id", 
            get(get_user_by_id)
                .put(update_user)
                .delete(delete_user)
        )
        .with_state(db)
        .layer(cors);

    // Run our server
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!("Starting server at {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

// Handler implementations
async fn get_users(
    State(db): State<Database>,
) -> Json<Vec<User>> {
    let db_lock = db.lock().unwrap();
    let users: Vec<User> = db_lock.values().cloned().collect();
    Json(users)
}

async fn create_user(
    State(db): State<Database>,
    Json(user_create): Json<UserCreate>,
) -> impl IntoResponse {
    let mut db_lock = db.lock().unwrap();
    
    // Generate new ID
    let mut next_id_lock = NEXT_ID.lock().unwrap();
    let id = *next_id_lock;
    *next_id_lock += 1;
    
    let user = User {
        id,
        name: user_create.name,
        email: user_create.email,
        created_at: Utc::now(),
        updated_at: None,
    };
    
    db_lock.insert(id, user.clone());
    
    (StatusCode::CREATED, Json(user))
}

async fn get_user_by_id(
    State(db): State<Database>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let db_lock = db.lock().unwrap();
    
    match db_lock.get(&id) {
        Some(user) => (StatusCode::OK, Json(user.clone())),
        None => (StatusCode::NOT_FOUND, Json(User {
            id: 0,
            name: String::new(),
            email: String::new(),
            created_at: Utc::now(),
            updated_at: None,
        })),
    }
}

async fn update_user(
    State(db): State<Database>,
    Path(id): Path<i64>,
    Json(user_update): Json<UserUpdate>,
) -> impl IntoResponse {
    let mut db_lock = db.lock().unwrap();
    
    if let Some(user) = db_lock.get_mut(&id) {
        if let Some(name) = user_update.name {
            user.name = name;
        }
        
        if let Some(email) = user_update.email {
            user.email = email;
        }
        
        user.updated_at = Some(Utc::now());
        
        (StatusCode::OK, Json(user.clone()))
    } else {
        (StatusCode::NOT_FOUND, Json(User {
            id: 0,
            name: String::new(),
            email: String::new(),
            created_at: Utc::now(),
            updated_at: None,
        }))
    }
}

async fn delete_user(
    State(db): State<Database>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let mut db_lock = db.lock().unwrap();
    
    if db_lock.remove(&id).is_some() {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}