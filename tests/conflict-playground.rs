// Conflict playground - test file for weavr development
use std::fmt::Display;

fn greet(name: &str) -> String {
    format!("Hi there, {}!", name)
}

fn calculate(a: i32, b: i32) -> i32 {
    a + b + 1
}

fn get_version() -> &'static str {
    "2.0.0-alpha"
}
