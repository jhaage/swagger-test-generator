use std::path::Path;
use std::fs::File;
use std::io::Write;
use serde_json::{Value, json};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SwaggerGeneratorError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}

type Result<T> = std::result::Result<T, SwaggerGeneratorError>;

/// Generate a new Swagger document with sample API endpoints for testing purposes
pub fn generate_swagger_document(output_path: &Path) -> Result<()> {
    let swagger_doc = create_sample_swagger_doc();
    
    let mut file = File::create(output_path)?;
    let json_str = serde_json::to_string_pretty(&swagger_doc)?;
    file.write_all(json_str.as_bytes())?;
    
    Ok(())
}

/// Create a sample Swagger document with CRUD operations for users
fn create_sample_swagger_doc() -> Value {
    json!({
        "swagger": "2.0",
        "info": {
            "title": "Sample API",
            "description": "A sample API for testing the Swagger test generator",
            "version": "1.0.0"
        },
        "host": "localhost:3000",
        "basePath": "/v1",
        "schemes": ["http"],
        "paths": {
            "/users": {
                "get": {
                    "summary": "Get all users",
                    "description": "Returns a list of all users",
                    "operationId": "getUsers",
                    "produces": ["application/json"],
                    "responses": {
                        "200": {
                            "description": "A list of users",
                            "schema": {
                                "type": "array",
                                "items": {
                                    "$ref": "#/definitions/User"
                                }
                            }
                        },
                        "500": {
                            "description": "Internal server error"
                        }
                    }
                },
                "post": {
                    "summary": "Create a new user",
                    "description": "Creates a new user with the provided data",
                    "operationId": "createUser",
                    "consumes": ["application/json"],
                    "produces": ["application/json"],
                    "parameters": [
                        {
                            "name": "user",
                            "in": "body",
                            "required": true,
                            "schema": {
                                "$ref": "#/definitions/UserCreate"
                            }
                        }
                    ],
                    "responses": {
                        "201": {
                            "description": "User created successfully",
                            "schema": {
                                "$ref": "#/definitions/User"
                            }
                        },
                        "400": {
                            "description": "Invalid input"
                        },
                        "500": {
                            "description": "Internal server error"
                        }
                    }
                }
            },
            "/users/{id}": {
                "get": {
                    "summary": "Get user by ID",
                    "description": "Returns a single user by ID",
                    "operationId": "getUserById",
                    "produces": ["application/json"],
                    "parameters": [
                        {
                            "name": "id",
                            "in": "path",
                            "required": true,
                            "type": "integer",
                            "format": "int64",
                            "description": "ID of the user to retrieve"
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Successful operation",
                            "schema": {
                                "$ref": "#/definitions/User"
                            }
                        },
                        "404": {
                            "description": "User not found"
                        },
                        "500": {
                            "description": "Internal server error"
                        }
                    }
                },
                "put": {
                    "summary": "Update an existing user",
                    "description": "Updates a user with the provided data",
                    "operationId": "updateUser",
                    "consumes": ["application/json"],
                    "produces": ["application/json"],
                    "parameters": [
                        {
                            "name": "id",
                            "in": "path",
                            "required": true,
                            "type": "integer",
                            "format": "int64",
                            "description": "ID of the user to update"
                        },
                        {
                            "name": "user",
                            "in": "body",
                            "required": true,
                            "schema": {
                                "$ref": "#/definitions/UserUpdate"
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "User updated successfully",
                            "schema": {
                                "$ref": "#/definitions/User"
                            }
                        },
                        "400": {
                            "description": "Invalid input"
                        },
                        "404": {
                            "description": "User not found"
                        },
                        "500": {
                            "description": "Internal server error"
                        }
                    }
                },
                "delete": {
                    "summary": "Delete a user",
                    "description": "Deletes a user by ID",
                    "operationId": "deleteUser",
                    "parameters": [
                        {
                            "name": "id",
                            "in": "path",
                            "required": true,
                            "type": "integer",
                            "format": "int64",
                            "description": "ID of the user to delete"
                        }
                    ],
                    "responses": {
                        "204": {
                            "description": "User deleted successfully"
                        },
                        "404": {
                            "description": "User not found"
                        },
                        "500": {
                            "description": "Internal server error"
                        }
                    }
                }
            }
        },
        "definitions": {
            "User": {
                "type": "object",
                "properties": {
                    "id": {
                        "type": "integer",
                        "format": "int64"
                    },
                    "name": {
                        "type": "string"
                    },
                    "email": {
                        "type": "string",
                        "format": "email"
                    },
                    "created_at": {
                        "type": "string",
                        "format": "date-time"
                    },
                    "updated_at": {
                        "type": "string",
                        "format": "date-time"
                    }
                },
                "required": ["id", "name", "email", "created_at"]
            },
            "UserCreate": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string"
                    },
                    "email": {
                        "type": "string",
                        "format": "email"
                    }
                },
                "required": ["name", "email"]
            },
            "UserUpdate": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string"
                    },
                    "email": {
                        "type": "string",
                        "format": "email"
                    }
                }
            }
        }
    })
}

/// Update an existing Swagger spec with custom modifications
pub fn update_swagger_spec(mut spec: Value) -> Result<Value> {
    // Example modifications that could be made to an existing spec
    
    // Add security definitions if they don't exist
    if !spec.get("securityDefinitions").is_some() {
        let security_defs = json!({
            "api_key": {
                "type": "apiKey",
                "name": "api_key",
                "in": "header"
            },
            "oauth2": {
                "type": "oauth2",
                "flow": "implicit",
                "authorizationUrl": "https://example.com/oauth/authorize",
                "scopes": {
                    "read": "Read access",
                    "write": "Write access"
                }
            }
        });
        
        if let Some(spec_obj) = spec.as_object_mut() {
            spec_obj.insert("securityDefinitions".to_string(), security_defs);
        }
    }
    
    Ok(spec)
}