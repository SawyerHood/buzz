use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use bytes::Bytes;
use reqwest::{multipart, Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{sync::Arc, time::Duration};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::Mutex as AsyncMutex,
    time::timeout,
};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::{
    auth_store::{now_epoch_seconds, AuthMethod, AuthStore},
    oauth,
};

use super::{
    normalize_transcript_text, TranscriptionError, TranscriptionOptions, TranscriptionProvider,
    TranscriptionResult,
};

const DEFAULT_CHATGPT_ENDPOINT: &str = "https://chatgpt.com/backend-api/transcribe";
const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 180;
const CHATGPT_ACCOUNT_HEADER: &str = "ChatGPT-Account-Id";
const CODEX_BASE64_HEADER: &str = "X-Codex-Base64";
const CODEX_BASE64_HEADER_VALUE: &str = "1";
const CHATGPT_WARMUP_URL: &str = "https://chatgpt.com/";
const BRIDGE_CALLBACK_PATH: &str = "/voice/chatgpt-transcribe-callback";
const BRIDGE_REQUEST_TIMEOUT_SECS: u64 = 180;
const BRIDGE_MAX_RESPONSE_BODY_LEN: usize = 2_000;
const BRIDGE_MAX_REQUEST_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone)]
pub struct ChatGptTranscriptionConfig {
    pub endpoint: String,
    pub request_timeout_secs: u64,
}

impl Default for ChatGptTranscriptionConfig {
    fn default() -> Self {
        Self {
            endpoint: DEFAULT_CHATGPT_ENDPOINT.to_string(),
            request_timeout_secs: DEFAULT_REQUEST_TIMEOUT_SECS,
        }
    }
}

impl ChatGptTranscriptionConfig {
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Some(endpoint) = read_non_empty_env("CHATGPT_TRANSCRIPTION_ENDPOINT") {
            config.endpoint = endpoint;
        }

        if let Some(timeout_secs) = read_u64_env("CHATGPT_TRANSCRIPTION_TIMEOUT_SECS") {
            config.request_timeout_secs = timeout_secs.max(1);
        }

        debug!(
            endpoint = %config.endpoint,
            request_timeout_secs = config.request_timeout_secs,
            "loaded ChatGPT transcription config"
        );

        config
    }
}

#[derive(Debug, Clone)]
pub struct ChatGptTranscriptionProvider {
    client: Client,
    config: ChatGptTranscriptionConfig,
    auth_store: AuthStore,
    request_lock: Arc<AsyncMutex<()>>,
}

#[derive(Debug, Clone)]
struct ChatGptAuthContext {
    access_token: String,
    account_id: String,
}

impl ChatGptTranscriptionProvider {
    pub fn new(config: ChatGptTranscriptionConfig, auth_store: AuthStore) -> Self {
        info!(
            endpoint = %config.endpoint,
            request_timeout_secs = config.request_timeout_secs,
            "ChatGPT transcription provider initialized"
        );

        Self {
            client: build_client(&config),
            config,
            auth_store,
            request_lock: Arc::new(AsyncMutex::new(())),
        }
    }

    async fn auth_context(&self) -> Result<ChatGptAuthContext, TranscriptionError> {
        let method = self
            .auth_store
            .current_auth_method()
            .map_err(TranscriptionError::Provider)?;

        if method != AuthMethod::ChatgptOauth {
            return Err(TranscriptionError::Authentication(
                "ChatGPT OAuth login is not active".to_string(),
            ));
        }

        let Some(credentials) = self
            .auth_store
            .chatgpt_credentials()
            .map_err(TranscriptionError::Provider)?
        else {
            return Err(TranscriptionError::Authentication(
                "Missing ChatGPT OAuth credentials. Please login again.".to_string(),
            ));
        };

        if credentials.expires_at <= now_epoch_seconds().saturating_add(60) {
            warn!("ChatGPT OAuth token expired or near expiry; refreshing");
            let refreshed = oauth::refresh_access_token(&credentials.refresh_token)
                .await
                .map_err(TranscriptionError::Authentication)?;

            let refreshed_refresh_token = refreshed
                .refresh_token
                .unwrap_or(credentials.refresh_token.clone());
            let refreshed_account_id = refreshed
                .account_id
                .unwrap_or(credentials.account_id.clone());

            self.auth_store
                .update_chatgpt_tokens(
                    &refreshed.access_token,
                    &refreshed_refresh_token,
                    refreshed.expires_at,
                    &refreshed_account_id,
                )
                .map_err(TranscriptionError::Provider)?;

            return Ok(ChatGptAuthContext {
                access_token: refreshed.access_token,
                account_id: refreshed_account_id,
            });
        }

        Ok(ChatGptAuthContext {
            access_token: credentials.access_token,
            account_id: credentials.account_id,
        })
    }

    fn build_form(&self, audio_data: Vec<u8>) -> Result<multipart::Form, TranscriptionError> {
        let encoded_audio = BASE64_STANDARD.encode(Bytes::from(audio_data));
        let audio_len = u64::try_from(encoded_audio.len())
            .map_err(|_| TranscriptionError::Provider("Audio upload is too large".to_string()))?;

        let file_part = multipart::Part::stream_with_length(encoded_audio.into_bytes(), audio_len)
            .file_name("audio.wav")
            .mime_str("application/octet-stream")
            .map_err(|error| {
                TranscriptionError::Provider(format!("Unable to prepare audio upload: {error}"))
            })?;

        Ok(multipart::Form::new().part("file", file_part))
    }

    pub async fn transcribe_via_webview(
        &self,
        app: &AppHandle,
        audio_data: Vec<u8>,
        options: TranscriptionOptions,
    ) -> Result<TranscriptionResult, TranscriptionError> {
        let _request_guard = self.request_lock.lock().await;

        let TranscriptionOptions {
            on_delta,
            language: _,
            prompt: _,
            context_hint: _,
        } = options;

        let auth = self.auth_context().await?;
        let window = self.ensure_auth_window(app)?;
        self.warmup_auth_window(&window).await?;

        let request_id = Uuid::new_v4().to_string();
        let payload = WebviewBridgeRequest {
            request_id: request_id.clone(),
            endpoint: self.config.endpoint.clone(),
            callback_url: String::new(),
            audio_base64: BASE64_STANDARD.encode(audio_data),
            access_token: auth.access_token,
            account_id: auth.account_id,
        };

        info!(
            request_id = %payload.request_id,
            endpoint = %payload.endpoint,
            "starting ChatGPT transcription request via webview bridge"
        );

        let callback = self.invoke_webview_bridge(&window, payload).await?;
        if !callback.ok {
            return Err(map_bridge_http_error(
                callback.status,
                callback.body.as_deref(),
                callback.error.as_deref(),
            ));
        }

        let body = callback.body.unwrap_or_default();
        let payload =
            serde_json::from_str::<ChatGptTranscriptionResponse>(&body).map_err(|error| {
                TranscriptionError::InvalidResponse(format!(
                    "Unable to parse ChatGPT transcription response: {error}"
                ))
            })?;

        let normalized = normalize_transcript_text(&payload.text);
        if let Some(callback) = on_delta {
            callback(normalized.clone());
        }

        Ok(TranscriptionResult {
            text: normalized,
            language: None,
            duration_secs: None,
            confidence: None,
        })
    }

    fn ensure_auth_window(&self, app: &AppHandle) -> Result<WebviewWindow, TranscriptionError> {
        if let Some(window) = app.get_webview_window(oauth::CHATGPT_AUTH_WINDOW_LABEL) {
            return Ok(window);
        }

        let initial_url = reqwest::Url::parse(CHATGPT_WARMUP_URL).map_err(|error| {
            TranscriptionError::Provider(format!("Invalid ChatGPT warmup URL: {error}"))
        })?;

        WebviewWindowBuilder::new(
            app,
            oauth::CHATGPT_AUTH_WINDOW_LABEL,
            WebviewUrl::External(initial_url),
        )
        .title("Login with ChatGPT")
        .inner_size(980.0, 760.0)
        .min_inner_size(700.0, 520.0)
        .resizable(true)
        .visible(false)
        .build()
        .map_err(|error| {
            TranscriptionError::Provider(format!("Failed to create ChatGPT auth webview: {error}"))
        })
    }

    async fn warmup_auth_window(&self, window: &WebviewWindow) -> Result<(), TranscriptionError> {
        let current_url = window.url().map_err(|error| {
            TranscriptionError::Provider(format!(
                "Failed to inspect ChatGPT auth webview URL: {error}"
            ))
        })?;
        let should_navigate = current_url.domain() != Some("chatgpt.com");

        if should_navigate {
            let warmup_url = reqwest::Url::parse(CHATGPT_WARMUP_URL).map_err(|error| {
                TranscriptionError::Provider(format!("Invalid ChatGPT warmup URL: {error}"))
            })?;
            window.navigate(warmup_url).map_err(|error| {
                TranscriptionError::Provider(format!(
                    "Failed to navigate ChatGPT auth webview for warmup: {error}"
                ))
            })?;
        }

        // Give chatgpt.com time to complete navigation/challenge scripts before fetch().
        tokio::time::sleep(Duration::from_millis(1200)).await;

        if let Err(error) = window.hide() {
            warn!(%error, "failed to keep ChatGPT auth webview hidden");
        }

        Ok(())
    }

    async fn invoke_webview_bridge(
        &self,
        window: &WebviewWindow,
        request: WebviewBridgeRequest,
    ) -> Result<WebviewBridgeCallback, TranscriptionError> {
        let callback_listener = TcpListener::bind(("127.0.0.1", 0)).await.map_err(|error| {
            TranscriptionError::Network(format!(
                "Failed to bind webview bridge callback listener: {error}"
            ))
        })?;
        let callback_port = callback_listener
            .local_addr()
            .map_err(|error| {
                TranscriptionError::Network(format!(
                    "Failed to inspect webview bridge callback address: {error}"
                ))
            })?
            .port();
        let callback_url = format!(
            "http://127.0.0.1:{callback_port}{BRIDGE_CALLBACK_PATH}?requestId={}",
            request.request_id
        );

        let request = WebviewBridgeRequest {
            callback_url,
            ..request
        };
        let expected_request_id = request.request_id.clone();
        let script = build_webview_bridge_script(&request)?;

        window.eval(script).map_err(|error| {
            TranscriptionError::Provider(format!(
                "Failed to execute ChatGPT webview bridge script: {error}"
            ))
        })?;

        timeout(
            Duration::from_secs(
                self.config
                    .request_timeout_secs
                    .max(BRIDGE_REQUEST_TIMEOUT_SECS),
            ),
            wait_for_webview_bridge_callback(callback_listener, &expected_request_id),
        )
        .await
        .map_err(|_| {
            TranscriptionError::Network(
                "Timed out waiting for ChatGPT webview transcription response".to_string(),
            )
        })?
    }
}

#[async_trait]
impl TranscriptionProvider for ChatGptTranscriptionProvider {
    fn name(&self) -> &'static str {
        "chatgpt-oauth"
    }

    async fn transcribe(
        &self,
        audio_data: Vec<u8>,
        options: TranscriptionOptions,
    ) -> Result<TranscriptionResult, TranscriptionError> {
        let TranscriptionOptions {
            on_delta,
            language: _,
            prompt: _,
            context_hint: _,
        } = options;

        let auth = self.auth_context().await?;
        let form = self.build_form(audio_data)?;

        info!(endpoint = %self.config.endpoint, "starting ChatGPT transcription request");
        let response = self
            .client
            .post(&self.config.endpoint)
            .bearer_auth(auth.access_token)
            .header(CHATGPT_ACCOUNT_HEADER, auth.account_id)
            .header(CODEX_BASE64_HEADER, CODEX_BASE64_HEADER_VALUE)
            .multipart(form)
            .send()
            .await
            .map_err(map_transport_error)?;

        if !response.status().is_success() {
            return Err(map_http_error(response).await);
        }

        let payload = response
            .json::<ChatGptTranscriptionResponse>()
            .await
            .map_err(|error| {
                TranscriptionError::InvalidResponse(format!(
                    "Unable to parse ChatGPT transcription response: {error}"
                ))
            })?;

        let normalized = normalize_transcript_text(&payload.text);
        if let Some(callback) = on_delta {
            callback(normalized.clone());
        }

        Ok(TranscriptionResult {
            text: normalized,
            language: None,
            duration_secs: None,
            confidence: None,
        })
    }
}

#[derive(Debug, Deserialize)]
struct ChatGptTranscriptionResponse {
    text: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WebviewBridgeRequest {
    request_id: String,
    endpoint: String,
    callback_url: String,
    audio_base64: String,
    access_token: String,
    account_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WebviewBridgeCallback {
    request_id: String,
    ok: bool,
    #[serde(default)]
    status: Option<u16>,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

fn build_webview_bridge_script(
    request: &WebviewBridgeRequest,
) -> Result<String, TranscriptionError> {
    let payload_json = serde_json::to_string(request).map_err(|error| {
        TranscriptionError::Provider(format!(
            "Failed to serialize webview transcription payload: {error}"
        ))
    })?;

    let template = r#"(async () => {
  const payload = __VOICE_CHATGPT_PAYLOAD__;
  const reportResult = (result) => {
    const serialized = encodeURIComponent(JSON.stringify(result));
    window.location.assign(`${payload.callbackUrl}&payload=${serialized}`);
  };

  try {
    const binary = atob(payload.audioBase64);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i += 1) {
      bytes[i] = binary.charCodeAt(i);
    }

    const form = new FormData();
    form.append("file", new Blob([bytes], { type: "audio/wav" }), "audio.wav");

    const response = await fetch(payload.endpoint, {
      method: "POST",
      credentials: "include",
      headers: {
        "Authorization": `Bearer ${payload.accessToken}`,
        "ChatGPT-Account-Id": payload.accountId,
        "X-Codex-Base64": "1"
      },
      body: form
    });

    const text = await response.text();
    reportResult({
      requestId: payload.requestId,
      ok: response.ok,
      status: response.status,
      body: text.slice(0, __VOICE_CHATGPT_BODY_LIMIT__)
    });
  } catch (error) {
    reportResult({
      requestId: payload.requestId,
      ok: false,
      error: error instanceof Error ? error.message : String(error)
    });
  }
})();"#;

    Ok(template
        .replace("__VOICE_CHATGPT_PAYLOAD__", &payload_json)
        .replace(
            "__VOICE_CHATGPT_BODY_LIMIT__",
            &BRIDGE_MAX_RESPONSE_BODY_LEN.to_string(),
        ))
}

async fn wait_for_webview_bridge_callback(
    listener: TcpListener,
    expected_request_id: &str,
) -> Result<WebviewBridgeCallback, TranscriptionError> {
    loop {
        let (mut stream, _) = listener.accept().await.map_err(|error| {
            TranscriptionError::Network(format!(
                "Failed to accept webview bridge callback connection: {error}"
            ))
        })?;

        let (method, target, body) = read_http_request(&mut stream).await?;
        if method != "POST" && method != "GET" {
            let _ =
                respond_callback(&mut stream, "405 Method Not Allowed", "Method not allowed").await;
            continue;
        }

        let callback_url =
            reqwest::Url::parse(&format!("http://localhost{target}")).map_err(|error| {
                TranscriptionError::InvalidResponse(format!(
                    "Failed to parse webview bridge callback URL: {error}"
                ))
            })?;
        if callback_url.path() != BRIDGE_CALLBACK_PATH {
            let _ = respond_callback(&mut stream, "404 Not Found", "Not found").await;
            continue;
        }

        let payload_json = callback_url
            .query_pairs()
            .find_map(|(key, value)| (key == "payload").then_some(value.into_owned()))
            .unwrap_or_else(|| body.trim().to_string());
        if payload_json.is_empty() {
            let _ = respond_callback(&mut stream, "400 Bad Request", "Missing payload").await;
            continue;
        }

        let payload =
            serde_json::from_str::<WebviewBridgeCallback>(&payload_json).map_err(|error| {
                TranscriptionError::InvalidResponse(format!(
                    "Webview bridge callback payload was invalid JSON: {error}"
                ))
            })?;

        if payload.request_id != expected_request_id {
            let _ = respond_callback(&mut stream, "202 Accepted", "Ignored").await;
            continue;
        }

        let _ = respond_callback(&mut stream, "204 No Content", "").await;
        return Ok(payload);
    }
}

async fn read_http_request(
    stream: &mut tokio::net::TcpStream,
) -> Result<(String, String, String), TranscriptionError> {
    let mut buffer = Vec::<u8>::with_capacity(4096);
    let mut chunk = [0_u8; 2048];
    let mut header_end = None;

    loop {
        let bytes_read = stream.read(&mut chunk).await.map_err(|error| {
            TranscriptionError::Network(format!(
                "Failed to read webview bridge callback request: {error}"
            ))
        })?;
        if bytes_read == 0 {
            break;
        }

        buffer.extend_from_slice(&chunk[..bytes_read]);
        if buffer.len() > BRIDGE_MAX_REQUEST_BYTES {
            return Err(TranscriptionError::InvalidResponse(
                "Webview bridge callback exceeded max request size".to_string(),
            ));
        }

        if let Some(index) = buffer.windows(4).position(|window| window == b"\r\n\r\n") {
            header_end = Some(index + 4);
            break;
        }
    }

    let header_end = header_end.ok_or_else(|| {
        TranscriptionError::InvalidResponse(
            "Webview bridge callback did not include HTTP headers".to_string(),
        )
    })?;

    let headers_text = String::from_utf8(buffer[..header_end].to_vec()).map_err(|error| {
        TranscriptionError::InvalidResponse(format!(
            "Webview bridge callback headers were not UTF-8: {error}"
        ))
    })?;
    let (method, target) = parse_request_line(&headers_text)?;
    let content_length = parse_content_length(&headers_text).unwrap_or(0);

    let mut body = buffer[header_end..].to_vec();
    while body.len() < content_length {
        let bytes_read = stream.read(&mut chunk).await.map_err(|error| {
            TranscriptionError::Network(format!(
                "Failed to read webview bridge callback body: {error}"
            ))
        })?;
        if bytes_read == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..bytes_read]);
        if header_end + body.len() > BRIDGE_MAX_REQUEST_BYTES {
            return Err(TranscriptionError::InvalidResponse(
                "Webview bridge callback exceeded max body size".to_string(),
            ));
        }
    }

    let body = if content_length == 0 {
        String::new()
    } else {
        let capped_len = content_length.min(body.len());
        String::from_utf8(body[..capped_len].to_vec()).map_err(|error| {
            TranscriptionError::InvalidResponse(format!(
                "Webview bridge callback body was not UTF-8: {error}"
            ))
        })?
    };

    Ok((method.to_string(), target.to_string(), body))
}

fn parse_request_line(request: &str) -> Result<(&str, &str), TranscriptionError> {
    let request_line = request.lines().next().ok_or_else(|| {
        TranscriptionError::InvalidResponse(
            "Webview bridge callback missing request line".to_string(),
        )
    })?;

    let mut parts = request_line.split_whitespace();
    let method = parts.next().ok_or_else(|| {
        TranscriptionError::InvalidResponse("Webview bridge callback missing method".to_string())
    })?;
    let target = parts.next().ok_or_else(|| {
        TranscriptionError::InvalidResponse(
            "Webview bridge callback missing request target".to_string(),
        )
    })?;

    Ok((method, target))
}

fn parse_content_length(headers: &str) -> Option<usize> {
    headers.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if !name.trim().eq_ignore_ascii_case("content-length") {
            return None;
        }
        value.trim().parse::<usize>().ok()
    })
}

async fn respond_callback(
    stream: &mut tokio::net::TcpStream,
    status: &str,
    body: &str,
) -> Result<(), TranscriptionError> {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: content-type\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(response.as_bytes())
        .await
        .map_err(|error| {
            TranscriptionError::Network(format!("Failed to write webview bridge response: {error}"))
        })?;
    let _ = stream.shutdown().await;
    Ok(())
}

fn map_bridge_http_error(
    status: Option<u16>,
    body: Option<&str>,
    bridge_error: Option<&str>,
) -> TranscriptionError {
    if let Some(error) =
        bridge_error.and_then(|value| normalize_optional_string(Some(value.to_string())))
    {
        return TranscriptionError::Network(error);
    }

    let body_text = body.unwrap_or_default();
    let message = parse_chatgpt_error_message(body_text).unwrap_or_else(|| match status {
        Some(code) => format!("ChatGPT request failed with status {code}"),
        None => "ChatGPT request failed in webview bridge".to_string(),
    });

    match status {
        Some(401) | Some(403) => TranscriptionError::Authentication(message),
        Some(429) => TranscriptionError::RateLimited(message),
        Some(408) => TranscriptionError::Network(message),
        Some(code) if code >= 500 => TranscriptionError::Network(message),
        _ => TranscriptionError::Provider(message),
    }
}

fn map_transport_error(error: reqwest::Error) -> TranscriptionError {
    if error.is_timeout() || error.is_connect() {
        TranscriptionError::Network(error.to_string())
    } else {
        TranscriptionError::Provider(error.to_string())
    }
}

async fn map_http_error(response: reqwest::Response) -> TranscriptionError {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    let message = parse_chatgpt_error_message(&body)
        .unwrap_or_else(|| format!("ChatGPT request failed with status {}", status.as_u16()));

    match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
            TranscriptionError::Authentication(message)
        }
        StatusCode::TOO_MANY_REQUESTS => TranscriptionError::RateLimited(message),
        StatusCode::REQUEST_TIMEOUT => TranscriptionError::Network(message),
        _ if status.is_server_error() => TranscriptionError::Network(message),
        _ => TranscriptionError::Provider(message),
    }
}

fn parse_chatgpt_error_message(raw: &str) -> Option<String> {
    let value = serde_json::from_str::<Value>(raw).ok()?;

    if let Some(message) = value
        .get("error")
        .and_then(|error| {
            error
                .as_str()
                .map(ToString::to_string)
                .or_else(|| error.get("message")?.as_str().map(ToString::to_string))
        })
        .and_then(|message| normalize_optional_string(Some(message)))
    {
        return Some(message);
    }

    if let Some(message) = value
        .get("message")
        .and_then(|message| message.as_str())
        .and_then(|message| normalize_optional_string(Some(message.to_string())))
    {
        return Some(message);
    }

    normalize_optional_string(Some(truncate_response_body(raw.to_string())))
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|content| {
        let trimmed = content.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn truncate_response_body(value: String) -> String {
    let trimmed = value.trim();
    if trimmed.len() <= 300 {
        return trimmed.to_string();
    }

    format!("{}...", &trimmed[..300])
}

fn read_non_empty_env(name: &str) -> Option<String> {
    std::env::var(name).ok().and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn read_u64_env(name: &str) -> Option<u64> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .and_then(|value| value.parse::<u64>().ok())
}

fn build_client(config: &ChatGptTranscriptionConfig) -> Client {
    Client::builder()
        .timeout(Duration::from_secs(config.request_timeout_secs.max(1)))
        .build()
        .expect("ChatGPT client construction should succeed")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth_store::AuthStore;
    use mockito::{Matcher, Server};
    use std::fs;

    fn temp_app_data_dir(label: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "voice-chatgpt-transcription-tests-{label}-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&path)
            .expect("temp chatgpt transcription test directory should be created");
        path
    }

    fn provider_for_test(server: &Server, auth_store: AuthStore) -> ChatGptTranscriptionProvider {
        ChatGptTranscriptionProvider::new(
            ChatGptTranscriptionConfig {
                endpoint: format!("{}/backend-api/transcribe", server.url()),
                request_timeout_secs: 5,
            },
            auth_store,
        )
    }

    #[tokio::test]
    async fn sends_required_headers_and_base64_audio() {
        let mut server = Server::new_async().await;
        let app_data_dir = temp_app_data_dir("headers");
        let auth_store = AuthStore::new(app_data_dir);
        auth_store
            .save_chatgpt_login(
                "access-token",
                "refresh-token",
                now_epoch_seconds().saturating_add(600),
                "acct_123",
            )
            .expect("oauth credentials should persist");

        let mock = server
            .mock("POST", "/backend-api/transcribe")
            .match_header("authorization", "Bearer access-token")
            .match_header("chatgpt-account-id", "acct_123")
            .match_header("x-codex-base64", "1")
            .match_body(Matcher::Regex("AQID".to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"text":"  hello   world "}"#)
            .create_async()
            .await;

        let provider = provider_for_test(&server, auth_store);
        let result = provider
            .transcribe(vec![1, 2, 3], TranscriptionOptions::default())
            .await
            .expect("transcription should succeed");

        mock.assert_async().await;
        assert_eq!(result.text, "hello world");
    }

    #[tokio::test]
    async fn maps_unauthorized_errors() {
        let mut server = Server::new_async().await;
        let app_data_dir = temp_app_data_dir("auth-error");
        let auth_store = AuthStore::new(app_data_dir);
        auth_store
            .save_chatgpt_login(
                "bad-token",
                "refresh-token",
                now_epoch_seconds().saturating_add(600),
                "acct_123",
            )
            .expect("oauth credentials should persist");

        let mock = server
            .mock("POST", "/backend-api/transcribe")
            .with_status(401)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error":{"message":"Token invalid"}}"#)
            .create_async()
            .await;

        let provider = provider_for_test(&server, auth_store);
        let error = provider
            .transcribe(vec![1, 2, 3], TranscriptionOptions::default())
            .await
            .expect_err("request should fail");

        mock.assert_async().await;
        assert_eq!(
            error,
            TranscriptionError::Authentication("Token invalid".to_string())
        );
    }
}
