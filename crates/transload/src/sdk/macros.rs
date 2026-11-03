/// Make a type that implements [`Importer`](crate::sdk::Importer) a plugin.
///
/// ```ignore
/// transload::importer!(PlainMarkdown, manifest = "../importer.toml");
/// ```
///
/// Supplies the authored manifest (the TOML at `manifest`, relative to the
/// invoking file, embedded at compile time) and the plugin crate's version,
/// and — when the target is wasm32 — the eight ABI exports: thin wrappers over
/// a [`Runtime`](crate::sdk::Runtime) that cross the buffer boundary and
/// install the panic hook. On a host target only the manifest is supplied;
/// the exports' `u32` pointers are wasm32's, and a plugin's host-side tests
/// drive the `Runtime` directly.
#[macro_export]
macro_rules! importer {
    ($ty:ty, manifest = $path:literal) => {
        impl $crate::sdk::Manifested for $ty {
            const AUTHORED: &'static str = include_str!($path);
            const VERSION: &'static str = env!("CARGO_PKG_VERSION");
        }

        #[cfg(target_arch = "wasm32")]
        ::std::thread_local! {
            static __WP_RUNTIME: ::std::cell::RefCell<$crate::sdk::Runtime<$ty>> =
                ::std::cell::RefCell::new($crate::sdk::Runtime::new());
        }

        #[cfg(target_arch = "wasm32")]
        #[unsafe(no_mangle)]
        pub extern "C" fn wp_abi() -> u32 {
            $crate::sdk::panic::install();
            $crate::abi::ABI
        }

        #[cfg(target_arch = "wasm32")]
        #[unsafe(no_mangle)]
        pub extern "C" fn wp_alloc(len: u32) -> u32 {
            $crate::sdk::panic::install();
            $crate::sdk::buffer::alloc(len)
        }

        #[cfg(target_arch = "wasm32")]
        #[unsafe(no_mangle)]
        pub extern "C" fn wp_free(ptr: u32, len: u32) {
            $crate::sdk::panic::install();
            // SAFETY: the host passes back exactly what `wp_alloc` or an output
            // returned; that is the ABI's contract.
            unsafe { $crate::sdk::buffer::free(ptr, len) }
        }

        #[cfg(target_arch = "wasm32")]
        #[unsafe(no_mangle)]
        pub extern "C" fn wp_manifest() -> u64 {
            $crate::sdk::panic::install();
            let bytes = __WP_RUNTIME.with(|runtime| runtime.borrow().manifest());
            $crate::sdk::buffer::pack(bytes)
        }

        #[cfg(target_arch = "wasm32")]
        #[unsafe(no_mangle)]
        pub extern "C" fn wp_init(cfg_ptr: u32, cfg_len: u32) -> u64 {
            $crate::sdk::panic::install();
            // SAFETY: a live host-written buffer from `wp_alloc`, per the ABI.
            let config = unsafe { $crate::sdk::buffer::slice(cfg_ptr, cfg_len) };
            let bytes = __WP_RUNTIME.with(|runtime| runtime.borrow_mut().init(config));
            $crate::sdk::buffer::pack(bytes)
        }

        #[cfg(target_arch = "wasm32")]
        #[unsafe(no_mangle)]
        pub extern "C" fn wp_entry(
            state: u32,
            name_ptr: u32,
            name_len: u32,
            bytes_ptr: u32,
            bytes_len: u32,
        ) -> u64 {
            $crate::sdk::panic::install();
            // SAFETY: live host-written buffers from `wp_alloc`, per the ABI.
            let name = unsafe { $crate::sdk::buffer::slice(name_ptr, name_len) };
            let bytes = unsafe { $crate::sdk::buffer::slice(bytes_ptr, bytes_len) };
            let name = ::std::string::String::from_utf8_lossy(name);
            let out = __WP_RUNTIME.with(|runtime| runtime.borrow_mut().entry(state, &name, bytes));
            $crate::sdk::buffer::pack(out)
        }

        #[cfg(target_arch = "wasm32")]
        #[unsafe(no_mangle)]
        pub extern "C" fn wp_finalize(state: u32) -> u64 {
            $crate::sdk::panic::install();
            let bytes = __WP_RUNTIME.with(|runtime| runtime.borrow_mut().finalize(state));
            $crate::sdk::buffer::pack(bytes)
        }

        #[cfg(target_arch = "wasm32")]
        #[unsafe(no_mangle)]
        pub extern "C" fn wp_panic() -> u64 {
            match $crate::sdk::panic::last() {
                Some(text) => $crate::sdk::buffer::pack(text.into_bytes()),
                None => 0,
            }
        }
    };
}
