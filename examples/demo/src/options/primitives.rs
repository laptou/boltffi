use boltffi::*;
use demo_bench_macros::benchmark_candidate;

#[export]
#[benchmark_candidate(function, uniffi, wasm_bindgen)]
pub fn echo_optional_i32(v: Option<i32>) -> Option<i32> {
    v
}

#[export]
pub fn echo_optional_f64(v: Option<f64>) -> Option<f64> {
    v
}

#[export]
pub fn echo_optional_bool(v: Option<bool>) -> Option<bool> {
    v
}

#[export]
pub fn unwrap_or_default_i32(v: Option<i32>, fallback: i32) -> i32 {
    v.unwrap_or(fallback)
}

#[export]
pub fn make_some_i32(v: i32) -> Option<i32> {
    Some(v)
}

#[export]
pub fn make_none_i32() -> Option<i32> {
    None
}

#[export]
pub fn double_if_some(v: Option<i32>) -> Option<i32> {
    v.map(|x| x * 2)
}

#[export]
#[benchmark_candidate(function, uniffi, wasm_bindgen)]
pub fn find_even(value: i32) -> Option<i32> {
    if value % 2 == 0 { Some(value) } else { None }
}

#[export]
#[benchmark_candidate(function, uniffi)]
pub fn find_positive_i64(value: i64) -> Option<i64> {
    if value > 0 { Some(value) } else { None }
}

#[export]
#[benchmark_candidate(function, uniffi, wasm_bindgen)]
pub fn find_positive_f64(value: f64) -> Option<f64> {
    if value > 0.0 { Some(value) } else { None }
}
