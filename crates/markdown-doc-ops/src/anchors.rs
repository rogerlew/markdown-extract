use percent_encoding::percent_decode_str;

/// Normalise anchor fragments by decoding percent-encoding, trimming, and lowering ASCII text.
pub fn normalize_anchor_fragment(fragment: &str) -> String {
    percent_decode_str(fragment)
        .decode_utf8_lossy()
        .trim()
        .to_ascii_lowercase()
}
