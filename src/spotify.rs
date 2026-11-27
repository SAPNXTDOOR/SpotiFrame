use std::fs;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::distributions::Alphanumeric;
use rand::Rng;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tiny_http::{Response, Server};
use url::Url;

use crate::lrclib::fetch_lyrics;
use crate::state::{AppStatus, TrackState};

const REDIRECT_URI: &str = "http://127.0.0.1:8989/callback";
const SESSION_FILE: &str = ".spotilyrics_session.json";

pub enum PlayerCommand {
    Seek(u64),
    Previous,
    TogglePlayPause(bool),
    Next,
    CycleRepeat(String),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SavedSession {
    pub client_id: String,
    pub refresh_token: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
    refresh_token: Option<String>,
}

#[derive(Deserialize)]
struct PlayerStateResponse {
    is_playing: bool,
    progress_ms: Option<u64>,
    #[serde(default)]
    repeat_state: String,
    item: Option<SpotifyTrack>,
}

#[derive(Deserialize)]
struct SpotifyTrack {
    id: String,
    name: String,
    duration_ms: u64,
    artists: Vec<SpotifyArtist>,
    album: SpotifyAlbum,
}

#[derive(Deserialize)]
struct SpotifyArtist {
    name: String,
}

#[derive(Deserialize)]
struct SpotifyAlbum {
    name: String,
    images: Vec<SpotifyImage>,
}

#[derive(Deserialize)]
struct SpotifyImage {
    url: String,
}

pub fn load_session() -> Option<SavedSession> {
    let data = fs::read_to_string(SESSION_FILE).ok()?;
    serde_json::from_str(&data).ok()
}

fn save_session(session: &SavedSession) {
    if let Ok(json) = serde_json::to_string_pretty(session) {
        let _ = fs::write(SESSION_FILE, json);
    }
}

pub fn authenticate_pkce(client_id: &str) -> Result<(String, String, Instant), String> {
    let verifier: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect();

    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));

    let auth_url = Url::parse_with_params(
        "https://accounts.spotify.com/authorize",
        &[
            ("client_id", client_id),
            ("response_type", "code"),
            ("redirect_uri", REDIRECT_URI),
            (
                "scope",
                "user-read-playback-state user-modify-playback-state",
            ),
            ("code_challenge_method", "S256"),
            ("code_challenge", challenge.as_str()),
        ],
    )
    .map_err(|e| e.to_string())?;

    let server = Server::http("127.0.0.1:8989")
        .map_err(|e| format!("Could not start local callback server on port 8989: {e}"))?;

    open::that(auth_url.as_str()).map_err(|e| format!("Could not open browser: {e}"))?;

    let request = server
        .recv()
        .map_err(|e| format!("Failed waiting for Spotify callback: {e}"))?;

    let full_url = format!("http://127.0.0.1:8989{}", request.url());
    let parsed = Url::parse(&full_url).map_err(|e| e.to_string())?;

    let code = parsed
        .query_pairs()
        .find(|(k, _)| k == "code")
        .map(|(_, v)| v.to_string());

    let html = if code.is_some() {
        "<html><body style='font-family:sans-serif;background:#111;color:#fff;padding:40px;'><h2>Signed in!</h2><p>You can close this tab and return to SpotiFrame.</p></body></html>"
    } else {
        "<html><body style='font-family:sans-serif;background:#111;color:#f66;padding:40px;'><h2>Sign-in cancelled or failed.</h2></body></html>"
    };
    let _ = request.respond(Response::from_string(html).with_header(
        tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..])
            .unwrap(),
    ));

    let code = code.ok_or_else(|| "No authorization code received from Spotify.".to_string())?;

    let client = Client::new();
    let token_resp: TokenResponse = client
        .post("https://accounts.spotify.com/api/token")
        .form(&[
            ("client_id", client_id),
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("redirect_uri", REDIRECT_URI),
            ("code_verifier", verifier.as_str()),
        ])
        .send()
        .map_err(|e| format!("Token request failed: {e}"))?
        .error_for_status()
        .map_err(|e| format!("Spotify rejected token exchange: {e}"))?
        .json()
        .map_err(|e| format!("Invalid token JSON: {e}"))?;

    let refresh_token = token_resp
        .refresh_token
        .ok_or_else(|| "Spotify did not return a refresh token.".to_string())?;

    save_session(&SavedSession {
        client_id: client_id.to_string(),
        refresh_token: refresh_token.clone(),
    });

    let expires_at = Instant::now() + Duration::from_secs(token_resp.expires_in.saturating_sub(60));
    Ok((token_resp.access_token, refresh_token, expires_at))
}

pub fn refresh_access_token(
    client: &Client,
    client_id: &str,
    refresh_token: &str,
) -> Result<(String, String, Instant), String> {
    let token_resp: TokenResponse = client
        .post("https://accounts.spotify.com/api/token")
        .form(&[
            ("client_id", client_id),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
        ])
        .send()
        .map_err(|e| format!("Refresh request failed: {e}"))?
        .error_for_status()
        .map_err(|e| format!("Token refresh rejected: {e}"))?
        .json()
        .map_err(|e| format!("Invalid refresh JSON: {e}"))?;

    let new_refresh = token_resp
        .refresh_token
        .unwrap_or_else(|| refresh_token.to_string());

    save_session(&SavedSession {
        client_id: client_id.to_string(),
        refresh_token: new_refresh.clone(),
    });

    let expires_at = Instant::now() + Duration::from_secs(token_resp.expires_in.saturating_sub(60));
    Ok((token_resp.access_token, new_refresh, expires_at))
}

pub fn spawn_poller(
    client_id: String,
    mut access_token: String,
    mut refresh_token: String,
    mut expires_at: Instant,
    shared: Arc<Mutex<AppStatus>>,
    cmd_rx: Receiver<PlayerCommand>,
    ctx: egui::Context,
) {
    std::thread::spawn(move || {
        let client = Client::new();
        let mut last_track_id = String::new();
        let mut last_poll = Instant::now() - Duration::from_secs(10);

        loop {
            while let Ok(cmd) = cmd_rx.try_recv() {
                match cmd {
                    PlayerCommand::Seek(pos_ms) => {
                        let _ = client
                            .put("https://api.spotify.com/v1/me/player/seek")
                            .bearer_auth(&access_token)
                            .query(&[("position_ms", pos_ms.to_string())])
                            .header("Content-Length", "0")
                            .send();

                        if let AppStatus::Connected {
                            track: Some(ref mut t),
                            ..
                        } = *shared.lock().unwrap()
                        {
                            t.progress_ms = pos_ms;
                            t.synced_at = Instant::now();
                        }
                    }
                    PlayerCommand::Previous => {
                        let _ = client
                            .post("https://api.spotify.com/v1/me/player/previous")
                            .bearer_auth(&access_token)
                            .header("Content-Length", "0")
                            .send();
                        last_poll = Instant::now() - Duration::from_secs(10);
                    }
                    PlayerCommand::Next => {
                        let _ = client
                            .post("https://api.spotify.com/v1/me/player/next")
                            .bearer_auth(&access_token)
                            .header("Content-Length", "0")
                            .send();
                        last_poll = Instant::now() - Duration::from_secs(10);
                    }
                    PlayerCommand::TogglePlayPause(currently_playing) => {
                        let url = if currently_playing {
                            "https://api.spotify.com/v1/me/player/pause"
                        } else {
                            "https://api.spotify.com/v1/me/player/play"
                        };
                        let _ = client
                            .put(url)
                            .bearer_auth(&access_token)
                            .header("Content-Length", "0")
                            .send();

                        if let AppStatus::Connected {
                            track: Some(ref mut t),
                            ..
                        } = *shared.lock().unwrap()
                        {
                            t.progress_ms = t.current_position_ms();
                            t.synced_at = Instant::now();
                            t.is_playing = !currently_playing;
                        }
                    }
                    PlayerCommand::CycleRepeat(current) => {
                        let next_state = match current.as_str() {
                            "off" => "context",
                            "context" => "track",
                            _ => "off",
                        };
                        let _ = client
                            .put("https://api.spotify.com/v1/me/player/repeat")
                            .bearer_auth(&access_token)
                            .query(&[("state", next_state)])
                            .header("Content-Length", "0")
                            .send();

                        if let AppStatus::Connected {
                            track: Some(ref mut t),
                            ..
                        } = *shared.lock().unwrap()
                        {
                            t.repeat_state = next_state.to_string();
                        }
                    }
                }
                ctx.request_repaint();
            }

            if last_poll.elapsed() < Duration::from_millis(1500) {
                std::thread::sleep(Duration::from_millis(60));
                continue;
            }
            last_poll = Instant::now();

            if Instant::now() >= expires_at {
                match refresh_access_token(&client, &client_id, &refresh_token) {
                    Ok((at, rt, exp)) => {
                        access_token = at;
                        refresh_token = rt;
                        expires_at = exp;
                    }
                    Err(e) => {
                        let mut st = shared.lock().unwrap();
                        *st = AppStatus::Connected {
                            track: None,
                            error: Some(e),
                        };
                        ctx.request_repaint();
                        continue;
                    }
                }
            }

            let resp = client
                .get("https://api.spotify.com/v1/me/player")
                .bearer_auth(&access_token)
                .send();

            match resp {
                Ok(r) if r.status().as_u16() == 204 => {
                    let mut st = shared.lock().unwrap();
                    *st = AppStatus::Connected {
                        track: None,
                        error: None,
                    };
                }
                Ok(r) if r.status().as_u16() == 401 => {
                    expires_at = Instant::now();
                }
                Ok(r) if r.status().is_success() => {
                    if let Ok(cp) = r.json::<PlayerStateResponse>() {
                        if let Some(item) = cp.item {
                            let progress_ms = cp.progress_ms.unwrap_or(0);
                            let repeat = if cp.repeat_state.is_empty() {
                                "off".to_string()
                            } else {
                                cp.repeat_state
                            };

                            if item.id != last_track_id {
                                last_track_id = item.id.clone();
                                let artist = item
                                    .artists
                                    .iter()
                                    .map(|a| a.name.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", ");

                                let cover_bytes = item.album.images.first().and_then(|img| {
                                    client
                                        .get(&img.url)
                                        .send()
                                        .ok()?
                                        .bytes()
                                        .ok()
                                        .map(|b| b.to_vec())
                                });

                                let (lyrics, plain_lyrics) = fetch_lyrics(
                                    &client,
                                    &item.name,
                                    &artist,
                                    &item.album.name,
                                    item.duration_ms,
                                );

                                let mut st = shared.lock().unwrap();
                                *st = AppStatus::Connected {
                                    track: Some(TrackState {
                                        track_id: item.id,
                                        title: item.name,
                                        artist,
                                        album: item.album.name,
                                        duration_ms: item.duration_ms,
                                        progress_ms,
                                        synced_at: Instant::now(),
                                        is_playing: cp.is_playing,
                                        repeat_state: repeat,
                                        cover_bytes,
                                        lyrics,
                                        plain_lyrics,
                                    }),
                                    error: None,
                                };
                            } else {
                                let mut st = shared.lock().unwrap();
                                if let AppStatus::Connected {
                                    track: Some(ref mut t),
                                    ..
                                } = *st
                                {
                                    t.progress_ms = progress_ms;
                                    t.synced_at = Instant::now();
                                    t.is_playing = cp.is_playing;
                                    t.repeat_state = repeat;
                                }
                            }
                        }
                    }
                }
                _ => {}
            }

            ctx.request_repaint();
        }
    });
}