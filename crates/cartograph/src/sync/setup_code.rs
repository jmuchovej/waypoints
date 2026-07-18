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
        assert_eq!(classify("https://waypoints.so/pair#%%%"), SetupCode::Unknown);
    }

    #[test]
    fn renders_a_qr_svg() {
        let svg = qr_svg("https://waypoints.so/pair#abc").unwrap();
        assert!(svg.starts_with("<svg"));
    }
}
