use boltffi::*;
use demo_bench_macros::benchmark_candidate;

#[export]
#[benchmark_candidate(function, uniffi, wasm_bindgen)]
pub fn echo_bytes(data: Vec<u8>) -> Vec<u8> {
    data
}

#[export]
pub fn bytes_length(data: Vec<u8>) -> u32 {
    data.len() as u32
}

#[export]
pub fn bytes_sum(data: Vec<u8>) -> u32 {
    data.iter().map(|&b| b as u32).sum()
}

#[export]
pub fn make_bytes(len: u32) -> Vec<u8> {
    (0..len).map(|i| (i % 256) as u8).collect()
}

#[export]
pub fn reverse_bytes(data: Vec<u8>) -> Vec<u8> {
    data.into_iter().rev().collect()
}

#[export]
#[benchmark_candidate(function, uniffi, wasm_bindgen)]
pub fn generate_bytes(size: i32) -> Vec<u8> {
    vec![42u8; size.max(0) as usize]
}
