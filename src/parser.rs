// This file contains the implementation of the parser module, responsible for reading and interpreting the Swagger document.

pub mod swagger;

pub use swagger::{
    parse_swagger_file,
    SwaggerSpec,
    ApiPath,
    ApiOperation,
    ApiParameter,
    ApiResponse,
    ParserError,
    Result,
};

use std::fs::File;
use std::io::{self, Read};
use serde_json::Value;

pub fn read_swagger_file(file_path: &str) -> io::Result<Value> {
    let mut file = File::open(file_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let swagger_doc: Value = serde_json::from_str(&contents)?;
    Ok(swagger_doc)
}