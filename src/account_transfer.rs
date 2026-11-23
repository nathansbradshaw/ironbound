//! Explicit, private account snapshots for moving between browsers. Ordinary
//! public setup links remain an allowlist in share.rs and never use this format.
use crate::setups::Saved;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use flate2::{Compression, GzBuilder, bufread::GzDecoder};
use std::io::{Read, Write};

pub const PREFIX: &str = "#account=v1.";
/// Keep links bounded; a backup file remains the fallback for larger accounts.
pub const MAX_FRAGMENT: usize = 48_000;
pub const MAX_JSON: usize = 4_000_000;

pub fn fragment(saved: &Saved) -> Result<String, String> {
    let mut portable = saved.clone();
    // A cached public URL is not account data and needlessly duplicates settings.
    portable.last_setup_fragment = None;
    let raw = serde_json::to_string(&portable).map_err(|_| "Could not encode your account.")?;
    Saved::parse(&raw)?;
    let mut encoder = GzBuilder::new()
        .mtime(0)
        .write(Vec::new(), Compression::best());
    encoder
        .write_all(raw.as_bytes())
        .map_err(|_| "Could not compress your account.")?;
    let compressed = encoder
        .finish()
        .map_err(|_| "Could not compress your account.")?;
    let link = format!("{PREFIX}{}", URL_SAFE_NO_PAD.encode(compressed));
    if link.len() > MAX_FRAGMENT {
        return Err("This account is too large for a reliable transfer link. Download a private backup and import the file on your other device.".into());
    }
    Ok(link)
}

pub fn parse(fragment: &str) -> Result<Saved, String> {
    let invalid = || {
        "This account transfer link is invalid or incomplete. Copy the whole link, or import a private backup file.".to_string()
    };
    if fragment.len() > MAX_FRAGMENT {
        return Err(invalid());
    }
    let encoded = fragment.strip_prefix(PREFIX).ok_or_else(invalid)?;
    let compressed = URL_SAFE_NO_PAD.decode(encoded).map_err(|_| invalid())?;
    let mut decoder = GzDecoder::new(compressed.as_slice());
    let mut bytes = Vec::new();
    decoder
        .by_ref()
        .take(MAX_JSON as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| invalid())?;
    if bytes.len() > MAX_JSON || !decoder.get_ref().is_empty() {
        return Err(invalid());
    }
    let raw = std::str::from_utf8(&bytes).map_err(|_| invalid())?;
    let mut saved = Saved::parse(raw)?;
    saved.last_setup_fragment = None;
    Ok(saved)
}
