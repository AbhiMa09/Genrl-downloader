use std::collections::HashMap;
use std::sync::Mutex;

use serde::Deserialize;

use crate::clock::Clock;
use crate::db::Db;
use crate::error::{Error, Result};
use crate::http::{Fetched, Gate, GetOptions, Service};
use crate::settings::Settings;

use super::models::{
    build_chains, to_mal_entry, Account, AnimeDetail, FetchedList, ListItem, MalEntry, Relation,
};
use super::pkce::{
    LIST_FIELDS, LIST_PATH, MAX_PAGES, PAGE_SIZE, REFRESH_MARGIN_SECS, RELATIONS_FIELD, TOKEN_PATH,
    TOKEN_URL,
};
use super::token::{TokenStore, Tokens};

#[derive(Debug, Deserialize)]
pub(crate) struct TokenResponse {
    pub(crate) access_token: String,
    #[serde(default)]
    pub(crate) refresh_token: Option<String>,
    #[serde(default = "default_expires_in")]
    pub(crate) expires_in: i64,
}

fn default_expires_in() -> i64 {
    3600
}

#[derive(Debug, Deserialize)]
pub(crate) struct ListPage {
    pub(crate) data: Vec<ListItem>,
    #[serde(default)]
    pub(crate) paging: Option<Paging>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Paging {
    #[serde(default)]
    pub(crate) next: Option<String>,
}

/// One MAL session: a gate, the settings and a per-process token cache.
pub struct MalClient<'a> {
    gate: &'a Gate,
    settings: Settings,
    store: &'a dyn TokenStore,
    clock: &'a dyn Clock,
    tokens: Mutex<Option<Tokens>>,
}

impl<'a> MalClient<'a> {
    pub fn new(
        gate: &'a Gate,
        settings: Settings,
        store: &'a dyn TokenStore,
        clock: &'a dyn Clock,
    ) -> Self {
        MalClient {
            gate,
            settings,
            store,
            clock,
            tokens: Mutex::new(None),
        }
    }

    pub fn clock(&self) -> &dyn Clock {
        self.clock
    }
    pub fn settings(&self) -> &Settings {
        &self.settings
    }
    pub fn db(&self) -> &Db {
        self.gate.db()
    }

    /// A refresh token is stored and usable.
    pub fn is_connected(&self) -> bool {
        self.store.refresh_token().ok().flatten().is_some()
    }

    fn base(&self) -> &str {
        self.settings.mal_base_url.trim_end_matches('/')
    }

    /// MAL hosts its OAuth token endpoint on `myanimelist.net/v1/oauth2/token`
    /// rather than the API host `api.myanimelist.net/v2`. In test suites or
    /// custom setups where `mal_base_url` points to a mock server, fall back
    /// to `{base}{TOKEN_PATH}`.
    fn token_url(&self) -> String {
        let base = self.base();
        if base.contains("myanimelist.net") {
            TOKEN_URL.to_string()
        } else {
            format!("{base}{TOKEN_PATH}")
        }
    }

    fn client_id(&self) -> Result<String> {
        let id = self.settings.mal_client_id.trim().to_string();
        if id.is_empty() {
            return Err(Error::Auth {
                service: "mal".to_string(),
                message: "no MAL client id is set; add one to settings.mal_client_id"
                    .to_string(),
            });
        }
        Ok(id)
    }

    /// A valid access token, refreshing five minutes before it expires.
    pub async fn access_token(&self) -> Result<String> {
        let now = self.clock.now();
        {
            let guard = self.tokens.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(tokens) = guard.as_ref() {
                if now < tokens.expires_at - REFRESH_MARGIN_SECS {
                    return Ok(tokens.access_token.clone());
                }
            }
        }

        let refresh = self.store.refresh_token()?.ok_or_else(|| Error::Auth {
            service: "mal".to_string(),
            message: "not connected to MyAnimeList; run `genrl mal login` first".to_string(),
        })?;
        let client_id = self.client_id()?;
        let tokens = self.refresh(&client_id, &refresh).await?;
        {
            let mut guard = self.tokens.lock().unwrap_or_else(|e| e.into_inner());
            *guard = Some(tokens.clone());
        }
        Ok(tokens.access_token)
    }

    async fn refresh(&self, client_id: &str, refresh_token: &str) -> Result<Tokens> {
        let url = self.token_url();
        let mut form = vec![
            ("grant_type".to_string(), "refresh_token".to_string()),
            ("client_id".to_string(), client_id.to_string()),
            ("refresh_token".to_string(), refresh_token.to_string()),
        ];
        let secret = self
            .store
            .client_secret()
            .ok()
            .flatten()
            .or_else(|| std::env::var("GENRL_MAL_CLIENT_SECRET").ok());
        if let Some(s) = secret {
            if !s.trim().is_empty() {
                form.push(("client_secret".to_string(), s.trim().to_string()));
            }
        }
        self.token_request(&url, &form).await
    }

    /// Swap a login code for tokens. The token endpoint rotates the refresh
    /// token; the fresh one is stored before this returns.
    pub(crate) async fn exchange_and_store(
        &self,
        redirect_uri: &str,
        code: &str,
        verifier: &str,
    ) -> Result<Tokens> {
        let client_id = self.client_id()?;
        let url = self.token_url();
        let mut form = vec![
            ("grant_type".to_string(), "authorization_code".to_string()),
            ("client_id".to_string(), client_id),
            ("code".to_string(), code.to_string()),
            ("code_verifier".to_string(), verifier.to_string()),
            ("redirect_uri".to_string(), redirect_uri.to_string()),
        ];
        let secret = self
            .store
            .client_secret()
            .ok()
            .flatten()
            .or_else(|| std::env::var("GENRL_MAL_CLIENT_SECRET").ok());
        if let Some(s) = secret {
            if !s.trim().is_empty() {
                form.push(("client_secret".to_string(), s.trim().to_string()));
            }
        }
        let tokens = self.token_request(&url, &form).await?;
        {
            let mut guard = self.tokens.lock().unwrap_or_else(|e| e.into_inner());
            *guard = Some(tokens.clone());
        }
        Ok(tokens)
    }

    /// Token endpoint, hosted on myanimelist.net (not the api host).
    async fn token_request(&self, url: &str, form: &[(String, String)]) -> Result<Tokens> {
        let body_result = self.gate.post_form(Service::Mal, url, form, &GetOptions::default()).await;
        let fetched = match body_result {
            Ok(fetched) => fetched,
            Err(Error::Status { status, url, body, .. }) => {
                let reason = body.unwrap_or_default();
                let detail = if reason.is_empty() {
                    format!("token endpoint returned HTTP {status} for {url}")
                } else {
                    format!("token endpoint returned HTTP {status}: {reason}")
                };
                return Err(Error::Auth {
                    service: "mal".to_string(),
                    message: format!(
                        "{detail}\nChecklist:\n1. Verify client_id in Settings.\n2. Ensure redirect URI registered in MAL API config matches http://localhost:<port>/callback exactly."
                    ),
                });
            }
            Err(err) => return Err(err),
        };
        let body = fetched.text().unwrap_or_default();
        let raw: TokenResponse = match fetched.json() {
            Ok(raw) => raw,
            Err(_) if !body.is_empty() => {
                return Err(Error::Auth {
                    service: "mal".to_string(),
                    message: format!("token endpoint response: {body}"),
                });
            }
            Err(err) => return Err(err),
        };
        if let Some(rotated) = raw.refresh_token.as_deref() {
            self.store.set_refresh_token(Some(rotated))?;
        }
        Ok(Tokens {
            access_token: raw.access_token,
            expires_at: self.clock.now() + raw.expires_in,
        })
    }

    /// GET through the gate with the bearer token on the header. The token
    /// never becomes part of the URL, so the gate's response cache cannot leak
    /// it anywhere either.
    async fn get(&self, url: &str) -> Result<Fetched> {
        let token = self.access_token().await?;
        let opts = GetOptions {
            headers: vec![("Authorization".to_string(), format!("Bearer {token}"))],
            ..Default::default()
        };
        self.gate.get_with(Service::Mal, url, &opts).await
    }

    /// The logged-in account (for `genrl mal status`/`login`).
    pub async fn account(&self) -> Result<Account> {
        let url = format!("{}/users/@me", self.base());
        let fetched = self.get(&url).await?;
        Ok(fetched.json()?)
    }

    /// The whole list, paged, with chains built from MAL relations.
    pub async fn fetch(&self) -> Result<FetchedList> {
        let mut entries = self.fetch_list().await?;
        let mut relations: HashMap<i64, Vec<Relation>> = HashMap::new();
        let mut relations_failed = 0usize;
        let mut first_error: Option<Error> = None;

        for entry in &entries {
            let id = entry.entry.mal_id;
            match self.relations(id).await {
                Ok(rels) => {
                    relations.insert(id, rels);
                }
                Err(err) => {
                    if first_error.is_none() {
                        first_error = Some(err);
                    }
                    relations_failed += 1;
                    tracing::warn!(
                        mal_id = id,
                        "MAL relation lookup failed; the entry stays its own chain"
                    );
                }
            }
        }

        if relations_failed > 0 && relations_failed == entries.len() {
            // Every lookup failed: something systematic (expired token, MAL
            // down). Swallowing it would hide a real outage behind a wall of
            // singleton chains.
            if let Some(err) = first_error {
                return Err(err);
            }
        }

        build_chains(&mut entries, &relations);
        Ok(FetchedList {
            entries,
            relations_failed,
        })
    }

    async fn fetch_list(&self) -> Result<Vec<MalEntry>> {
        let mut out = Vec::new();
        let mut offset = 0usize;
        for _ in 0..MAX_PAGES {
            let url = format!(
                "{}{LIST_PATH}?fields={LIST_FIELDS}&limit={PAGE_SIZE}&offset={offset}&nsfw=true",
                self.base(),
            );
            let fetched = self.get(&url).await?;
            let page: ListPage = fetched.json()?;
            let n = page.data.len();
            if n == 0 {
                break;
            }
            out.extend(page.data.iter().map(to_mal_entry));
            offset += n;
            let has_more = page
                .paging
                .as_ref()
                .and_then(|p| p.next.as_ref())
                .is_some();
            if !has_more {
                break;
            }
        }
        Ok(out)
    }

    async fn relations(&self, mal_id: i64) -> Result<Vec<Relation>> {
        let url = format!(
            "{}/anime/{mal_id}?fields={RELATIONS_FIELD}",
            self.base(),
        );
        let fetched = self.get(&url).await?;
        let detail: AnimeDetail = fetched.json()?;
        Ok(detail.related_anime)
    }
}
