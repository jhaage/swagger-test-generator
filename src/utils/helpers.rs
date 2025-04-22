// This file contains helper functions that assist with various tasks, such as formatting output or handling errors.

use std::path::{Path, PathBuf};
use std::io::{self, Write};
use std::fs::{self, File};

/// Creates a directory if it doesn't exist
pub fn ensure_directory_exists<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let path = path.as_ref();
    if !path.exists() {
        fs::create_dir_all(path)?;
    }
    Ok(())
}

/// Gets a relative path between two absolute paths
pub fn get_relative_path<P: AsRef<Path>, B: AsRef<Path>>(path: P, base: B) -> PathBuf {
    let path = path.as_ref();
    let base = base.as_ref();
    
    pathdiff::diff_paths(path, base).unwrap_or_else(|| path.to_path_buf())
}

/// Sanitizes a path segment for use in filenames
pub fn sanitize_path_for_filename(path: &str) -> String {
    path.replace('/', "_")
        .replace('\\', "_")
        .replace('{', "")
        .replace('}', "")
        .replace(':', "")
        .trim_matches('_')
        .to_string()
}

/// Writes content to a file, creating parent directories if needed
pub fn write_to_file<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, content: C) -> io::Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        ensure_directory_exists(parent)?;
    }
    
    let mut file = File::create(path)?;
    file.write_all(content.as_ref())?;
    Ok(())
}

/// Converts CamelCase to snake_case
pub fn camel_to_snake(camel: &str) -> String {
    let mut snake = String::new();
    let mut chars = camel.chars().peekable();
    
    while let Some(c) = chars.next() {
        if c.is_uppercase() {
            if !snake.is_empty() && snake.chars().last().unwrap() != '_' {
                snake.push('_');
            }
            snake.push(c.to_lowercase().next().unwrap());
        } else {
            snake.push(c);
        }
    }
    
    snake
}

/// Converts snake_case to CamelCase
pub fn snake_to_camel(snake: &str) -> String {
    let mut camel = String::new();
    let mut capitalize_next = true;
    
    for c in snake.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            camel.push(c.to_uppercase().next().unwrap());
            capitalize_next = false;
        } else {
            camel.push(c);
        }
    }
    
    camel
}