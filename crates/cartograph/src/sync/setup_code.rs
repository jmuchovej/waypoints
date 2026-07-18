//! The setup-code engine: one classifier for everything a paste, scan, or
//! deep link hands the setup flow, plus the pairing URL envelope and QR
//! rendering. Lives here — not in the frontend — so every platform
//! (desktop webview, mobile native scanner, browser dev) runs the same
//! engine and split-brain between a TS and a Rust implementation cannot
//! happen.
//!
//! Classification only routes; it never trusts. Real validation stays with
//! `outpost::vessel::PairingPayload::decode` and the BIP-39 parse.

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::{Deserialize, Serialize};

/// Where a setup code should be routed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", tag = "kind", content = "value")]
pub enum SetupCode {
    /// A pairing payload (JSON string) for `pairing_join`.
    Pairing(String),
    /// A recovery phrase for `identity_import`.
    Phrase(String),
    /// Unrecognized — surface a "not a setup code" error to the user.
    Unknown,
}

/// Valid BIP-39 mnemonic lengths.
const PHRASE_WORD_COUNTS: [usize; 5] = [12, 15, 18, 21, 24];

/// The two URL heads a pairing link may wear: the universal link (opens
/// the app when installed, the waypoints.so landing page when not) and the
/// custom scheme the app registers.
const PAIR_URL_HEAD_WEB: &str = "https://waypoints.so/pair";
const PAIR_URL_HEAD_APP: &str = "waypoints://pair";

pub fn classify(input: &str) -> SetupCode {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return SetupCode::Unknown;
    }

    if trimmed.starts_with('{') {
        return SetupCode::Pairing(trimmed.to_string());
    }

    if let Some(fragment) = pair_url_fragment(trimmed) {
        return match decode_base64_url(fragment) {
            Some(code) => SetupCode::Pairing(code),
            None => SetupCode::Unknown,
        };
    }

    let words: Vec<String> = trimmed
        .split_whitespace()
        .map(|w| w.to_lowercase())
        .collect();
    if PHRASE_WORD_COUNTS.contains(&words.len())
        && words
            .iter()
            .all(|w| w.chars().all(|c| c.is_ascii_lowercase()))
    {
        return SetupCode::Phrase(words.join(" "));
    }

    SetupCode::Unknown
}

/// Wrap a pairing payload (JSON string) into the shareable universal-link
/// form — what the Add-device QR encodes. The payload rides the URL
/// fragment, which never travels in an HTTP request.
pub fn encode_pair_url(payload_json: &str) -> String {
    format!(
        "{PAIR_URL_HEAD_WEB}#{}",
        URL_SAFE_NO_PAD.encode(payload_json.as_bytes())
    )
}

/// Render a QR code as an SVG string (plain square modules — maximum
/// scanner compatibility, including a decade-old crumpled printout).
pub fn qr_svg(content: &str) -> Result<String, super::SyncError> {
    use fast_qr::convert::Builder as _;

    let qr = fast_qr::QRBuilder::new(content)
        .ecl(fast_qr::ECL::M)
        .build()
        .map_err(|e| super::SyncError::Pairing(format!("QR encoding failed: {e:?}")))?;
    Ok(fast_qr::convert::svg::SvgBuilder::default()
        .margin(0)
        .to_str(&qr))
}

/// Render a QR code as a "liquid" SVG: modules that share an edge merge
/// into one coherent shape — convex corners round only where a module has
/// no neighbor on either side of that corner, and empty cells enclosed on
/// two orthogonal sides (plus the diagonal) get a concave fillet so inner
/// notches dissolve too. This is the neighborhood-aware projection over
/// the module matrix that per-module renderers (etiket et al.) cannot
/// express, and the reason QR rendering lives beside the matrix in Rust.
pub fn qr_svg_liquid(content: &str) -> Result<String, super::SyncError> {
    let qr = fast_qr::QRBuilder::new(content)
        .ecl(fast_qr::ECL::M)
        .build()
        .map_err(|e| super::SyncError::Pairing(format!("QR encoding failed: {e:?}")))?;

    let n = qr.size;
    let filled = |x: isize, y: isize| -> bool {
        x >= 0
            && y >= 0
            && (x as usize) < n
            && (y as usize) < n
            && qr[y as usize][x as usize].value()
    };

    // Corner radius in module units. 0.5 = a lone module becomes a circle.
    const R: f32 = 0.5;
    let mut d = String::new();

    for y in 0..n as isize {
        for x in 0..n as isize {
            let (fx, fy) = (x as f32, y as f32);
            if filled(x, y) {
                // Convex: a corner rounds only when BOTH touching neighbors
                // are empty (otherwise the edge continues into a neighbor).
                let nw = if !filled(x, y - 1) && !filled(x - 1, y) {
                    R
                } else {
                    0.0
                };
                let ne = if !filled(x, y - 1) && !filled(x + 1, y) {
                    R
                } else {
                    0.0
                };
                let se = if !filled(x, y + 1) && !filled(x + 1, y) {
                    R
                } else {
                    0.0
                };
                let sw = if !filled(x, y + 1) && !filled(x - 1, y) {
                    R
                } else {
                    0.0
                };

                d.push_str(&format!("M{},{}", fx + nw, fy));
                d.push_str(&format!("H{}", fx + 1.0 - ne));
                if ne > 0.0 {
                    d.push_str(&format!("A{R},{R} 0 0 1 {},{}", fx + 1.0, fy + ne));
                }
                d.push_str(&format!("V{}", fy + 1.0 - se));
                if se > 0.0 {
                    d.push_str(&format!("A{R},{R} 0 0 1 {},{}", fx + 1.0 - se, fy + 1.0));
                }
                d.push_str(&format!("H{}", fx + sw));
                if sw > 0.0 {
                    d.push_str(&format!("A{R},{R} 0 0 1 {},{}", fx, fy + 1.0 - sw));
                }
                d.push_str(&format!("V{}", fy + nw));
                if nw > 0.0 {
                    d.push_str(&format!("A{R},{R} 0 0 1 {},{}", fx + nw, fy));
                }
                d.push('Z');
            } else {
                // Concave fillets: an empty cell corner whose two orthogonal
                // neighbors AND diagonal are filled is an inner notch of one
                // blob — patch it with a quarter-fillet. (Diagonal required:
                // without it the fillet would bridge two blobs that only
                // touch at a point.)
                // (corner point, arc start, arc end) per corner.
                let corners: [((f32, f32), (f32, f32), (f32, f32), bool); 4] = [
                    // NW corner: needs W, N, NW filled.
                    (
                        (fx, fy),
                        (fx + R, fy),
                        (fx, fy + R),
                        filled(x - 1, y) && filled(x, y - 1) && filled(x - 1, y - 1),
                    ),
                    // NE corner: needs E, N, NE filled.
                    (
                        (fx + 1.0, fy),
                        (fx + 1.0, fy + R),
                        (fx + 1.0 - R, fy),
                        filled(x + 1, y) && filled(x, y - 1) && filled(x + 1, y - 1),
                    ),
                    // SE corner: needs E, S, SE filled.
                    (
                        (fx + 1.0, fy + 1.0),
                        (fx + 1.0 - R, fy + 1.0),
                        (fx + 1.0, fy + 1.0 - R),
                        filled(x + 1, y) && filled(x, y + 1) && filled(x + 1, y + 1),
                    ),
                    // SW corner: needs W, S, SW filled.
                    (
                        (fx, fy + 1.0),
                        (fx, fy + 1.0 - R),
                        (fx + R, fy + 1.0),
                        filled(x - 1, y) && filled(x, y + 1) && filled(x - 1, y + 1),
                    ),
                ];
                for ((cx, cy), (ax, ay), (bx, by), wanted) in corners {
                    if wanted {
                        d.push_str(&format!("M{cx},{cy}L{ax},{ay}A{R},{R} 0 0 0 {bx},{by}Z"));
                    }
                }
            }
        }
    }

    Ok(format!(
        concat!(
            "<svg viewBox=\"0 0 {n} {n}\" xmlns=\"http://www.w3.org/2000/svg\">",
            "<rect width=\"100%\" height=\"100%\" fill=\"#fff\"/>",
            "<path d=\"{d}\" fill=\"#000\"/></svg>"
        ),
        n = n,
        d = d
    ))
}

fn pair_url_fragment(input: &str) -> Option<&str> {
    [PAIR_URL_HEAD_WEB, PAIR_URL_HEAD_APP]
        .iter()
        .find(|head| input.starts_with(*head))
        .and_then(|_| input.split_once('#'))
        .map(|(_, fragment)| fragment)
}

fn decode_base64_url(encoded: &str) -> Option<String> {
    let bytes = URL_SAFE_NO_PAD.decode(encoded.trim_end_matches('=')).ok()?;
    String::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_raw_pairing_json() {
        let code = r#"{"v":1,"kind":"waypoints-pairing","mnemonic":"a b c"}"#;
        assert_eq!(
            classify(&format!("  {code}\n")),
            SetupCode::Pairing(code.to_string())
        );
    }

    #[test]
    fn rejects_empty_and_prose() {
        assert_eq!(classify("   \n "), SetupCode::Unknown);
        assert_eq!(classify("hello there, general kenobi"), SetupCode::Unknown);
    }

    #[test]
    fn recognizes_phrases_normalizing_case_and_whitespace() {
        let phrase = ["abandon"; 24].join(" ");
        let messy = format!("  {}\n", phrase.to_uppercase().replace(' ', "   "));
        assert_eq!(classify(&messy), SetupCode::Phrase(phrase));

        let twelve = "legal winner thank year wave sausage worth useful legal winner thank yellow";
        assert_eq!(classify(twelve), SetupCode::Phrase(twelve.to_string()));
    }

    #[test]
    fn rejects_non_bip39_word_counts_and_tokens() {
        assert_eq!(classify("just five words in here"), SetupCode::Unknown);
        assert_eq!(classify(&["abandon"; 13].join(" ")), SetupCode::Unknown);
        let with_digit = format!("{} n0tw0rd", ["abandon"; 11].join(" "));
        assert_eq!(classify(&with_digit), SetupCode::Unknown);
    }

    #[test]
    fn round_trips_the_pair_url_envelope() {
        let code = r#"{"v":1,"kind":"waypoints-pairing","mnemonic":"ä ö ü"}"#;
        let url = encode_pair_url(code);
        assert!(url.starts_with("https://waypoints.so/pair#"));
        assert!(!url.contains('?'));
        assert_eq!(classify(&url), SetupCode::Pairing(code.to_string()));

        let fragment = url.split_once('#').unwrap().1;
        assert_eq!(
            classify(&format!("waypoints://pair#{fragment}")),
            SetupCode::Pairing(code.to_string())
        );
    }

    #[test]
    fn rejects_pair_urls_with_missing_or_undecodable_fragments() {
        assert_eq!(classify("https://waypoints.so/pair"), SetupCode::Unknown);
        assert_eq!(
            classify("https://waypoints.so/pair#%%%"),
            SetupCode::Unknown
        );
    }

    #[test]
    fn renders_a_qr_svg() {
        let svg = qr_svg("https://waypoints.so/pair#abc").unwrap();
        assert!(svg.starts_with("<svg"));
    }

    /// The liquid rendering must stay a *valid, decodable* QR — proven by
    /// rasterizing the SVG (resvg) and decoding it with a real QR reader
    /// (rqrr), not by asserting on path strings.
    #[test]
    fn liquid_qr_round_trips_through_a_real_decoder() {
        let content = "https://waypoints.so/pair#eyJ2IjoxLCJraW5kIjoid2F5cG9pbnRzLXBhaXJpbmcifQ";
        let svg = qr_svg_liquid(content).unwrap();
        assert!(svg.contains("A0.5,0.5"), "rounding arcs present");

        let options = resvg::usvg::Options::default();
        let tree = resvg::usvg::Tree::from_str(&svg, &options).unwrap();
        let scale = 12.0_f32;
        let side = (tree.size().width() * scale).ceil() as u32;
        let mut pixmap = resvg::tiny_skia::Pixmap::new(side, side).unwrap();
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_scale(scale, scale),
            &mut pixmap.as_mut(),
        );

        let gray = image::GrayImage::from_fn(side, side, |x, y| {
            image::Luma([pixmap.pixel(x, y).map_or(255, |p| p.red())])
        });
        let mut prepared = rqrr::PreparedImage::prepare(gray);
        let grids = prepared.detect_grids();
        assert_eq!(grids.len(), 1, "exactly one QR detected");
        let (_, decoded) = grids[0].decode().unwrap();
        assert_eq!(decoded, content);
    }
}
