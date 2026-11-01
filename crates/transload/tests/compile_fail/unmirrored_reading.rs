//! The exhaustiveness guard: a mirror that misses a data-model variant is a
//! compile error, never a silent gap.
use domain::Reading;
use transload::fragment::ReadingFragment;

fn mirror(reading: &Reading) -> ReadingFragment {
    match reading {
        Reading::Null => ReadingFragment::Null,
        Reading::Number(n) => ReadingFragment::Number(*n),
    }
}

fn main() {
    let _ = mirror(&Reading::Null);
}
