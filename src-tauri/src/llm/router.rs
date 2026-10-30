//! Provider settings, fail-closed routing and credential references.
//!
//! Settings live as rows in the existing `settings` table:
//! `provider.routing_profile` (`local` default / `hybrid` / `cloud`),
//! `provider.active` (`ollama` default), and per remote provider
//! `provider.<id>.base_url`, `provider.<id>.model`, `provider.<id>.credential_ref`.
//! Ollama keeps using the existing `endpoint` and `model` rows.
//!
//! Routing never falls back between providers. In `local` only the loopback
//! Ollama route is admitted; anything else is refused with an explanation.
//! A credential reference is bound to the destination host, so changing a
//! base URL never sends a stored key to a different server.

use anyhow::{anyhow, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use super::anthropic::{self, AnthropicProvider};
use super::client::ChatMessage;
use super::credentials::{validate_secret, CredentialStore, Secret, SystemCredentialStore};
use super::ollama::OllamaProvider;
use super::openai::OpenAiCompatibleProvider;
use super::provider::{
    remote_http_client, text_capabilities, ChatProvider, DataDestination, Locality, ProviderKind,
    StreamChatResult, StreamTimeouts, TokenSink,
};
use crate::conversation::controller::CancellationToken;
use crate::db;
use crate::domain::events::ProjectEvent;
use crate::domain::provider::ProviderCapabilities;

pub const KEY_ROUTING_PROFILE: &str = "provider.routing_profile";
pub const KEY_ACTIVE_PROVIDER: &str = "provider.active";
const MAX_MODEL_CHARS: usize = 200;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingProfile {
    #[default]
    Local,
    Hybrid,
    Cloud,
}

impl RoutingProfile {
    pub fn id(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Hybrid => "hybrid",
            Self::Cloud => "cloud",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        [Self::Local, Self::Hybrid, Self::Cloud]
            .into_iter()
            .find(|profile| profile.id() == value)
    }
}

fn setting_key(kind: ProviderKind, field: &str) -> String {
    format!("provider.{}.{}", kind.id(), field)
}

pub fn default_base_url(kind: ProviderKind) -> &'static str {
    match kind {
        ProviderKind::Ollama => "http://localhost:11434",
        ProviderKind::OpenaiCompatible => "https://api.openai.com/v1",
        ProviderKind::Anthropic => anthropic::DEFAULT_BASE_URL,
    }
}

pub fn default_model(kind: ProviderKind) -> &'static str {
    match kind {
        ProviderKind::Ollama => "llama3.2:8b",
        // No universal default exists across compatible servers.
        ProviderKind::OpenaiCompatible => "",
        ProviderKind::Anthropic => anthropic::DEFAULT_MODEL,
    }
}

#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub kind: ProviderKind,
    pub base_url: String,
    pub model: String,
    pub credential_ref: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProviderSettings {
    pub profile: RoutingProfile,
    pub active: Option<ProviderKind>,
    pub active_raw: String,
    pub configs: Vec<ProviderConfig>,
}

impl ProviderSettings {
    pub fn config(&self, kind: ProviderKind) -> &ProviderConfig {
        self.configs
            .iter()
            .find(|config| config.kind == kind)
            .expect("every provider kind has a config")
    }

    fn config_mut(&mut self, kind: ProviderKind) -> &mut ProviderConfig {
        self.configs
            .iter_mut()
            .find(|config| config.kind == kind)
            .expect("every provider kind has a config")
    }
}

/// Unknown or tampered profile values fail closed to `local`.
pub fn load_settings(conn: &Connection) -> ProviderSettings {
    let profile = db::get_setting(conn, KEY_ROUTING_PROFILE)
        .and_then(|value| RoutingProfile::parse(&value))
        .unwrap_or_default();
    let active_raw = db::get_setting(conn, KEY_ACTIVE_PROVIDER)
        .unwrap_or_else(|| ProviderKind::Ollama.id().into());
    let configs = ProviderKind::ALL
        .into_iter()
        .map(|kind| {
            let (base_key, model_key) = match kind {
                ProviderKind::Ollama => ("endpoint".to_string(), "model".to_string()),
                _ => (setting_key(kind, "base_url"), setting_key(kind, "model")),
            };
            ProviderConfig {
                kind,
                base_url: db::get_setting(conn, &base_key)
                    .unwrap_or_else(|| default_base_url(kind).into()),
                model: db::get_setting(conn, &model_key)
                    .unwrap_or_else(|| default_model(kind).into()),
                credential_ref: db::get_setting(conn, &setting_key(kind, "credential_ref")),
            }
        })
        .collect();
    ProviderSettings {
        profile,
        active: ProviderKind::parse(&active_raw),
        active_raw,
        configs,
    }
}

fn is_loopback_host(host: &str) -> bool {
    let host = host.trim_start_matches('[').trim_end_matches(']');
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
}

/// Validate a base URL and describe where requests to it terminate.
pub fn parse_base_url(kind: ProviderKind, raw: &str) -> Result<(String, DataDestination), String> {
    let url =
        reqwest::Url::parse(raw.trim()).map_err(|_| "Enter a full http(s) URL".to_string())?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("Only http and https URLs are supported".into());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("URLs must not contain credentials".into());
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err("URLs must not contain a query or fragment".into());
    }
    let host = url
        .host_str()
        .ok_or("URL has no host")?
        .to_ascii_lowercase();
    let loopback = is_loopback_host(&host);
    // Ollama has no TLS of its own, so LAN Ollama stays possible (as a remote,
    // consent-requiring destination); credential-bearing providers need TLS.
    if url.scheme() == "http" && !loopback && kind != ProviderKind::Ollama {
        return Err("Remote providers must use https".into());
    }
    let authority = match url.port() {
        Some(port) => format!("{}:{}", host, port),
        None => host,
    };
    let locality = match (loopback, kind) {
        (true, ProviderKind::Ollama) => Locality::Local,
        (true, _) => Locality::LoopbackUnverified,
        (false, _) => Locality::Remote,
    };
    let destination = DataDestination {
        provider: kind,
        host: authority,
        origin: url.origin().ascii_serialization(),
        locality,
    };
    Ok((url.as_str().trim_end_matches('/').to_string(), destination))
}

pub fn credential_ref_for(kind: ProviderKind, destination: &DataDestination) -> String {
    format!("{}@{}", kind.id(), destination.host)
}

pub fn check_profile(profile: RoutingProfile, destination: &DataDestination) -> Result<(), String> {
    if profile == RoutingProfile::Local && destination.locality != Locality::Local {
        let reason = match destination.locality {
            Locality::LoopbackUnverified => {
                "a local server whose onward data flow VoicePartner cannot verify"
            }
            _ => "a remote server",
        };
        return Err(format!(
            "Private local mode blocked {}: messages would go to {} ({}). Choose Hybrid or Cloud in Settings to allow it, or switch back to local Ollama.",
            destination.provider.label(),
            destination.host,
            reason
        ));
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct ResolvedRoute {
    pub kind: ProviderKind,
    pub base_url: String,
    pub model: String,
    pub destination: DataDestination,
    /// Present only when the stored reference matches this destination.
    pub credential_ref: Option<String>,
}

pub fn resolve_route(settings: &ProviderSettings) -> Result<ResolvedRoute, String> {
    let kind = settings.active.ok_or_else(|| {
        format!(
            "Unknown AI provider '{}' in settings. Choose a provider in Settings.",
            settings.active_raw
        )
    })?;
    let config = settings.config(kind);
    let (base_url, destination) = parse_base_url(kind, &config.base_url)
        .map_err(|e| format!("{} address is invalid: {}", kind.label(), e))?;
    check_profile(settings.profile, &destination)?;
    let model = config.model.trim();
    if model.is_empty() {
        return Err(format!("Choose a model for {} in Settings.", kind.label()));
    }
    let expected = credential_ref_for(kind, &destination);
    Ok(ResolvedRoute {
        kind,
        base_url,
        model: model.to_string(),
        credential_ref: config.credential_ref.clone().filter(|r| *r == expected),
        destination,
    })
}

pub fn load_api_key(
    route: &ResolvedRoute,
    store: &dyn CredentialStore,
) -> Result<Option<Secret>, String> {
    let key = match (&route.credential_ref, route.kind.accepts_key()) {
        (Some(reference), true) => store.get(reference).map_err(|e| e.to_string())?,
        _ => None,
    };
    if key.is_none() && route.kind.requires_key() {
        return Err(format!(
            "No API key is stored for {} at {}. Add one in Settings.",
            route.kind.label(),
            route.destination.host
        ));
    }
    Ok(key)
}

pub enum ActiveProvider {
    Ollama(OllamaProvider),
    OpenaiCompatible(OpenAiCompatibleProvider),
    Anthropic(AnthropicProvider),
}

impl ActiveProvider {
    pub fn build(route: ResolvedRoute, key: Option<Secret>) -> Result<Self> {
        let timeouts = StreamTimeouts::default();
        Ok(match route.kind {
            ProviderKind::Ollama => Self::Ollama(OllamaProvider {
                client: super::client::build_client()?,
                endpoint: route.base_url,
                model: route.model,
                destination: route.destination,
            }),
            ProviderKind::OpenaiCompatible => Self::OpenaiCompatible(OpenAiCompatibleProvider {
                client: remote_http_client()?,
                base_url: route.base_url,
                model: route.model,
                api_key: key,
                destination: route.destination,
                timeouts,
            }),
            ProviderKind::Anthropic => Self::Anthropic(AnthropicProvider {
                client: remote_http_client()?,
                base_url: route.base_url,
                model: route.model,
                api_key: key.ok_or_else(|| anyhow!("Anthropic requires an API key"))?,
                destination: route.destination,
                timeouts,
            }),
        })
    }
}

impl ChatProvider for ActiveProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        match self {
            Self::Ollama(provider) => provider.capabilities(),
            Self::OpenaiCompatible(provider) => provider.capabilities(),
            Self::Anthropic(provider) => provider.capabilities(),
        }
    }

    fn destination(&self) -> &DataDestination {
        match self {
            Self::Ollama(provider) => provider.destination(),
            Self::OpenaiCompatible(provider) => provider.destination(),
            Self::Anthropic(provider) => provider.destination(),
        }
    }

    async fn stream_chat(
        &self,
        messages: &[ChatMessage],
        cancellation: &CancellationToken,
        sink: &mut TokenSink<'_>,
    ) -> Result<StreamChatResult> {
        match self {
            Self::Ollama(provider) => provider.stream_chat(messages, cancellation, sink).await,
            Self::OpenaiCompatible(provider) => {
                provider.stream_chat(messages, cancellation, sink).await
            }
            Self::Anthropic(provider) => provider.stream_chat(messages, cancellation, sink).await,
        }
    }
}

/// Generate a reply through the configured provider. `endpoint`/`model` are
/// the Ollama settings already read by the caller; remote providers use their
/// own rows. Tokens are appended to the conversation and emitted as before.
pub async fn stream_chat(
    app: &AppHandle,
    endpoint: &str,
    model: &str,
    messages: Vec<ChatMessage>,
    cancellation: &CancellationToken,
    project_id: &str,
    turn_id: &str,
) -> Result<StreamChatResult> {
    let state = app.state::<crate::AppState>();
    let route = {
        let conn = state
            .db
            .lock()
            .map_err(|_| anyhow!("Settings are unavailable"))?;
        let mut settings = load_settings(&conn);
        let ollama = settings.config_mut(ProviderKind::Ollama);
        ollama.base_url = endpoint.into();
        ollama.model = model.into();
        resolve_route(&settings).map_err(|e| anyhow!(e))?
    };
    let key = load_api_key(&route, &SystemCredentialStore).map_err(|e| anyhow!(e))?;
    let provider = ActiveProvider::build(route, key)?;
    let destination = provider.destination();
    tracing::info!(
        provider = destination.provider.id(),
        model = provider.capabilities().model_id.as_str(),
        destination = destination.host.as_str(),
        locality = ?destination.locality,
        "chat generation route"
    );

    let mut sink = |token: &str| {
        state.conversation.append(turn_id, token);
        let _ = app.emit("conversation:snapshot", state.conversation.snapshot());
        let _ = app.emit("chat:token", ProjectEvent::new(project_id, token));
    };
    provider
        .stream_chat(&messages, cancellation, &mut sink)
        .await
}

// ── Settings view and mutations (used by commands/providers.rs) ─────────────

#[derive(Debug, Serialize)]
pub struct ProviderView {
    pub provider: ProviderKind,
    pub label: &'static str,
    pub base_url: String,
    pub model: String,
    pub destination: Option<DataDestination>,
    pub destination_error: Option<String>,
    pub accepts_key: bool,
    pub requires_key: bool,
    pub has_key: bool,
    /// Host a stored key is bound to, when one exists (never the key itself).
    pub key_destination: Option<String>,
    pub capabilities: ProviderCapabilities,
}

#[derive(Debug, Serialize)]
pub struct RouteStatus {
    pub allowed: bool,
    pub message: Option<String>,
    pub destination: Option<DataDestination>,
}

#[derive(Debug, Serialize)]
pub struct ProviderSettingsView {
    pub routing_profile: RoutingProfile,
    pub active_provider: Option<ProviderKind>,
    pub providers: Vec<ProviderView>,
    pub route: RouteStatus,
}

fn has_key(config: &ProviderConfig, store: &dyn CredentialStore) -> Result<bool, String> {
    if !config.kind.accepts_key() {
        return Ok(false);
    }
    let Ok((_, destination)) = parse_base_url(config.kind, &config.base_url) else {
        return Ok(false);
    };
    let expected = credential_ref_for(config.kind, &destination);
    if config.credential_ref.as_deref() != Some(expected.as_str()) {
        return Ok(false);
    }
    Ok(store.get(&expected).map_err(|e| e.to_string())?.is_some())
}

pub fn settings_view(conn: &Connection, store: &dyn CredentialStore) -> ProviderSettingsView {
    let settings = load_settings(conn);
    let providers = settings
        .configs
        .iter()
        .map(|config| {
            let parsed = parse_base_url(config.kind, &config.base_url);
            ProviderView {
                provider: config.kind,
                label: config.kind.label(),
                base_url: config.base_url.clone(),
                model: config.model.clone(),
                destination: parsed.as_ref().ok().map(|(_, d)| d.clone()),
                destination_error: parsed.err(),
                accepts_key: config.kind.accepts_key(),
                requires_key: config.kind.requires_key(),
                has_key: has_key(config, store).unwrap_or(false),
                key_destination: config
                    .credential_ref
                    .as_ref()
                    .and_then(|r| r.split_once('@'))
                    .map(|(_, host)| host.to_string()),
                capabilities: text_capabilities(config.kind, &config.model),
            }
        })
        .collect();
    let route = match resolve_route(&settings) {
        Ok(route) => {
            let destination = Some(route.destination.clone());
            match load_api_key(&route, store) {
                Ok(_) => RouteStatus {
                    allowed: true,
                    message: None,
                    destination,
                },
                Err(message) => RouteStatus {
                    allowed: false,
                    message: Some(message),
                    destination,
                },
            }
        }
        Err(message) => RouteStatus {
            allowed: false,
            message: Some(message),
            destination: settings
                .active
                .and_then(|kind| parse_base_url(kind, &settings.config(kind).base_url).ok())
                .map(|(_, destination)| destination),
        },
    };
    ProviderSettingsView {
        routing_profile: settings.profile,
        active_provider: settings.active,
        providers,
        route,
    }
}

fn parse_kind(raw: &str) -> Result<ProviderKind, String> {
    ProviderKind::parse(raw).ok_or_else(|| format!("Unknown provider '{}'", raw))
}

pub fn set_routing_profile(conn: &Connection, raw: &str) -> Result<(), String> {
    let profile = RoutingProfile::parse(raw)
        .ok_or_else(|| "Routing profile must be local, hybrid, or cloud".to_string())?;
    db::set_setting(conn, KEY_ROUTING_PROFILE, profile.id()).map_err(|e| e.to_string())
}

pub fn set_active_provider(conn: &Connection, raw: &str) -> Result<(), String> {
    let kind = parse_kind(raw)?;
    db::set_setting(conn, KEY_ACTIVE_PROVIDER, kind.id()).map_err(|e| e.to_string())
}

fn validate_model(raw: &str) -> Result<String, String> {
    let model = raw.trim();
    if model.is_empty() {
        return Err("Model is required".into());
    }
    if model.chars().count() > MAX_MODEL_CHARS || model.chars().any(char::is_control) {
        return Err("Model name is not valid".into());
    }
    Ok(model.to_string())
}

pub fn configure_provider(
    conn: &Connection,
    provider: &str,
    base_url: &str,
    model: &str,
) -> Result<(), String> {
    let kind = parse_kind(provider)?;
    if kind == ProviderKind::Ollama {
        return Err("Configure Ollama in the Local LLM section".into());
    }
    let (base_url, _) = parse_base_url(kind, base_url)?;
    let model = validate_model(model)?;
    let transaction = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    db::set_setting(&transaction, &setting_key(kind, "base_url"), &base_url)
        .map_err(|e| e.to_string())?;
    db::set_setting(&transaction, &setting_key(kind, "model"), &model)
        .map_err(|e| e.to_string())?;
    transaction.commit().map_err(|e| e.to_string())
}

/// Store a key for the provider's current destination. The secret goes to the
/// credential store; only the host-bound reference is written to SQLite.
pub fn set_provider_key(
    conn: &Connection,
    store: &dyn CredentialStore,
    provider: &str,
    key: &str,
) -> Result<(), String> {
    let kind = parse_kind(provider)?;
    if !kind.accepts_key() {
        return Err(format!("{} does not use an API key", kind.label()));
    }
    let secret = validate_secret(key)?;
    let settings = load_settings(conn);
    let config = settings.config(kind);
    let (_, destination) = parse_base_url(kind, &config.base_url)?;
    let reference = credential_ref_for(kind, &destination);
    store.set(&reference, &secret).map_err(|e| e.to_string())?;
    if let Err(error) = db::set_setting(conn, &setting_key(kind, "credential_ref"), &reference) {
        let _ = store.delete(&reference);
        return Err(error.to_string());
    }
    if let Some(previous) = config.credential_ref.as_ref().filter(|r| **r != reference) {
        // Do not leave an orphaned secret for the previous destination.
        let _ = store.delete(previous);
    }
    Ok(())
}

pub fn clear_provider_key(
    conn: &Connection,
    store: &dyn CredentialStore,
    provider: &str,
) -> Result<(), String> {
    let kind = parse_kind(provider)?;
    let settings = load_settings(conn);
    let config = settings.config(kind);
    if let Some(reference) = &config.credential_ref {
        store.delete(reference).map_err(|e| e.to_string())?;
    }
    if let Ok((_, destination)) = parse_base_url(kind, &config.base_url) {
        store
            .delete(&credential_ref_for(kind, &destination))
            .map_err(|e| e.to_string())?;
    }
    conn.execute(
        "DELETE FROM settings WHERE key = ?1",
        rusqlite::params![setting_key(kind, "credential_ref")],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn has_provider_key(
    conn: &Connection,
    store: &dyn CredentialStore,
    provider: &str,
) -> Result<bool, String> {
    let kind = parse_kind(provider)?;
    has_key(load_settings(conn).config(kind), store)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::credentials::MemoryCredentialStore;

    fn conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        db::migrations::run(&conn).unwrap();
        conn
    }

    const KEY: &str = "sk-unit-test-secret-789";

    #[test]
    fn defaults_route_to_local_ollama() {
        let conn = conn();
        let settings = load_settings(&conn);
        assert_eq!(settings.profile, RoutingProfile::Local);
        let route = resolve_route(&settings).unwrap();
        assert_eq!(route.kind, ProviderKind::Ollama);
        assert_eq!(route.destination.locality, Locality::Local);
        assert_eq!(route.destination.host, "localhost:11434");
    }

    #[test]
    fn local_profile_refuses_remote_and_unverified_providers() {
        let conn = conn();
        for provider in ["openai_compatible", "anthropic"] {
            set_active_provider(&conn, provider).unwrap();
            let error = resolve_route(&load_settings(&conn)).unwrap_err();
            assert!(error.contains("Private local mode blocked"), "{}", error);
            assert!(error.contains("Hybrid"));
        }
        // A loopback OpenAI-compatible server (e.g. LM Studio) is not proof of
        // local processing.
        configure_provider(&conn, "openai_compatible", "http://localhost:1234/v1", "m").unwrap();
        set_active_provider(&conn, "openai_compatible").unwrap();
        let error = resolve_route(&load_settings(&conn)).unwrap_err();
        assert!(error.contains("cannot verify"));
        // Ollama on another machine is a remote destination too.
        db::set_setting(&conn, "endpoint", "http://192.168.1.20:11434").unwrap();
        set_active_provider(&conn, "ollama").unwrap();
        assert!(resolve_route(&load_settings(&conn)).is_err());
    }

    #[test]
    fn hybrid_and_cloud_admit_the_selected_remote_provider_only() {
        let conn = conn();
        set_active_provider(&conn, "anthropic").unwrap();
        for profile in ["hybrid", "cloud"] {
            set_routing_profile(&conn, profile).unwrap();
            let route = resolve_route(&load_settings(&conn)).unwrap();
            assert_eq!(route.kind, ProviderKind::Anthropic);
            assert_eq!(route.destination.host, "api.anthropic.com");
            assert_eq!(route.destination.locality, Locality::Remote);
            assert_eq!(route.model, "claude-sonnet-5-5");
        }
    }

    #[test]
    fn missing_key_fails_instead_of_falling_back() {
        let conn = conn();
        let store = MemoryCredentialStore::default();
        set_routing_profile(&conn, "cloud").unwrap();
        set_active_provider(&conn, "anthropic").unwrap();
        let route = resolve_route(&load_settings(&conn)).unwrap();
        let error = load_api_key(&route, &store).unwrap_err();
        assert!(error.contains("No API key"));
        let view = settings_view(&conn, &store);
        assert!(!view.route.allowed);
        assert_eq!(view.active_provider, Some(ProviderKind::Anthropic));
    }

    #[test]
    fn invalid_stored_values_fail_closed() {
        let conn = conn();
        db::set_setting(&conn, KEY_ROUTING_PROFILE, "everything").unwrap();
        db::set_setting(&conn, KEY_ACTIVE_PROVIDER, "anthropic").unwrap();
        // Tampered profile behaves as local, so the remote route is refused.
        assert!(resolve_route(&load_settings(&conn)).is_err());
        db::set_setting(&conn, KEY_ACTIVE_PROVIDER, "mystery").unwrap();
        let error = resolve_route(&load_settings(&conn)).unwrap_err();
        assert!(error.contains("Unknown AI provider"));
        assert!(set_routing_profile(&conn, "public").is_err());
        assert!(set_active_provider(&conn, "mystery").is_err());
    }

    #[test]
    fn base_urls_are_validated() {
        let kind = ProviderKind::OpenaiCompatible;
        assert!(parse_base_url(kind, "http://api.openai.com/v1").is_err());
        assert!(parse_base_url(kind, "https://user:pw@api.openai.com/v1").is_err());
        assert!(parse_base_url(kind, "https://api.openai.com/v1?key=x").is_err());
        assert!(parse_base_url(kind, "ftp://example.com").is_err());
        assert!(parse_base_url(kind, "api.openai.com").is_err());
        let (url, destination) = parse_base_url(kind, "https://OpenRouter.ai/api/v1/").unwrap();
        assert_eq!(url, "https://openrouter.ai/api/v1");
        assert_eq!(destination.host, "openrouter.ai");
        let (_, destination) = parse_base_url(kind, "http://[::1]:8080/v1").unwrap();
        assert_eq!(destination.locality, Locality::LoopbackUnverified);
        assert!(configure_provider(&conn(), "ollama", "http://localhost:1", "m").is_err());
        assert!(
            configure_provider(&conn(), "anthropic", "https://api.anthropic.com", " ").is_err()
        );
    }

    #[test]
    fn keys_live_only_in_the_credential_store() {
        let conn = conn();
        let store = MemoryCredentialStore::default();
        set_routing_profile(&conn, "hybrid").unwrap();
        set_active_provider(&conn, "openai_compatible").unwrap();
        configure_provider(
            &conn,
            "openai_compatible",
            "https://api.openai.com/v1",
            "gpt-x",
        )
        .unwrap();
        set_provider_key(&conn, &store, "openai_compatible", &format!("  {}  ", KEY)).unwrap();

        assert!(has_provider_key(&conn, &store, "openai_compatible").unwrap());
        assert_eq!(store.references(), vec!["openai_compatible@api.openai.com"]);
        for (key, value) in db::get_all_settings(&conn) {
            assert!(!value.contains(KEY), "secret leaked into setting {}", key);
        }
        let view = settings_view(&conn, &store);
        let json = serde_json::to_string(&view).unwrap();
        assert!(!json.contains(KEY));
        assert!(view.route.allowed);
        let route = resolve_route(&load_settings(&conn)).unwrap();
        let loaded = load_api_key(&route, &store).unwrap().unwrap();
        assert_eq!(loaded.expose(), KEY);
        assert!(!format!("{:?}", route).contains(KEY));

        clear_provider_key(&conn, &store, "openai_compatible").unwrap();
        assert!(store.references().is_empty());
        assert!(!has_provider_key(&conn, &store, "openai_compatible").unwrap());
        assert!(set_provider_key(&conn, &store, "ollama", KEY).is_err());
        assert!(set_provider_key(&conn, &store, "anthropic", "bad key\n").is_err());
    }

    #[test]
    fn keys_are_bound_to_their_destination_host() {
        let conn = conn();
        let store = MemoryCredentialStore::default();
        set_routing_profile(&conn, "hybrid").unwrap();
        set_active_provider(&conn, "openai_compatible").unwrap();
        configure_provider(
            &conn,
            "openai_compatible",
            "https://api.openai.com/v1",
            "gpt-x",
        )
        .unwrap();
        set_provider_key(&conn, &store, "openai_compatible", KEY).unwrap();

        configure_provider(
            &conn,
            "openai_compatible",
            "https://evil.example/v1",
            "gpt-x",
        )
        .unwrap();
        let route = resolve_route(&load_settings(&conn)).unwrap();
        assert!(route.credential_ref.is_none());
        assert!(load_api_key(&route, &store).unwrap().is_none());
        let view = settings_view(&conn, &store);
        let entry = view
            .providers
            .iter()
            .find(|p| p.provider == ProviderKind::OpenaiCompatible)
            .unwrap();
        assert!(!entry.has_key);
        assert_eq!(entry.key_destination.as_deref(), Some("api.openai.com"));

        // Replacing the key for the new host removes the old host's secret.
        set_provider_key(&conn, &store, "openai_compatible", "sk-second").unwrap();
        assert_eq!(store.references(), vec!["openai_compatible@evil.example"]);
    }
}
