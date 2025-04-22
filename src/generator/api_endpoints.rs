// This file contains the logic for generating custom API endpoints based on the Swagger document.

use std::path::Path;
use std::fs::{self, File};
use std::io::Write;
use crate::parser::{SwaggerSpec, ApiPath, ApiOperation};
use crate::utils::sanitize_path_for_filename;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiGeneratorError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),
}

type Result<T> = std::result::Result<T, ApiGeneratorError>;

/// Generate Rust axum API endpoints from a Swagger specification
pub fn generate_axum_api(spec: &SwaggerSpec, output_dir: &Path) -> Result<()> {
    // Create the output directory if it doesn't exist
    fs::create_dir_all(output_dir)?;
    
    // Generate models module
    generate_models_module(spec, output_dir)?;
    
    // Generate routes module
    generate_routes_module(spec, output_dir)?;
    
    // Generate handlers module with route handlers
    generate_handlers_module(spec, output_dir)?;
    
    // Generate main.rs entrypoint
    generate_main_file(spec, output_dir)?;
    
    // Generate Cargo.toml file
    generate_cargo_toml(output_dir)?;
    
    Ok(())
}

fn generate_models_module(spec: &SwaggerSpec, output_dir: &Path) -> Result<()> {
    let models_dir = output_dir.join("src").join("models");
    fs::create_dir_all(&models_dir)?;
    
    // Create mod.rs for models
    let mod_path = models_dir.join("mod.rs");
    let mut mod_file = File::create(mod_path)?;
    
    // Extract schema definitions from the spec
    let definitions = spec.raw_spec.get("definitions")
        .or_else(|| spec.raw_spec.get("components").and_then(|c| c.get("schemas")));
    
    if let Some(defs) = definitions {
        if let Some(def_obj) = defs.as_object() {
            // Write model structs
            for (name, schema) in def_obj {
                let model_name = name.clone();
                let model_path = models_dir.join(format!("{}.rs", model_name.to_lowercase()));
                let mut model_file = File::create(&model_path)?;
                
                // Generate model struct from schema
                let model_code = generate_model_from_schema(&model_name, schema);
                writeln!(model_file, "{}", model_code)?;
                
                // Add model to mod.rs
                writeln!(mod_file, "pub mod {};", model_name.to_lowercase())?;
                writeln!(mod_file, "pub use {}::{};", model_name.to_lowercase(), model_name)?;
            }
        }
    } else {
        // If no definitions found, create basic User model
        let user_path = models_dir.join("user.rs");
        let mut user_file = File::create(user_path)?;
        
        writeln!(user_file, r#"use serde::{{Deserialize, Serialize}};
use chrono::{{DateTime, Utc}};"#)?;
        
        writeln!(user_file, r#"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {{
    pub id: i64,
    pub name: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}}

#[derive(Debug, Clone, Deserialize)]
pub struct UserCreate {{
    pub name: String,
    pub email: String,
}}

#[derive(Debug, Clone, Deserialize)]
pub struct UserUpdate {{
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}}"#)?;
        
        // Add model to mod.rs
        writeln!(mod_file, "pub mod user;")?;
        writeln!(mod_file, "pub use user::{{User, UserCreate, UserUpdate}};")?;
    }
    
    Ok(())
}

fn generate_model_from_schema(name: &str, schema: &serde_json::Value) -> String {
    // Simple model generator - can be expanded for more complex schemas
    let mut model = format!(r#"use serde::{{Deserialize, Serialize}};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct {} {{
"#, name);
    
    if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
        for (prop_name, prop_schema) in props {
            let field_type = match prop_schema.get("type").and_then(|t| t.as_str()) {
                Some("string") => {
                    if let Some("date-time") = prop_schema.get("format").and_then(|f| f.as_str()) {
                        "DateTime<Utc>".to_string()
                    } else {
                        "String".to_string()
                    }
                },
                Some("integer") => "i64".to_string(),
                Some("number") => "f64".to_string(),
                Some("boolean") => "bool".to_string(),
                Some("array") => {
                    // Simple array handling
                    "Vec<String>".to_string()
                },
                Some("object") => "serde_json::Value".to_string(),
                _ => "String".to_string(),
            };
            
            model.push_str(&format!("    pub {}: {},\n", prop_name, field_type));
        }
    }
    
    model.push_str("}\n");
    
    if model.contains("DateTime<Utc>") {
        model = format!("use chrono::{{DateTime, Utc}};\n{}", model);
    }
    
    model
}

fn generate_routes_module(spec: &SwaggerSpec, output_dir: &Path) -> Result<()> {
    let routes_dir = output_dir.join("src").join("routes");
    fs::create_dir_all(&routes_dir)?;
    
    // Create mod.rs for routes
    let mod_path = routes_dir.join("mod.rs");
    let mut mod_file = File::create(mod_path)?;
    
    writeln!(mod_file, r#"use axum::Router;
use tower_http::cors::CorsLayer;

// Import route modules
"#)?;
    
    // Group routes by top-level path segment
    let mut route_groups = std::collections::HashMap::new();
    
    for path in &spec.paths {
        let first_segment = path.path
            .trim_start_matches('/')
            .split('/')
            .next()
            .unwrap_or("api")
            .to_string();
            
        route_groups.entry(first_segment).or_insert_with(Vec::new).push(path);
    }
    
    // Collect group names for later use
    let group_names: Vec<String> = route_groups.keys().cloned().collect();
    
    // Generate each route module
    for (group, paths) in &route_groups {
        let group_name = sanitize_path_for_filename(&group);
        let group_path = routes_dir.join(format!("{}.rs", group_name));
        let mut group_file = File::create(&group_path)?;
        
        // Import handlers
        writeln!(group_file, r#"use axum::{{
    routing::{{get, post, put, delete}},
    Router,
}};
use crate::handlers::{}::*;
"#, group_name)?;
        
        // Define routes
        writeln!(group_file, "pub fn routes() -> Router {{")?;
        writeln!(group_file, "    Router::new()")?;
        
        for path in paths {
            for op in &path.operations {
                let route_path = if path.path.starts_with('/') {
                    path.path.clone()
                } else {
                    format!("/{}", path.path)
                };
                
                let _handler_name = op.operation_id.to_lowercase();
                let method = op.method.to_lowercase();
                
                writeln!(group_file, "        .route(\"{}\", {}({}))", route_path, method, _handler_name)?;
            }
        }
        
        writeln!(group_file, "}}\n")?;
        
        // Add module to mod.rs
        writeln!(mod_file, "pub mod {};", group_name)?;
    }
    
    // Create app router function
    writeln!(mod_file, r#"
pub fn app_router() -> Router {{
    // Configure CORS
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any);
        
    Router::new()"#)?;
    
    for group in group_names {
        let group_name = sanitize_path_for_filename(&group);
        writeln!(mod_file, "        .merge({0}::routes())", group_name)?;
    }
    
    writeln!(mod_file, "        .layer(cors)\n}}")?;
    
    Ok(())
}

fn generate_handlers_module(spec: &SwaggerSpec, output_dir: &Path) -> Result<()> {
    let handlers_dir = output_dir.join("src").join("handlers");
    fs::create_dir_all(&handlers_dir)?;
    
    // Create mod.rs for handlers
    let mod_path = handlers_dir.join("mod.rs");
    let mut mod_file = File::create(mod_path)?;
    
    // Group handlers by top-level path segment
    let mut handler_groups = std::collections::HashMap::new();
    
    for path in &spec.paths {
        let first_segment = path.path
            .trim_start_matches('/')
            .split('/')
            .next()
            .unwrap_or("api")
            .to_string();
            
        handler_groups.entry(first_segment).or_insert_with(Vec::new).push(path);
    }
    
    // Generate each handler module
    for (group, paths) in handler_groups {
        let group_name = sanitize_path_for_filename(&group);
        writeln!(mod_file, "pub mod {};", group_name)?;
        
        let group_path = handlers_dir.join(format!("{}.rs", group_name));
        let mut group_file = File::create(&group_path)?;
        
        // Import dependencies
        writeln!(group_file, r#"use axum::{{
    extract::{{Path, Json, State}},
    http::StatusCode,
    response::IntoResponse,
}};
use serde_json::json;
use crate::models::*;
use std::sync::{{Arc, Mutex}};
use std::collections::HashMap;
"#)?;
        
        // Generate handlers for paths in this group
        for path in paths {
            for op in &path.operations {
                let handler_name = op.operation_id.to_lowercase();
                let method = op.method.to_uppercase();
                
                // Generate handler based on HTTP method and path
                match method.as_str() {
                    "GET" => {
                        if path.path.contains('{') {
                            // Get by ID
                            generate_get_by_id_handler(&mut group_file, op, path)?;
                        } else {
                            // Get all
                            generate_get_all_handler(&mut group_file, op, path)?;
                        }
                    },
                    "POST" => generate_create_handler(&mut group_file, op, path)?,
                    "PUT" => generate_update_handler(&mut group_file, op, path)?,
                    "DELETE" => generate_delete_handler(&mut group_file, op, path)?,
                    _ => {
                        return Err(ApiGeneratorError::UnsupportedOperation(
                            format!("Unsupported HTTP method: {}", method)
                        ));
                    }
                }
            }
        }
    }
    
    Ok(())
}

fn generate_get_all_handler(file: &mut File, op: &ApiOperation, path: &ApiPath) -> Result<()> {
    let resource = path.path
        .trim_start_matches('/')
        .split('/')
        .next()
        .unwrap_or("items");
    
    let model_name = resource.trim_end_matches('s');
    let model_name = model_name.chars().next().unwrap_or('i').to_uppercase()
        .chain(model_name.chars().skip(1))
        .collect::<String>();
    
    writeln!(file, r#"
pub async fn {}() -> impl IntoResponse {{
    // This would typically come from a database
    let items = vec![
        {}::{{
            id: 1,
            name: "Example {}".into(),
            email: "example@example.com".into(),
            created_at: chrono::Utc::now(),
            updated_at: None,
        }},
        {}::{{
            id: 2,
            name: "Another {}".into(),
            email: "another@example.com".into(),
            created_at: chrono::Utc::now(),
            updated_at: None,
        }}
    ];
    
    (StatusCode::OK, Json(items))
}}"#, 
        op.operation_id.to_lowercase(),
        model_name, model_name, 
        model_name, model_name
    )?;
    
    Ok(())
}

fn generate_get_by_id_handler(file: &mut File, op: &ApiOperation, path: &ApiPath) -> Result<()> {
    let resource = path.path
        .trim_start_matches('/')
        .split('/')
        .next()
        .unwrap_or("items");
    
    let model_name = resource.trim_end_matches('s');
    let model_name = model_name.chars().next().unwrap_or('i').to_uppercase()
        .chain(model_name.chars().skip(1))
        .collect::<String>();
    
    let id_param = op.path_params.iter()
        .find(|p| p.name == "id")
        .map(|p| p.param_type.clone())
        .unwrap_or_else(|| "i64".to_string());
    
    writeln!(file, r#"
pub async fn {}(Path(id): Path<{}>) -> impl IntoResponse {{
    // In a real application, we would fetch from a database
    if id == 1 {{
        let item = {}::{{
            id: 1,
            name: "Example {}".into(),
            email: "example@example.com".into(),
            created_at: chrono::Utc::now(),
            updated_at: None,
        }};
        
        (StatusCode::OK, Json(item))
    }} else {{
        (StatusCode::NOT_FOUND, Json(json!({{ 
            "error": "Not found" 
        }})))
    }}
}}"#, 
        op.operation_id.to_lowercase(),
        id_param,
        model_name, model_name
    )?;
    
    Ok(())
}

fn generate_create_handler(file: &mut File, op: &ApiOperation, path: &ApiPath) -> Result<()> {
    let resource = path.path
        .trim_start_matches('/')
        .split('/')
        .next()
        .unwrap_or("items");
    
    let model_name = resource.trim_end_matches('s');
    let model_name = model_name.chars().next().unwrap_or('i').to_uppercase()
        .chain(model_name.chars().skip(1))
        .collect::<String>();
    
    writeln!(file, r#"
pub async fn {}(Json(payload): Json<{}Create>) -> impl IntoResponse {{
    // In a real application, we would insert into a database
    let item = {}::{{
        id: 42, // Would be generated by the database
        name: payload.name,
        email: payload.email,
        created_at: chrono::Utc::now(),
        updated_at: None,
    }};
    
    (StatusCode::CREATED, Json(item))
}}"#, 
        op.operation_id.to_lowercase(),
        model_name, model_name
    )?;
    
    Ok(())
}

fn generate_update_handler(file: &mut File, op: &ApiOperation, path: &ApiPath) -> Result<()> {
    let resource = path.path
        .trim_start_matches('/')
        .split('/')
        .next()
        .unwrap_or("items");
    
    let model_name = resource.trim_end_matches('s');
    let model_name = model_name.chars().next().unwrap_or('i').to_uppercase()
        .chain(model_name.chars().skip(1))
        .collect::<String>();
    
    let id_param = op.path_params.iter()
        .find(|p| p.name == "id")
        .map(|p| p.param_type.clone())
        .unwrap_or_else(|| "i64".to_string());
    
    writeln!(file, r#"
pub async fn {}(
    Path(id): Path<{}>,
    Json(payload): Json<{}Update>
) -> impl IntoResponse {{
    // In a real application, we would update a database record
    if id == 1 {{
        let item = {}::{{
            id: 1,
            name: payload.name.unwrap_or("Example {}".into()),
            email: payload.email.unwrap_or("example@example.com".into()),
            created_at: chrono::Utc::now(),
            updated_at: Some(chrono::Utc::now()),
        }};
        
        (StatusCode::OK, Json(item))
    }} else {{
        (StatusCode::NOT_FOUND, Json(json!({{ 
            "error": "Not found" 
        }})))
    }}
}}"#, 
        op.operation_id.to_lowercase(),
        id_param,
        model_name, 
        model_name, model_name
    )?;
    
    Ok(())
}

fn generate_delete_handler(file: &mut File, op: &ApiOperation, _path: &ApiPath) -> Result<()> {
    let id_param = op.path_params.iter()
        .find(|p| p.name == "id")
        .map(|p| p.param_type.clone())
        .unwrap_or_else(|| "i64".to_string());
    
    writeln!(file, r#"
pub async fn {}(Path(id): Path<{}>) -> impl IntoResponse {{
    // In a real application, we would delete from a database
    if id == 1 {{
        StatusCode::NO_CONTENT
    }} else {{
        StatusCode::NOT_FOUND
    }}
}}"#, 
        op.operation_id.to_lowercase(),
        id_param
    )?;
    
    Ok(())
}

fn generate_main_file(_spec: &SwaggerSpec, output_dir: &Path) -> Result<()> {
    let src_dir = output_dir.join("src");
    fs::create_dir_all(&src_dir)?;
    
    let main_path = src_dir.join("main.rs");
    let mut main_file = File::create(main_path)?;
    
    writeln!(main_file, r#"mod models;
mod handlers;
mod routes;

use routes::app_router;
use std::net::SocketAddr;

#[tokio::main]
async fn main() {{
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    // Build our application
    let app = app_router();

    // Listen on the default port
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("Starting server at {{}}", addr);
    
    // Start the server
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}}"#)?;
    
    Ok(())
}

fn generate_cargo_toml(output_dir: &Path) -> Result<()> {
    let cargo_path = output_dir.join("Cargo.toml");
    let mut cargo_file = File::create(cargo_path)?;
    
    writeln!(cargo_file, r#"[package]
name = "generated_api"
version = "0.1.0"
edition = "2021"

[dependencies]
axum = "0.6.18"
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
tokio = {{ version = "1", features = ["full"] }}
uuid = {{ version = "1.3", features = ["v4", "serde"] }}
chrono = {{ version = "0.4", features = ["serde"] }}
tracing = "0.1"
tracing-subscriber = {{ version = "0.3", features = ["env-filter"] }}
tower-http = {{ version = "0.4", features = ["cors"] }}
once_cell = "1.17"
thiserror = "1.0"
"#)?;
    
    Ok(())
}

/// Generate a Swagger document from a spec for validation purposes
pub fn generate_swagger_doc(spec: &SwaggerSpec, output_dir: &Path) -> Result<()> {
    // For now just copy the parsed spec to avoid complexities
    let swagger_path = output_dir.join("swagger.json");
    let mut swagger_file = File::create(swagger_path)?;
    
    // Write the raw spec back out
    let json_str = serde_json::to_string_pretty(&spec.raw_spec)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        
    swagger_file.write_all(json_str.as_bytes())?;
    
    Ok(())
}