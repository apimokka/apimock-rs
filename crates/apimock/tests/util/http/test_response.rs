use hyper::body::Bytes;
use reqwest::Response;

/// convert response body bytes to string
pub async fn response_body_str(response: Response) -> String {
    let body_bytes = response_body_bytes(response).await;
    String::from_utf8(body_bytes.into()).unwrap()
}

/// Translate the `\n`s in a fixture-file-derived expected string to
/// whatever line ending git's checkout actually produced on this
/// platform.
///
/// # Why this exists (RFC 076)
///
/// A `.json`/text `file_path` response is now served exactly as
/// written, byte for byte (no parse/reserialise round-trip) — so a
/// checked-in fixture's own line endings now reach the response
/// verbatim, and Windows checks out a normal text file with `\r\n`
/// while Linux and macOS check it out with `\n` (no `.gitattributes`
/// pins this repository to one convention, and it should not: `\n`
/// wins on this dev machine, `\r\n` wins in CI's own Windows runner,
/// so pinning either one server-side would make what's actually on
/// disk *not* what a real Windows deployment serves — the opposite of
/// what this RFC promises). `html.rs`'s own `NEW_LINE` constant
/// already handled this for one fixture, ad hoc; this generalises the
/// same idea so every fixture-comparison test doesn't need its own
/// copy.
pub fn platform_eol(expected: &str) -> String {
    if cfg!(windows) {
        expected.replace('\n', "\r\n")
    } else {
        expected.to_owned()
    }
}

/// convert response body bytes to string
pub async fn response_body_bytes(response: Response) -> Bytes {
    response
        .bytes()
        .await
        .expect("failed to get response body bytes")
}
