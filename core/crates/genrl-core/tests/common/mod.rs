//! Shared responders for the service integration tests.
#![allow(dead_code)]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use wiremock::{Request, Respond, ResponseTemplate};

/// `first` for the first `fail_times` requests, then `then` forever — used to
/// make an endpoint reject a session once before accepting it.
pub struct Sequence {
    hits: Arc<AtomicUsize>,
    fail_times: usize,
    first: ResponseTemplate,
    then: ResponseTemplate,
}

impl Sequence {
    pub fn new(
        fail_times: usize,
        first: ResponseTemplate,
        then: ResponseTemplate,
    ) -> (Arc<AtomicUsize>, Sequence) {
        let hits = Arc::new(AtomicUsize::new(0));
        (
            hits.clone(),
            Sequence {
                hits,
                fail_times,
                first,
                then,
            },
        )
    }
}

impl Respond for Sequence {
    fn respond(&self, _request: &Request) -> ResponseTemplate {
        let n = self.hits.fetch_add(1, Ordering::SeqCst);
        if n < self.fail_times {
            self.first.clone()
        } else {
            self.then.clone()
        }
    }
}

/// The same response every time, counting how often it was served.
pub struct Fixed {
    hits: Arc<AtomicUsize>,
    response: ResponseTemplate,
}

impl Fixed {
    pub fn new(response: ResponseTemplate) -> (Arc<AtomicUsize>, Fixed) {
        let hits = Arc::new(AtomicUsize::new(0));
        (
            hits.clone(),
            Fixed {
                hits,
                response,
            },
        )
    }
}

impl Respond for Fixed {
    fn respond(&self, _request: &Request) -> ResponseTemplate {
        self.hits.fetch_add(1, Ordering::SeqCst);
        self.response.clone()
    }
}
