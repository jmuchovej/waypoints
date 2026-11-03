//! A trap keeps its message. The target has no standard error, so a panic's
//! text would be lost; the hook stores it, and `wp_panic` hands it back.

use std::sync::{Mutex, Once};

static LAST: Mutex<Option<String>> = Mutex::new(None);
static HOOK: Once = Once::new();

/// Install the hook once. Every export calls this first; the `Once` makes
/// the repetition free.
pub fn install() {
    HOOK.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let text = info.to_string();
            if let Ok(mut last) = LAST.lock() {
                *last = Some(text);
            }
            previous(info);
        }));
    });
}

/// The last panic's message and location, if any panic has occurred.
pub fn last() -> Option<String> {
    LAST.lock().ok().and_then(|last| last.clone())
}

/// Forget the last panic. Tests use it; a plugin never needs to.
pub fn clear() {
    if let Ok(mut last) = LAST.lock() {
        *last = None;
    }
}
