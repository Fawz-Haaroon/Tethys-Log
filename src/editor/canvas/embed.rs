// URL and embed-HTML recognition for inline media cards.
//
// ── What can be pasted ────────────────────────────────────────────────────────
//   YouTube watch / share / embed URL or <iframe>
//   Instagram reel/post URL or <blockquote class="instagram-media">
//   TikTok video URL
//   Vimeo video URL
//   Twitter / X status URL
//   Pinterest pin URL or assets.pinterest.com/ext/embed.html iframe src
//   Rumble video URL
//   Dailymotion video URL
//   Twitch clip URL
//   Reddit video URL (v.redd.it or reddit.com/r/*/comments/*)
//   Bilibili video URL
//   Streamable / Streamff clip URL
//   Niconico video URL
//   SoundCloud (audio — yt-dlp handles it)
//   ANY <iframe> whose src domain is on the known-video-host list
//   A raw URL on the known list not caught by the above (Generic fallback)
//
// ── On-disk format ────────────────────────────────────────────────────────────
//   \u{E001}embed:<watch_url>\u{E001}
//   Only the watch_url is persisted; platform branding is re-derived at load.
//
//   EMBED_OPEN lives in the Unicode Private Use Area rather than a C0
//   control character. Notes saved before this change used \x01 (SOH) --
//   see EMBED_OPEN_LEGACY, which codec::deserialise_into_buffer still
//   recognises on read so those notes keep working. The reason for the
//   change: a NUL, SOH, or STX byte anywhere in a file is exactly what
//   makes `file`, `git`, `less`, and GitHub's own viewer decide the file is
//   binary instead of text -- which is what a .tlog note built before this
//   fix looks like to every tool except Tethys-Log itself.

pub const EMBED_OPEN:        char = '\u{E001}';
pub const EMBED_OPEN_LEGACY: char = '\x01';
pub const EMBED_TAG:         &str = "embed:";

#[derive(Clone, Copy)]
pub struct PlatformInfo {
    pub name:   &'static str,
    pub accent: &'static str, // CSS hex colour for the card left-stripe
}

pub enum EmbedKind {
    YouTube { embed_src: String, watch_url: String },
    Generic { watch_url: String, platform: PlatformInfo },
}

// ── Platform constants ────────────────────────────────────────────────────────
pub const YOUTUBE:    PlatformInfo = PlatformInfo { name: "YouTube",      accent: "#c8312a" };
pub const INSTAGRAM:  PlatformInfo = PlatformInfo { name: "Instagram",    accent: "#c13584" };
pub const TIKTOK:     PlatformInfo = PlatformInfo { name: "TikTok",       accent: "#69c9d0" };
pub const VIMEO:      PlatformInfo = PlatformInfo { name: "Vimeo",        accent: "#1ab7ea" };
pub const TWITTER:    PlatformInfo = PlatformInfo { name: "Twitter / X",  accent: "#1da1f2" };
pub const PINTEREST:  PlatformInfo = PlatformInfo { name: "Pinterest",    accent: "#e60023" };
pub const RUMBLE:     PlatformInfo = PlatformInfo { name: "Rumble",       accent: "#85c742" };
pub const DAILYMOTION:PlatformInfo = PlatformInfo { name: "Dailymotion",  accent: "#0066dc" };
pub const TWITCH:     PlatformInfo = PlatformInfo { name: "Twitch",       accent: "#9146ff" };
pub const REDDIT:     PlatformInfo = PlatformInfo { name: "Reddit",       accent: "#ff4500" };
pub const BILIBILI:   PlatformInfo = PlatformInfo { name: "Bilibili",     accent: "#fb7299" };
pub const NICONICO:   PlatformInfo = PlatformInfo { name: "Niconico",     accent: "#252525" };
pub const SOUNDCLOUD: PlatformInfo = PlatformInfo { name: "SoundCloud",   accent: "#ff5500" };
pub const GENERIC:    PlatformInfo = PlatformInfo { name: "Video",        accent: "#5a7a9a" };

// ── Main classification entry point ───────────────────────────────────────────
pub fn classify_url(raw: &str) -> Option<EmbedKind> {
    let raw = raw.trim();

    // ── Instagram embed blockquote HTML ──────────────────────────────────────
    if raw.contains("instagram-media") || raw.contains("data-instgrm-permalink") {
        if let Some(url) = extract_attr(raw, "data-instgrm-permalink") {
            return Some(EmbedKind::Generic {
                watch_url: strip_query(url).to_string(),
                platform: INSTAGRAM,
            });
        }
    }

    // ── Any <iframe> — extract src, classify or fall back to known-host check
    if raw.to_ascii_lowercase().starts_with("<iframe") {
        if let Some(src) = extract_iframe_src(raw) {
            // Try full classify first (handles YouTube iframes, etc.)
            if let Some(kind) = classify_bare_url(src) {
                return Some(kind);
            }
            // Pinterest embed iframe: assets.pinterest.com/ext/embed.html?id=...
            if src.contains("assets.pinterest.com") || src.contains("pinterest.com/ext/embed") {
                if let Some(id) = query_param(src, "id") {
                    return Some(EmbedKind::Generic {
                        watch_url: format!("https://www.pinterest.com/pin/{id}/"),
                        platform: PINTEREST,
                    });
                }
            }
            // Any iframe from a known video host → generic embed via yt-dlp
            if is_known_video_host(src) {
                return Some(EmbedKind::Generic {
                    watch_url: src.to_string(),
                    platform: platform_for_url(src),
                });
            }
            // Universal fallback: any iframe with a valid http/https src gets
            // an embed card — yt-dlp tries to download it; if that fails the
            // card shows "Unavailable" with the Open button still working.
            if src.starts_with("https://") || src.starts_with("http://") {
                return Some(EmbedKind::Generic {
                    watch_url: src.to_string(),
                    platform: platform_for_url(src),
                });
            }
        }
        return None;
    }

    // ── Plain URL — classify directly ────────────────────────────────────────
    classify_bare_url(raw)
}

// Classify a plain URL (no HTML markup).
fn classify_bare_url(url: &str) -> Option<EmbedKind> {
    // YouTube
    if let Some(id) = youtube_watch_id(url) {
        return Some(EmbedKind::YouTube {
            embed_src: youtube_embed_src(id),
            watch_url: youtube_watch_url(id),
        });
    }
    if let Some(id) = youtu_be_id(url) {
        return Some(EmbedKind::YouTube {
            embed_src: youtube_embed_src(id),
            watch_url: youtube_watch_url(id),
        });
    }
    if url.contains("youtube.com/embed/") {
        if let Some(id) = youtube_id_from_embed(url) {
            return Some(EmbedKind::YouTube {
                embed_src: youtube_embed_src(id),
                watch_url: youtube_watch_url(id),
            });
        }
    }

    // Instagram
    if url.contains("instagram.com/reel/") || url.contains("instagram.com/p/") {
        return Some(generic(strip_query(url), INSTAGRAM));
    }

    // TikTok
    if url.contains("tiktok.com") && (url.contains("/video/") || url.contains("/@")) {
        return Some(generic(strip_query(url), TIKTOK));
    }

    // Vimeo
    if url.contains("vimeo.com/") && !url.contains("player.vimeo.com") {
        return Some(generic(strip_query(url), VIMEO));
    }
    if url.contains("player.vimeo.com/video/") {
        // e.g. https://player.vimeo.com/video/ID
        if let Some(after) = url.split("player.vimeo.com/video/").nth(1) {
            let id = after.split('?').next().unwrap_or(after);
            return Some(generic(&format!("https://vimeo.com/{id}"), VIMEO));
        }
    }

    // Twitter / X
    if (url.contains("twitter.com") || url.contains("x.com")) && url.contains("/status/") {
        return Some(generic(strip_query(url), TWITTER));
    }

    // Pinterest
    if url.contains("pinterest.com/pin/") || url.contains("pin.it/") {
        return Some(generic(strip_query(url), PINTEREST));
    }
    if url.contains("assets.pinterest.com") {
        if let Some(id) = query_param(url, "id") {
            return Some(generic(
                &format!("https://www.pinterest.com/pin/{id}/"),
                PINTEREST,
            ));
        }
    }

    // Rumble
    if url.contains("rumble.com/embed/") || url.contains("rumble.com/v") {
        return Some(generic(strip_query(url), RUMBLE));
    }

    // Dailymotion
    if url.contains("dailymotion.com/video/") || url.contains("dai.ly/") {
        return Some(generic(strip_query(url), DAILYMOTION));
    }

    // Twitch
    if url.contains("twitch.tv/") && url.contains("/clip/") {
        return Some(generic(strip_query(url), TWITCH));
    }
    if url.contains("clips.twitch.tv/") {
        return Some(generic(strip_query(url), TWITCH));
    }

    // Reddit video
    if url.contains("v.redd.it/") || (url.contains("reddit.com/") && url.contains("/comments/")) {
        return Some(generic(strip_query(url), REDDIT));
    }

    // Bilibili
    if url.contains("bilibili.com/video/") || url.contains("b23.tv/") {
        return Some(generic(strip_query(url), BILIBILI));
    }

    // Niconico
    if url.contains("nicovideo.jp/watch/") || url.contains("nico.ms/") {
        return Some(generic(strip_query(url), NICONICO));
    }

    // SoundCloud
    if url.contains("soundcloud.com/") {
        return Some(generic(strip_query(url), SOUNDCLOUD));
    }

    // Streamable / Streamff
    if url.contains("streamable.com/") || url.contains("streamff.com/") {
        return Some(generic(strip_query(url), GENERIC));
    }

    None
}

fn generic(url: &str, platform: PlatformInfo) -> EmbedKind {
    EmbedKind::Generic { watch_url: url.to_string(), platform }
}

// ── Domain whitelist for unknown iframes ─────────────────────────────────────
fn is_known_video_host(url: &str) -> bool {
    const HOSTS: &[&str] = &[
        "youtube.com", "youtu.be",
        "instagram.com",
        "tiktok.com",
        "vimeo.com",
        "twitter.com", "x.com",
        "pinterest.com", "pin.it", "assets.pinterest.com",
        "rumble.com",
        "dailymotion.com", "dai.ly",
        "twitch.tv", "clips.twitch.tv",
        "reddit.com", "v.redd.it",
        "bilibili.com", "b23.tv",
        "nicovideo.jp", "nico.ms",
        "soundcloud.com",
        "streamable.com", "streamff.com",
        "gfycat.com",
    ];
    HOSTS.iter().any(|h| url.contains(h))
}

// ── Derive platform branding from URL ────────────────────────────────────────
pub fn platform_for_url(url: &str) -> PlatformInfo {
    if url.contains("youtube.com") || url.contains("youtu.be") { YOUTUBE }
    else if url.contains("instagram.com")                       { INSTAGRAM }
    else if url.contains("tiktok.com")                         { TIKTOK }
    else if url.contains("vimeo.com")                          { VIMEO }
    else if url.contains("twitter.com") || url.contains("x.com") { TWITTER }
    else if url.contains("pinterest.com") || url.contains("pin.it") { PINTEREST }
    else if url.contains("rumble.com")                         { RUMBLE }
    else if url.contains("dailymotion.com") || url.contains("dai.ly") { DAILYMOTION }
    else if url.contains("twitch.tv")                          { TWITCH }
    else if url.contains("reddit.com") || url.contains("v.redd.it") { REDDIT }
    else if url.contains("bilibili.com") || url.contains("b23.tv") { BILIBILI }
    else if url.contains("nicovideo.jp") || url.contains("nico.ms") { NICONICO }
    else if url.contains("soundcloud.com")                     { SOUNDCLOUD }
    else                                                       { GENERIC }
}

// ── Disk codec helpers ────────────────────────────────────────────────────────

pub fn watch_url_from_embed_src(embed_src: &str) -> String {
    // Old YouTube /embed/ID URLs stored from previous versions
    if let Some(id) = youtube_id_from_embed(embed_src) {
        return youtube_watch_url(id);
    }
    embed_src.to_string()
}

pub fn embed_marker(watch_url: &str) -> String {
    format!("{}{}{}{}", EMBED_OPEN, EMBED_TAG, watch_url, EMBED_OPEN)
}

pub fn parse_embed_tag(tag_content: &str) -> Option<&str> {
    tag_content.strip_prefix(EMBED_TAG)
}

// ── HTML attribute / URL helpers ─────────────────────────────────────────────

fn extract_iframe_src(html: &str) -> Option<&str> {
    // handle src="..." and src='...'
    for (dq, sq) in [("src=\"", '"'), ("src='", '\'')] {
        if let Some(after) = html.split(dq).nth(1) {
            return Some(after.split(sq).next().unwrap_or(after));
        }
    }
    None
}

fn extract_attr<'a>(html: &'a str, attr: &str) -> Option<&'a str> {
    let dq = format!("{}=\"", attr);
    let sq = format!("{}='", attr);
    if let Some(after) = html.split(dq.as_str()).nth(1) {
        return Some(after.split('"').next().unwrap_or(after));
    }
    if let Some(after) = html.split(sq.as_str()).nth(1) {
        return Some(after.split('\'').next().unwrap_or(after));
    }
    None
}

fn query_param<'a>(url: &'a str, key: &str) -> Option<&'a str> {
    let search = format!("{}=", key);
    url.split('?').nth(1)?
        .split('&')
        .find(|p| p.starts_with(search.as_str()))
        .map(|p| p[key.len() + 1..].split('&').next().unwrap_or(&p[key.len() + 1..]))
}

fn strip_query(url: &str) -> &str {
    url.split('?').next().unwrap_or(url).split('#').next().unwrap_or(url)
}

// ── YouTube helpers ───────────────────────────────────────────────────────────
fn youtube_watch_id(url: &str) -> Option<&str> {
    if !url.contains("youtube.com/watch") { return None; }
    url.split('?').nth(1)?
        .split('&')
        .find(|p| p.starts_with("v="))
        .map(|p| &p[2..])
}

fn youtu_be_id(url: &str) -> Option<&str> {
    let path = url.strip_prefix("https://youtu.be/")
        .or_else(|| url.strip_prefix("http://youtu.be/"))?;
    Some(path.split('?').next().unwrap_or(path))
}

fn youtube_id_from_embed(url: &str) -> Option<&str> {
    let after = url.split("youtube.com/embed/").nth(1)?;
    Some(after.split('?').next().unwrap_or(after))
}

fn youtube_embed_src(id: &str) -> String {
    format!("https://www.youtube.com/embed/{}?autoplay=1", id)
}

fn youtube_watch_url(id: &str) -> String {
    format!("https://www.youtube.com/watch?v={}", id)
}
