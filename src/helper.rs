use serde_json::{Result, Value};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

pub fn print_splash() {
    println!(r"    __  ___________  _____  ________  __   _________    ____  ____ ");
    println!(r"   /  |/  / ____/  |/  / / / / __ \ \/ /  / ____/   |  / __ \/ __ \");
    println!(r"  / /|_/ / __/ / /|_/ / / / / /_/ /\  /  / /   / /| | / /_/ / / / /");
    println!(r" / /  / / /___/ /  / / /_/ / _, _/ / /  / /___/ ___ |/ _, _/ /_/ / ");
    println!(r"/_/  /_/_____/_/  /_/\____/_/ |_| /_/   \____/_/  |_/_/ |_/_____/  ");
}

pub fn parse_json(p: &PathBuf) -> Result<Value> {
    let bytes = std::fs::read_to_string(p).unwrap();
    serde_json::from_str(&bytes)
}

pub fn strip_quotes(s: &str) -> String {
    let s = s.to_string();
    // TODO: this doesn't do what the function says it does
    s.trim_matches('"').to_string()
}

#[allow(dead_code)]
pub fn print_type_of<T>(_: &T) {
    log::info!("{}", std::any::type_name::<T>())
}

pub fn file_sha256(path: &str) -> String {
    // TODO: check file exists
    // TODO: make better hash format
    let bytes = std::fs::read(path).unwrap();
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:02X?}", hasher.finalize())
}
