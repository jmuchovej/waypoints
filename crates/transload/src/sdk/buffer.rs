//! The buffer convention, guest side: linear memory is the guest's, so the
//! guest allocates every buffer a host writes into and every buffer it
//! returns, and a `u64` return packs `(ptr << 32) | len`.
//!
//! wasm32 only: pointers are 32 bits here by construction, and a host-side
//! test drives [`Runtime`](super::Runtime) over bytes instead.

#![cfg(target_arch = "wasm32")]

/// A buffer the host may write `len` bytes into. Released by [`free`].
pub fn alloc(len: u32) -> u32 {
    let buffer = vec![0u8; len as usize].into_boxed_slice();
    Box::into_raw(buffer) as *mut u8 as u32
}

/// Release a buffer that [`alloc`] or an output returned.
///
/// # Safety
/// `ptr` and `len` must be exactly what [`alloc`] or [`pack`] produced, once.
pub unsafe fn free(ptr: u32, len: u32) {
    let slice = std::ptr::slice_from_raw_parts_mut(ptr as *mut u8, len as usize);
    drop(unsafe { Box::from_raw(slice) });
}

/// Hand bytes to the host: a fresh buffer, packed.
pub fn pack(bytes: Vec<u8>) -> u64 {
    let boxed = bytes.into_boxed_slice();
    let len = boxed.len() as u32;
    let ptr = Box::into_raw(boxed) as *mut u8 as u32;
    (u64::from(ptr) << 32) | u64::from(len)
}

/// A host-written buffer, borrowed.
///
/// # Safety
/// `ptr` and `len` must describe a live buffer from [`alloc`].
pub unsafe fn slice<'a>(ptr: u32, len: u32) -> &'a [u8] {
    unsafe { std::slice::from_raw_parts(ptr as *const u8, len as usize) }
}
