use crate::near::agent::host;

pub const GRAPH_API_BASE: &str = "https://graph.microsoft.com/v1.0";
pub const SIMPLE_UPLOAD_LIMIT: usize = 4 * 1024 * 1024;

pub const DOCX_MIME: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
pub const PPTX_MIME: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.presentation";
pub const OCTET_STREAM_MIME: &str = "application/octet-stream";

pub fn require_token() -> Result<(), String> {
    if host::secret_exists("microsoft_oauth_token") {
        Ok(())
    } else {
        Err(
            "Microsoft OAuth token not configured. Run `ironclaw tool auth microsoft` \
             after exporting MICROSOFT_OAUTH_CLIENT_ID and MICROSOFT_OAUTH_CLIENT_SECRET."
                .to_string(),
        )
    }
}

pub fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push('%');
                out.push(char::from(b"0123456789ABCDEF"[(b >> 4) as usize]));
                out.push(char::from(b"0123456789ABCDEF"[(b & 0xf) as usize]));
            }
        }
    }
    out
}

pub fn request(
    method: &str,
    endpoint: &str,
    body: Option<&str>,
) -> Result<(u16, serde_json::Value), String> {
    let url = format!("{}{}", GRAPH_API_BASE, endpoint);
    let headers = if body.is_some() {
        r#"{"Content-Type": "application/json; charset=utf-8"}"#
    } else {
        "{}"
    };
    let body_bytes = body.map(|b| b.as_bytes().to_vec());

    host::log(
        host::LogLevel::Debug,
        &format!("Graph API: {} {}", method, endpoint),
    );

    let response = host::http_request(method, &url, headers, body_bytes.as_deref(), None)?;
    let body_text = String::from_utf8(response.body)
        .map_err(|e| format!("Invalid UTF-8 in Graph response: {}", e))?;

    if response.status < 200 || response.status >= 300 {
        let reason = extract_error(&body_text).unwrap_or_else(|| body_text.clone());
        return Err(format!(
            "Microsoft Graph returned {}: {}",
            response.status, reason
        ));
    }

    if body_text.is_empty() {
        return Ok((response.status, serde_json::Value::Null));
    }

    let parsed =
        serde_json::from_str(&body_text).map_err(|e| format!("Invalid JSON from Graph: {}", e))?;
    Ok((response.status, parsed))
}

pub fn put_bytes(
    endpoint: &str,
    bytes: &[u8],
    content_type: &str,
) -> Result<serde_json::Value, String> {
    let url = format!("{}{}", GRAPH_API_BASE, endpoint);
    let headers = format!(r#"{{"Content-Type": "{}"}}"#, content_type);

    host::log(
        host::LogLevel::Debug,
        &format!("Graph upload: PUT {} ({} bytes)", endpoint, bytes.len()),
    );

    let response = host::http_request("PUT", &url, &headers, Some(bytes), None)?;
    let body_text = String::from_utf8(response.body)
        .map_err(|e| format!("Invalid UTF-8 in upload response: {}", e))?;

    if response.status < 200 || response.status >= 300 {
        let reason = extract_error(&body_text).unwrap_or_else(|| body_text.clone());
        return Err(format!(
            "Upload failed with {}: {}",
            response.status, reason
        ));
    }

    serde_json::from_str(&body_text).map_err(|e| format!("Invalid JSON from upload: {}", e))
}

pub fn extract_error(body: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(body).ok()?;
    let err = parsed.get("error")?;
    let code = err.get("code").and_then(|c| c.as_str()).unwrap_or("");
    let message = err.get("message").and_then(|m| m.as_str()).unwrap_or("");
    if code.is_empty() && message.is_empty() {
        return None;
    }
    Some(format!("{}: {}", code, message))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_encode_preserves_unreserved() {
        assert_eq!(url_encode("abcXYZ123-_.~"), "abcXYZ123-_.~");
    }

    #[test]
    fn url_encode_percent_escapes_reserved() {
        assert_eq!(url_encode("a b/c?d=e&f"), "a%20b%2Fc%3Fd%3De%26f");
    }

    #[test]
    fn url_encode_handles_unicode_bytes() {
        assert_eq!(url_encode("é"), "%C3%A9");
    }

    #[test]
    fn extract_error_returns_code_and_message() {
        let body = r#"{"error":{"code":"InvalidAuthenticationToken","message":"Token expired"}}"#;
        assert_eq!(
            extract_error(body),
            Some("InvalidAuthenticationToken: Token expired".to_string())
        );
    }

    #[test]
    fn extract_error_returns_none_for_non_error_payload() {
        assert!(extract_error(r#"{"value":[]}"#).is_none());
    }

    #[test]
    fn extract_error_returns_none_for_invalid_json() {
        assert!(extract_error("not json").is_none());
    }
}
