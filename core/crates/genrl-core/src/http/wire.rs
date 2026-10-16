use std::time::Duration;

use crate::error::{Error, Result};

/// What one in-flight request actually does on the wire.
pub(crate) enum Wire<'a> {
    Get { if_none_match: Option<&'a str> },
    Form { fields: &'a [(String, String)] },
}

pub(crate) enum Outcome {
    Success {
        status: u16,
        body: Vec<u8>,
        etag: Option<String>,
        set_cookie: Option<String>,
    },
    RateLimited {
        retry_after: Option<Duration>,
    },
    ServerError {
        status: u16,
        body: Option<String>,
    },
    ClientError {
        status: u16,
        body: Option<String>,
    },
}

pub(crate) async fn read_body(
    resp: reqwest::Response,
    cap: usize,
    url: &str,
    timeout: Duration,
) -> Result<Vec<u8>> {
    let mut resp = resp;
    let mut body: Vec<u8> = Vec::new();
    loop {
        match resp.chunk().await {
            Ok(Some(chunk)) => {
                let total = body.len() + chunk.len();
                if total > cap {
                    return Err(Error::BodyTooLarge {
                        url: url.to_string(),
                        got: total,
                        cap,
                    });
                }
                body.extend_from_slice(&chunk);
            }
            Ok(None) => break,
            Err(err) => {
                return if err.is_timeout() {
                    Err(Error::Timeout {
                        url: url.to_string(),
                        timeout_ms: timeout.as_millis() as u64,
                    })
                } else {
                    Err(Error::Http(err))
                };
            }
        }
    }
    Ok(body)
}
