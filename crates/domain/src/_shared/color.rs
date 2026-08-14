use serde::{Deserialize, Serialize};
use specta::Type;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum Color {
    #[default]
    Default,
    Gray,
    Brown,
    Orange,
    Yellow,
    Green,
    Blue,
    Purple,
    Pink,
    Red,
}

impl Color {
    /// All available colors for sampling
    const ALL_COLORS: &'static [Color] = &[
        Color::Gray,
        Color::Brown,
        Color::Orange,
        Color::Yellow,
        Color::Green,
        Color::Blue,
        Color::Purple,
        Color::Pink,
        Color::Red,
    ];

    /// Sample a color based on a string (deterministic)
    pub fn sample_from_string(input: &str) -> Self {
        if input.is_empty() {
            return Color::Default;
        }

        let mut hasher = DefaultHasher::new();
        input.hash(&mut hasher);
        let hash = hasher.finish();

        let index = (hash as usize) % Self::ALL_COLORS.len();
        Self::ALL_COLORS[index].clone()
    }

    /// Get a random color (deterministic based on seed)
    pub fn sample_with_seed(seed: u64) -> Self {
        let index = (seed as usize) % Self::ALL_COLORS.len();
        Self::ALL_COLORS[index].clone()
    }

    /// Convert to hex color string
    pub fn to_hex(&self) -> &'static str {
        match self {
            Color::Default => "#6b7280", // gray-500
            Color::Gray => "#9ca3af",    // gray-400
            Color::Brown => "#a3a3a3",   // neutral-400
            Color::Orange => "#fb923c",  // orange-400
            Color::Yellow => "#fbbf24",  // yellow-400
            Color::Green => "#4ade80",   // green-400
            Color::Blue => "#60a5fa",    // blue-400
            Color::Purple => "#a78bfa",  // violet-400
            Color::Pink => "#f472b6",    // pink-400
            Color::Red => "#f87171",     // red-400
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_sampling_deterministic() {
        // Same input should always produce same color
        let color1 = Color::sample_from_string("test");
        let color2 = Color::sample_from_string("test");
        assert_eq!(color1, color2);

        // Different inputs should usually produce different colors
        let color3 = Color::sample_from_string("different");
        // Note: With hash-based sampling, there's a small chance of collision
        assert_ne!(color1, color3);
    }

    #[test]
    fn test_color_sampling_coverage() {
        // Test that sampling can produce various colors
        let test_strings = vec![
            "red", "green", "blue", "yellow", "purple", "orange", "pink", "brown", "gray", "black",
        ];

        let mut colors = std::collections::HashSet::new();
        for s in test_strings {
            colors.insert(Color::sample_from_string(s));
        }

        // Should have at least a few different colors
        assert!(colors.len() >= 3, "Should sample diverse colors");
    }

    #[test]
    fn test_empty_string_handling() {
        assert_eq!(Color::sample_from_string(""), Color::Default);
    }

    #[test]
    fn test_hex_color_conversion() {
        // All colors should have valid hex values
        for color in Color::ALL_COLORS {
            let hex = color.to_hex();
            assert!(hex.starts_with('#'));
            assert_eq!(hex.len(), 7); // #RRGGBB format
        }
    }

    #[test]
    fn test_color_seed_sampling() {
        // Same seed should produce same color
        let color1 = Color::sample_with_seed(42);
        let color2 = Color::sample_with_seed(42);
        assert_eq!(color1, color2);

        // Different seeds should usually produce different colors
        let color3 = Color::sample_with_seed(100);
        assert_ne!(color1, color3);
    }
}
