//! wire-encoded return payloads (typed errors, callback buffer returns) allocated for the host.
//! uses the global allocator so hosts must free via [`boltffi_free_return_buffer`] — not libc `free`.

use core::alloc::Layout;

/// allocates `len` bytes at align 1 for opaque wire bytes; returns null on len 0 or layout error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn boltffi_alloc_return_buffer(len: usize) -> *mut u8 {
    if len == 0 {
        return core::ptr::null_mut();
    }
    let Ok(layout) = Layout::from_size_align(len, 1) else {
        return core::ptr::null_mut();
    };
    unsafe { std::alloc::alloc(layout) }
}

/// frees a buffer allocated by [`boltffi_alloc_return_buffer`]; ptr null or len 0 is a no-op.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn boltffi_free_return_buffer(ptr: *mut u8, len: usize) {
    if ptr.is_null() || len == 0 {
        return;
    }
    let Ok(layout) = Layout::from_size_align(len, 1) else {
        return;
    };
    unsafe { std::alloc::dealloc(ptr, layout) };
}

#[cfg(test)]
mod tests {
    use super::{boltffi_alloc_return_buffer, boltffi_free_return_buffer};

    #[test]
    fn alloc_zero_returns_null() {
        assert!(unsafe { boltffi_alloc_return_buffer(0) }.is_null());
    }

    #[test]
    fn alloc_nonzero_roundtrip_write_free() {
        let len = 16usize;
        let ptr = unsafe { boltffi_alloc_return_buffer(len) };
        assert!(!ptr.is_null());
        unsafe {
            core::ptr::write_bytes(ptr, 0xAB, len);
            assert_eq!(*ptr, 0xAB);
            assert_eq!(*ptr.add(len - 1), 0xAB);
            boltffi_free_return_buffer(ptr, len);
        }
    }

    #[test]
    fn free_null_and_zero_len_noop() {
        unsafe {
            boltffi_free_return_buffer(core::ptr::null_mut(), 8);
            let p = boltffi_alloc_return_buffer(4);
            boltffi_free_return_buffer(p, 0);
            boltffi_free_return_buffer(p, 4);
        }
    }
}
