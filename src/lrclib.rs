use reqwest::blocking::Client;
use serde::Deserialize;

use crate::state::LyricLine;

#[derive(Deserialize)]
struct LrcLibResponse {
    #[serde(rename = "syncedLyrics")]
    synced_lyrics: Option<String>,
    #[serde(rename = "plainLyrics")]
    plain_lyrics: Option<String>,
}

pub fn fetch_lyrics(
    client: &Client,
    title: &str,
    artist: &str,
    album: &str,
    duration_ms: u64,
) -> (Vec<LyricLine>, Option<String>) {
    let duration_sec = (duration_ms / 1000).to_string();

    if let Ok(resp) = client
        .get("https://lrclib.net/api/get")
        .header("User-Agent", "SpotiFrame/0.1")
        .query(&[
            ("track_name", title),
            ("artist_name", artist),
            ("album_name", album),
            ("duration", duration_sec.as_str()),
        ])
        .send()
    {
        if resp.status().is_success() {
            if let Ok(data) = resp.json::<LrcLibResponse>() {
                if let Some(synced) = data.synced_lyrics {
                    let parsed = parse_lrc(&synced);
                    if !parsed.is_empty() {
                        return (parsed, data.plain_lyrics);
                    }
                }
                if data.plain_lyrics.is_some() {
                    return (Vec::new(), data.plain_lyrics);
                }
            }
        }
    }

    let query = format!("{title} {artist}");
    if let Ok(resp) = client
        .get("https://lrclib.net/api/search")
        .header("User-Agent", "SpotiFrame/0.1")
        .query(&[("q", query.as_str())])
        .send()
    {
        if resp.status().is_success() {
            if let Ok(results) = resp.json::<Vec<LrcLibResponse>>() {
                for item in &results {
                    if let Some(synced) = &item.synced_lyrics {
                        let parsed = parse_lrc(synced);
                        if !parsed.is_empty() {
                            return (parsed, item.plain_lyrics.clone());
                        }
                    }
                }
                if let Some(first_plain) = results.into_iter().find_map(|r| r.plain_lyrics) {
                    return (Vec::new(), Some(first_plain));
                }
            }
        }
    }

    (Vec::new(), None)
}

fn parse_lrc(lrc: &str) -> Vec<LyricLine> {
    let mut lines: Vec<LyricLine> = Vec::new();
    for raw in lrc.lines() {
        let raw = raw.trim();
        if !raw.starts_with('[') {
            continue;
        }
        let Some(end) = raw.find(']') else { continue };
        let timestamp = &raw[1..end];
        let text = raw[end + 1..].trim();

        let Some((min_str, sec_str)) = timestamp.split_once(':') else {
            continue;
        };
        let Ok(mins) = min_str.parse::<u64>() else {
            continue;
        };
        let Ok(secs) = sec_str.parse::<f64>() else {
            continue;
        };
        let time_ms = mins * 60_000 + (secs * 1000.0) as u64;

        // Render empty/instrumental timestamps as a music note like Spotifast
        let display_text = if text.is_empty() {
            "♪".to_string()
        } else {
            text.to_string()
        };

        // Avoid back-to-back duplicate music notes
        if display_text == "♪" && lines.last().map_or(false, |l| l.text == "♪") {
            continue;
        }

        lines.push(LyricLine {
            time_ms,
            text: display_text,
        });
    }
    lines.sort_by_key(|l| l.time_ms);
    lines
}