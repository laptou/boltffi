use boltffi::*;
use demo_bench_macros::benchmark_candidate;

#[export]
#[benchmark_candidate(function, uniffi, wasm_bindgen)]
pub fn echo_string(v: String) -> String {
    v
}

/// Concatenates two strings and returns the combined result.
#[export]
pub fn concat_strings(a: String, b: String) -> String {
    format!("{}{}", a, b)
}

#[export]
pub fn string_length(v: String) -> u32 {
    v.len() as u32
}

#[export]
pub fn string_is_empty(v: String) -> bool {
    v.is_empty()
}

#[export]
pub fn repeat_string(v: String, count: u32) -> String {
    v.repeat(count as usize)
}

#[export]
#[benchmark_candidate(function, uniffi, wasm_bindgen)]
pub fn generate_string(size: i32) -> String {
    "x".repeat(size.max(0) as usize)
}
