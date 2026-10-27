//! Tolerant parser for Innertube responses → `domain.rs` types.
//!
//! Instead of following fixed paths (which YouTube changes often), the known
//! *renderers* are searched for anywhere in the JSON and each element is
//! interpreted separately: one that isn't understood is skipped with a `debug!`
//! and never breaks the whole list (spike/FINDINGS.md §5).

use serde_json::Value;

use crate::domain::{
    AlbumDetail, AlbumRef, AlbumSummary, ArtistDetail, ArtistRef, ArtistSummary, BrowseId,
    LikeStatus, MediaItem, Page, PlaylistDetail, PlaylistId, PlaylistSummary, Section, Thumbnail,
    Track, TrackId,
};

// ------------------------------------------------------------------ utilities

/// Follows a path like `a.b.0.c`.
pub fn at<'a>(v: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.')
        .try_fold(v, |cur, key| match key.parse::<usize>() {
            Ok(i) => cur.get(i),
            Err(_) => cur.get(key),
        })
}

fn str_at<'a>(v: &'a Value, path: &str) -> Option<&'a str> {
    at(v, path).and_then(Value::as_str)
}

/// Text of a `{runs: [...]}` or `{simpleText}` object.
pub fn text(v: Option<&Value>) -> Option<String> {
    let v = v?;
    if let Some(s) = v.get("simpleText").and_then(Value::as_str) {
        return Some(s.to_owned());
    }
    let runs = v.get("runs")?.as_array()?;
    let s: String = runs
        .iter()
        .filter_map(|r| r.get("text").and_then(Value::as_str))
        .collect();
    (!s.is_empty()).then_some(s)
}

fn runs(v: Option<&Value>) -> &[Value] {
    v.and_then(|v| v.get("runs"))
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

/// All values under the key `key`, in document order. Does not descend
/// into a value that was already found.
pub fn find_all<'a>(v: &'a Value, key: &str) -> Vec<&'a Value> {
    fn walk<'a>(v: &'a Value, key: &str, out: &mut Vec<&'a Value>) {
        match v {
            Value::Object(m) => {
                for (k, x) in m {
                    if k == key {
                        out.push(x);
                    } else {
                        walk(x, key, out);
                    }
                }
            }
            Value::Array(a) => a.iter().for_each(|x| walk(x, key, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    walk(v, key, &mut out);
    out
}

pub fn find_first<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    match v {
        Value::Object(m) => m
            .get(key)
            .or_else(|| m.values().find_map(|x| find_first(x, key))),
        Value::Array(a) => a.iter().find_map(|x| find_first(x, key)),
        _ => None,
    }
}

/// First `thumbnails` list inside `v`, sorted from smallest to largest.
pub fn thumbnails(v: Option<&Value>) -> Vec<Thumbnail> {
    let Some(list) = v
        .and_then(|v| find_first(v, "thumbnails"))
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|t| {
            Some(Thumbnail {
                url: t.get("url")?.as_str()?.to_owned(),
                width: t.get("width").and_then(Value::as_u64).unwrap_or(0) as u32,
                height: t.get("height").and_then(Value::as_u64).unwrap_or(0) as u32,
            })
        })
        .collect()
}

fn page_type(endpoint: &Value) -> Option<&str> {
    str_at(
        endpoint,
        "browseEndpoint.browseEndpointContextSupportedConfigs.browseEndpointContextMusicConfig.pageType",
    )
}

fn browse_id(endpoint: &Value) -> Option<&str> {
    str_at(endpoint, "browseEndpoint.browseId")
}

/// "3:07" or "1:02:03" → milliseconds.
pub fn parse_duration(s: &str) -> Option<u32> {
    let parts: Vec<&str> = s.trim().split(':').collect();
    if !(2..=3).contains(&parts.len())
        || parts
            .iter()
            .any(|p| p.is_empty() || !p.chars().all(|c| c.is_ascii_digit()))
    {
        return None;
    }
    let secs = parts
        .iter()
        .try_fold(0u32, |acc, p| Some(acc * 60 + p.parse::<u32>().ok()?))?;
    Some(secs * 1000)
}

/// Type words YouTube puts in the secondary line that are not artists (English
/// and Spanish UI labels; the Spanish ones are matched on purpose).
fn is_type_word(s: &str) -> bool {
    matches!(
        s,
        "Song"
            | "Video"
            | "Episode"
            | "Single"
            | "EP"
            | "Album"
            | "Playlist"
            | "Artist"
            | "Profile"
            | "Podcast"
            | "Canción"
            | "Vídeo"
            | "Episodio"
            | "Sencillo"
            | "Álbum"
            | "Lista"
            | "Artista"
            | "Perfil"
    )
}

/// Metric labels such as "1B plays" (English and Spanish UI labels, matched on purpose).
fn is_metric(s: &str) -> bool {
    let l = s.to_ascii_lowercase();
    [
        "plays",
        "views",
        "monthly audience",
        "subscribers",
        "listeners",
        "reproducciones",
        "visualizaciones",
        "suscriptores",
        "oyentes",
    ]
    .iter()
    .any(|w| l.ends_with(w))
}

fn is_year(s: &str) -> bool {
    s.len() == 4 && s.chars().all(|c| c.is_ascii_digit())
}

/// Separators between runs ("y" is the Spanish "and", matched on purpose).
fn is_separator(s: &str) -> bool {
    matches!(s.trim(), "•" | "&" | "," | "" | "y" | "and")
}

fn badges_explicit(r: &Value) -> bool {
    find_all(r, "musicInlineBadgeRenderer")
        .iter()
        .any(|b| str_at(b, "icon.iconType") == Some("MUSIC_EXPLICIT_BADGE"))
}

fn like_status(r: &Value) -> Option<LikeStatus> {
    match find_first(r, "likeButtonRenderer")
        .and_then(|l| l.get("likeStatus"))
        .and_then(Value::as_str)?
    {
        "LIKE" => Some(LikeStatus::Like),
        "DISLIKE" => Some(LikeStatus::Dislike),
        _ => Some(LikeStatus::Indifferent),
    }
}

/// Artists, album and duration from runs (secondary line of a row).
#[derive(Default)]
struct Byline {
    artists: Vec<ArtistRef>,
    album: Option<AlbumRef>,
    duration_ms: Option<u32>,
}

fn parse_byline(all_runs: &[&Value]) -> Byline {
    let mut b = Byline::default();
    let mut unlinked: Vec<String> = Vec::new();
    for r in all_runs {
        let Some(t) = r.get("text").and_then(Value::as_str) else {
            continue;
        };
        let t = t.trim();
        if is_separator(t) {
            continue;
        }
        if let Some(ep) = r.get("navigationEndpoint") {
            match page_type(ep) {
                Some("MUSIC_PAGE_TYPE_ARTIST" | "MUSIC_PAGE_TYPE_USER_CHANNEL") => {
                    b.artists.push(ArtistRef {
                        id: browse_id(ep).map(|s| BrowseId(s.to_owned())),
                        name: t.to_owned(),
                    });
                    continue;
                }
                Some("MUSIC_PAGE_TYPE_ALBUM") => {
                    b.album = Some(AlbumRef {
                        id: browse_id(ep).map(|s| BrowseId(s.to_owned())),
                        name: t.to_owned(),
                    });
                    continue;
                }
                _ => {}
            }
        }
        if let Some(d) = parse_duration(t) {
            b.duration_ms = Some(d);
        } else if !is_type_word(t) && !is_metric(t) && !is_year(t) {
            unlinked.push(t.to_owned());
        }
    }
    // With no linked artists, the first loose text is the artist (e.g. videos).
    if b.artists.is_empty()
        && let Some(first) = unlinked.into_iter().next()
    {
        b.artists.push(ArtistRef {
            id: None,
            name: first,
        });
    }
    b
}

// ------------------------------------------------------------------ items

/// `musicResponsiveListItemRenderer` → track, or card if it links to a page.
pub fn list_item(r: &Value) -> Option<MediaItem> {
    let flex: Vec<&Value> = r
        .get("flexColumns")
        .and_then(Value::as_array)
        .map(|cols| {
            cols.iter()
                .filter_map(|c| c.get("musicResponsiveListItemFlexColumnRenderer"))
                .collect()
        })
        .unwrap_or_default();
    let title = text(flex.first().and_then(|c| c.get("text")))?;
    let thumbs = thumbnails(r.get("thumbnail"));

    let video_id = str_at(r, "playlistItemData.videoId")
        .or_else(|| {
            flex.first()
                .and_then(|c| str_at(c, "text.runs.0.navigationEndpoint.watchEndpoint.videoId"))
        })
        .or_else(|| {
            find_first(r, "playNavigationEndpoint").and_then(|e| str_at(e, "watchEndpoint.videoId"))
        });

    if let Some(video_id) = video_id {
        if str_at(r, "musicItemRendererDisplayPolicy")
            == Some("MUSIC_ITEM_RENDERER_DISPLAY_POLICY_GREY_OUT")
        {
            return None; // Unavailable.
        }
        let mut all: Vec<&Value> = flex
            .iter()
            .skip(1)
            .flat_map(|c| runs(c.get("text")))
            .collect();
        let fixed: Vec<&Value> = r
            .get("fixedColumns")
            .and_then(Value::as_array)
            .map(|cols| {
                cols.iter()
                    .flat_map(|c| runs(at(c, "musicResponsiveListItemFixedColumnRenderer.text")))
                    .collect()
            })
            .unwrap_or_default();
        all.extend(fixed);
        let b = parse_byline(&all);
        return Some(MediaItem::Track(Track {
            id: TrackId(video_id.to_owned()),
            title,
            artists: b.artists,
            album: b.album,
            duration_ms: b.duration_ms,
            thumbnails: thumbs,
            explicit: badges_explicit(r),
            like_status: like_status(r),
            set_video_id: str_at(r, "playlistItemData.playlistSetVideoId").map(str::to_owned),
        }));
    }

    let ep = r.get("navigationEndpoint")?;
    let id = browse_id(ep)?.to_owned();
    let subtitle = flex.get(1).and_then(|c| text(c.get("text")));
    card_from(
        page_type(ep),
        id,
        title,
        subtitle,
        runs(flex.get(1).and_then(|c| c.get("text"))),
        thumbs,
    )
}

fn card_from(
    page: Option<&str>,
    id: String,
    title: String,
    subtitle: Option<String>,
    sub_runs: &[Value],
    thumbs: Vec<Thumbnail>,
) -> Option<MediaItem> {
    let sub_refs: Vec<&Value> = sub_runs.iter().collect();
    match page {
        Some(
            "MUSIC_PAGE_TYPE_ARTIST"
            | "MUSIC_PAGE_TYPE_USER_CHANNEL"
            | "MUSIC_PAGE_TYPE_LIBRARY_ARTIST",
        ) => Some(MediaItem::Artist(ArtistSummary {
            id: BrowseId(id),
            name: title,
            subtitle,
            thumbnails: thumbs,
        })),
        Some("MUSIC_PAGE_TYPE_ALBUM" | "MUSIC_PAGE_TYPE_AUDIOBOOK") => {
            Some(MediaItem::Album(AlbumSummary {
                id: BrowseId(id),
                title,
                artists: parse_byline(&sub_refs).artists,
                subtitle,
                thumbnails: thumbs,
            }))
        }
        Some("MUSIC_PAGE_TYPE_PLAYLIST") => {
            let track_count = subtitle.as_deref().and_then(count_tracks);
            let author = sub_runs
                .iter()
                .filter_map(|r| r.get("text").and_then(Value::as_str))
                .map(str::trim)
                .find(|t| {
                    !is_separator(t)
                        && !is_type_word(t)
                        && !is_metric(t)
                        && count_tracks(t).is_none()
                })
                .map(str::to_owned);
            let id = if id.starts_with("VL") {
                id
            } else {
                format!("VL{id}")
            };
            Some(MediaItem::Playlist(PlaylistSummary {
                id: PlaylistId(id),
                title,
                author,
                track_count,
                thumbnails: thumbs,
            }))
        }
        other => {
            tracing::debug!(page = ?other, "card of unknown type; skipped");
            None
        }
    }
}

/// "41 tracks", "10 songs", "1,234 canciones" → number.
fn count_tracks(s: &str) -> Option<u32> {
    s.split('•').map(str::trim).find_map(|part| {
        let (num, word) = part.split_once(' ')?;
        let word = word.to_ascii_lowercase();
        if ["track", "song", "canci", "pista", "tema"]
            .iter()
            .any(|w| word.starts_with(w))
        {
            num.replace([',', '.'], "").parse().ok()
        } else {
            None
        }
    })
}

/// `musicTwoRowItemRenderer` → card (or track if it links to a video).
pub fn two_row_item(r: &Value) -> Option<MediaItem> {
    let title = text(r.get("title"))?;
    let thumbs = thumbnails(r.get("thumbnailRenderer"));
    let subtitle = text(r.get("subtitle"));
    let sub_runs = runs(r.get("subtitle"));
    let ep = r.get("navigationEndpoint")?;
    if let Some(video_id) = str_at(ep, "watchEndpoint.videoId") {
        let refs: Vec<&Value> = sub_runs.iter().collect();
        let b = parse_byline(&refs);
        return Some(MediaItem::Track(Track {
            id: TrackId(video_id.to_owned()),
            title,
            artists: b.artists,
            album: b.album,
            duration_ms: b.duration_ms,
            thumbnails: thumbs,
            explicit: badges_explicit(r),
            like_status: None,
            set_video_id: None,
        }));
    }
    let id = browse_id(ep)?.to_owned();
    card_from(page_type(ep), id, title, subtitle, sub_runs, thumbs)
}

/// `playlistPanelVideoRenderer` (radio queue / "up next").
pub fn panel_item(r: &Value) -> Option<Track> {
    let video_id = r.get("videoId").and_then(Value::as_str)?;
    if r.get("unplayableText").is_some() {
        return None;
    }
    let byline: Vec<&Value> = runs(r.get("longBylineText")).iter().collect();
    let b = parse_byline(&byline);
    Some(Track {
        id: TrackId(video_id.to_owned()),
        title: text(r.get("title"))?,
        artists: b.artists,
        album: b.album,
        duration_ms: text(r.get("lengthText"))
            .as_deref()
            .and_then(parse_duration),
        thumbnails: thumbnails(r.get("thumbnail")),
        explicit: badges_explicit(r),
        like_status: None,
        set_video_id: None,
    })
}

fn items_in(v: &Value) -> Vec<MediaItem> {
    // Walk in document order over the two row types.
    fn walk(v: &Value, out: &mut Vec<MediaItem>) {
        match v {
            Value::Object(m) => {
                for (k, x) in m {
                    match k.as_str() {
                        "musicResponsiveListItemRenderer" => out.extend(list_item(x)),
                        "musicTwoRowItemRenderer" => out.extend(two_row_item(x)),
                        _ => walk(x, out),
                    }
                }
            }
            Value::Array(a) => a.iter().for_each(|x| walk(x, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    walk(v, &mut out);
    out
}

fn tracks_only(items: Vec<MediaItem>) -> Vec<Track> {
    items
        .into_iter()
        .filter_map(|i| match i {
            MediaItem::Track(t) => Some(t),
            _ => None,
        })
        .collect()
}

// ------------------------------------------------------------------ continuations

/// Next-page token of the *list* (not of chips or menus).
/// Prefix `c:` = modern token (`continuation` body), `n:` = legacy (`ctoken`).
pub fn list_continuation(v: &Value) -> Option<String> {
    if let Some(t) = find_all(v, "continuationItemRenderer")
        .into_iter()
        .find_map(|c| str_at(c, "continuationEndpoint.continuationCommand.token"))
    {
        return Some(format!("c:{t}"));
    }
    for shelf in [
        "musicShelfRenderer",
        "musicPlaylistShelfRenderer",
        "musicShelfContinuation",
        "musicPlaylistShelfContinuation",
    ] {
        if let Some(t) = find_all(v, shelf)
            .into_iter()
            .find_map(|s| str_at(s, "continuations.0.nextContinuationData.continuation"))
        {
            return Some(format!("n:{t}"));
        }
    }
    None
}

/// Next page of sections (Home).
pub fn sections_continuation(v: &Value) -> Option<String> {
    find_all(v, "sectionListRenderer")
        .into_iter()
        .chain(find_all(v, "sectionListContinuation"))
        .find_map(|s| str_at(s, "continuations.0.nextContinuationData.continuation"))
        .map(|t| format!("n:{t}"))
}

// ------------------------------------------------------------------ pages

/// Track list of a playlist (or its continuation).
pub fn track_page(v: &Value) -> Page<Track> {
    Page {
        items: tracks_only(items_in_list_shelves(v)),
        continuation: list_continuation(v),
    }
}

/// Items of list shelves (not of suggestion carousels).
fn items_in_list_shelves(v: &Value) -> Vec<MediaItem> {
    let mut out = Vec::new();
    let shelves: Vec<&Value> = [
        "musicPlaylistShelfRenderer",
        "musicShelfRenderer",
        "musicPlaylistShelfContinuation",
        "musicShelfContinuation",
        "appendContinuationItemsAction",
    ]
    .iter()
    .flat_map(|k| find_all(v, k))
    .collect();
    if shelves.is_empty() {
        return items_in(v);
    }
    for s in shelves {
        out.extend(items_in(s));
    }
    out
}

pub fn playlist_page(v: &Value, id: &PlaylistId) -> PlaylistDetail {
    let header = find_first(v, "musicResponsiveHeaderRenderer")
        .or_else(|| find_first(v, "musicDetailHeaderRenderer"));
    let title = header
        .and_then(|h| text(h.get("title")))
        .unwrap_or_else(|| id.0.clone());
    let author = header.and_then(|h| {
        str_at(h, "facepile.avatarStackViewModel.text.content")
            .map(str::to_owned)
            .or_else(|| text(h.get("straplineTextOne")))
    });
    let stats = header.and_then(|h| text(h.get("secondSubtitle")));
    let description = header
        .and_then(|h| find_first(h, "musicDescriptionShelfRenderer"))
        .and_then(|d| text(d.get("description")))
        .or_else(|| header.and_then(|h| text(h.get("description"))));
    // Only the user's own playlists have the edit button.
    let owned = header.is_some_and(|h| find_first(h, "playlistEditorEndpoint").is_some());
    PlaylistDetail {
        summary: PlaylistSummary {
            id: id.clone(),
            title,
            author,
            track_count: stats.as_deref().and_then(count_tracks),
            thumbnails: thumbnails(header.and_then(|h| h.get("thumbnail"))),
        },
        description,
        stats,
        owned,
        tracks: track_page(v),
    }
}

pub fn album_page(v: &Value, id: &BrowseId) -> AlbumDetail {
    let header = find_first(v, "musicResponsiveHeaderRenderer")
        .or_else(|| find_first(v, "musicDetailHeaderRenderer"));
    let title = header
        .and_then(|h| text(h.get("title")))
        .unwrap_or_default();
    let artist_runs: Vec<&Value> = runs(header.and_then(|h| h.get("straplineTextOne")))
        .iter()
        .collect();
    let artists = parse_byline(&artist_runs).artists;
    let thumbs = thumbnails(header.and_then(|h| h.get("thumbnail")));
    let album_ref = AlbumRef {
        id: Some(id.clone()),
        name: title.clone(),
    };
    let mut tracks = tracks_only(items_in_list_shelves(v));
    // Album rows carry neither album nor cover art (and sometimes no artist either).
    for t in &mut tracks {
        t.album.get_or_insert_with(|| album_ref.clone());
        if t.thumbnails.is_empty() {
            t.thumbnails = thumbs.clone();
        }
        if t.artists.is_empty() {
            t.artists = artists.clone();
        }
    }
    let playlist_id = header
        .and_then(|h| find_first(h, "watchPlaylistEndpoint"))
        .and_then(|e| e.get("playlistId"))
        .and_then(Value::as_str)
        .map(|p| PlaylistId(p.to_owned()));
    AlbumDetail {
        summary: AlbumSummary {
            id: id.clone(),
            title,
            artists,
            subtitle: header.and_then(|h| text(h.get("subtitle"))),
            thumbnails: thumbs,
        },
        playlist_id,
        stats: header.and_then(|h| text(h.get("secondSubtitle"))),
        description: header
            .and_then(|h| find_first(h, "musicDescriptionShelfRenderer"))
            .and_then(|d| text(d.get("description"))),
        tracks,
    }
}

pub fn artist_page(v: &Value, id: &BrowseId) -> ArtistDetail {
    let header = find_first(v, "musicImmersiveHeaderRenderer")
        .or_else(|| find_first(v, "musicVisualHeaderRenderer"));
    let name = header
        .and_then(|h| text(h.get("title")))
        .unwrap_or_default();
    let radio_seed = header
        .and_then(|h| {
            str_at(
                h,
                "startRadioButton.buttonRenderer.navigationEndpoint.watchEndpoint.videoId",
            )
            .or_else(|| {
                str_at(
                    h,
                    "playButton.buttonRenderer.navigationEndpoint.watchEndpoint.videoId",
                )
            })
        })
        .map(|s| TrackId(s.to_owned()));
    let top_songs = find_all(v, "musicShelfRenderer")
        .first()
        .map(|s| tracks_only(items_in(s)))
        .unwrap_or_default();
    ArtistDetail {
        summary: ArtistSummary {
            id: id.clone(),
            name,
            subtitle: header.and_then(|h| text(h.get("monthlyListenerCount"))),
            thumbnails: thumbnails(header.and_then(|h| h.get("thumbnail"))),
        },
        description: header.and_then(|h| text(h.get("description"))),
        top_songs,
        sections: carousels(v),
        radio_seed,
    }
}

/// Carousels with a title (Home, artist…).
pub fn carousels(v: &Value) -> Vec<Section> {
    find_all(v, "musicCarouselShelfRenderer")
        .into_iter()
        .filter_map(|c| {
            let title = text(at(c, "header.musicCarouselShelfBasicHeaderRenderer.title"))
                .or_else(|| text(find_first(c.get("header")?, "title")))?;
            let items = items_in(c.get("contents")?);
            (!items.is_empty()).then_some(Section { title, items })
        })
        .collect()
}

pub fn home_page(v: &Value) -> Page<Section> {
    Page {
        items: carousels(v),
        continuation: sections_continuation(v),
    }
}

/// History grouped by period ("Today", "June 2025"…).
pub fn history_page(v: &Value) -> Vec<Section> {
    find_all(v, "musicShelfRenderer")
        .into_iter()
        .filter_map(|s| {
            let items = items_in(s.get("contents")?);
            (!items.is_empty()).then(|| Section {
                title: text(s.get("title")).unwrap_or_default(),
                items,
            })
        })
        .collect()
}

/// Grid (library playlists or albums).
pub fn grid_items(v: &Value) -> Vec<MediaItem> {
    find_all(v, "gridRenderer")
        .into_iter()
        .flat_map(|g| g.get("items").map(items_in).unwrap_or_default())
        .collect()
}

/// Search results: the top result first and then the rest, without duplicates.
pub fn search_page(v: &Value) -> Page<MediaItem> {
    let mut items: Vec<MediaItem> = Vec::new();
    for card in find_all(v, "musicCardShelfRenderer") {
        if let Some(top) = card_shelf_top(card) {
            items.push(top);
        }
    }
    for it in items_in_list_shelves(v).into_iter().chain(
        find_all(v, "itemSectionRenderer")
            .into_iter()
            .flat_map(items_in),
    ) {
        if !items.iter().any(|x| same_item(x, &it)) {
            items.push(it);
        }
    }
    Page {
        items,
        continuation: list_continuation(v),
    }
}

fn same_item(a: &MediaItem, b: &MediaItem) -> bool {
    match (a, b) {
        (MediaItem::Track(x), MediaItem::Track(y)) => x.id == y.id,
        (MediaItem::Album(x), MediaItem::Album(y)) => x.id == y.id,
        (MediaItem::Artist(x), MediaItem::Artist(y)) => x.id == y.id,
        (MediaItem::Playlist(x), MediaItem::Playlist(y)) => x.id == y.id,
        _ => false,
    }
}

/// Top result of a search.
fn card_shelf_top(card: &Value) -> Option<MediaItem> {
    let title_run = at(card, "title.runs.0")?;
    let title = title_run.get("text")?.as_str()?.to_owned();
    let ep = title_run.get("navigationEndpoint")?;
    let thumbs = thumbnails(card.get("thumbnail"));
    let sub = runs(card.get("subtitle"));
    if let Some(video_id) = str_at(ep, "watchEndpoint.videoId") {
        let refs: Vec<&Value> = sub.iter().collect();
        let b = parse_byline(&refs);
        return Some(MediaItem::Track(Track {
            id: TrackId(video_id.to_owned()),
            title,
            artists: b.artists,
            album: b.album,
            duration_ms: b.duration_ms,
            thumbnails: thumbs,
            explicit: badges_explicit(card),
            like_status: None,
            set_video_id: None,
        }));
    }
    card_from(
        page_type(ep),
        browse_id(ep)?.to_owned(),
        title,
        text(card.get("subtitle")),
        sub,
        thumbs,
    )
}

/// Radio queue from `next` (and its continuation).
pub fn radio_page(v: &Value) -> Page<Track> {
    // A `playlistPanelVideoWrapperRenderer` row carries the song (`primaryRenderer`)
    // and its music video (`counterpart`): only the first one belongs in the queue.
    fn walk<'a>(v: &'a Value, out: &mut Vec<&'a Value>) {
        match v {
            Value::Object(m) => {
                for (k, x) in m {
                    match k.as_str() {
                        "playlistPanelVideoRenderer" => out.push(x),
                        "counterpart" => {}
                        _ => walk(x, out),
                    }
                }
            }
            Value::Array(a) => a.iter().for_each(|x| walk(x, out)),
            _ => {}
        }
    }
    let mut rows = Vec::new();
    walk(v, &mut rows);
    let mut seen = std::collections::HashSet::new();
    let tracks: Vec<Track> = rows
        .into_iter()
        .filter_map(panel_item)
        .filter(|t| seen.insert(t.id.0.clone()))
        .collect();
    let continuation = find_all(v, "nextRadioContinuationData")
        .into_iter()
        .chain(find_all(v, "nextContinuationData"))
        .find_map(|c| c.get("continuation").and_then(Value::as_str))
        .map(str::to_owned);
    Page {
        items: tracks,
        continuation,
    }
}

pub fn suggestions(v: &Value) -> Vec<String> {
    find_all(v, "searchSuggestionRenderer")
        .into_iter()
        .chain(find_all(v, "historySuggestionRenderer"))
        .filter_map(|s| text(s.get("suggestion")))
        .collect()
}

/// Name, handle and photo of the active account (`account/account_menu`).
pub fn account_info(v: &Value) -> Option<(String, Option<String>, Vec<Thumbnail>)> {
    let h = find_first(v, "activeAccountHeaderRenderer")?;
    Some((
        text(h.get("accountName"))?,
        text(h.get("channelHandle")),
        thumbnails(h.get("accountPhoto")),
    ))
}

/// Data from the `player` endpoint: tracking URL (history) and loudness.
pub struct PlayerInfo {
    pub tracking_url: Option<String>,
    pub loudness_db: Option<f32>,
}

pub fn player_info(v: &Value) -> PlayerInfo {
    PlayerInfo {
        tracking_url: str_at(v, "playbackTracking.videostatsPlaybackUrl.baseUrl")
            .map(str::to_owned),
        loudness_db: at(v, "playerConfig.audioConfig.loudnessDb")
            .and_then(Value::as_f64)
            .map(|x| x as f32),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn artist_run(name: &str, id: &str) -> Value {
        json!({ "text": name, "navigationEndpoint": { "browseEndpoint": { "browseId": id,
            "browseEndpointContextSupportedConfigs": { "browseEndpointContextMusicConfig": { "pageType": "MUSIC_PAGE_TYPE_ARTIST" } } } } })
    }

    fn album_run(name: &str, id: &str) -> Value {
        json!({ "text": name, "navigationEndpoint": { "browseEndpoint": { "browseId": id,
            "browseEndpointContextSupportedConfigs": { "browseEndpointContextMusicConfig": { "pageType": "MUSIC_PAGE_TYPE_ALBUM" } } } } })
    }

    /// Playlist row with the real YouTube Music shape (October 2026).
    fn playlist_row(video: &str, title: &str) -> Value {
        json!({ "musicResponsiveListItemRenderer": {
            "thumbnail": { "musicThumbnailRenderer": { "thumbnail": { "thumbnails": [ { "url": "https://lh3.googleusercontent.com/x=w60-h60", "width": 60, "height": 60 } ] } } },
            "flexColumns": [
                { "musicResponsiveListItemFlexColumnRenderer": { "text": { "runs": [ { "text": title, "navigationEndpoint": { "watchEndpoint": { "videoId": video } } } ] } } },
                { "musicResponsiveListItemFlexColumnRenderer": { "text": { "runs": [ artist_run("Aventura", "UCDaQK4nrsvz27YzufrhJfgg") ] } } },
                { "musicResponsiveListItemFlexColumnRenderer": { "text": { "runs": [ album_run("We Broke The Rules", "MPREb_WixYTw0z5H2") ] } } }
            ],
            "fixedColumns": [ { "musicResponsiveListItemFixedColumnRenderer": { "text": { "runs": [ { "text": "4:15" } ] } } } ],
            "playlistItemData": { "playlistSetVideoId": "56B44F6D10557CC6", "videoId": video },
            "badges": [ { "musicInlineBadgeRenderer": { "icon": { "iconType": "MUSIC_EXPLICIT_BADGE" } } } ],
            "menu": { "menuRenderer": { "topLevelButtons": [ { "likeButtonRenderer": { "likeStatus": "LIKE" } } ] } }
        } })
    }

    #[test]
    fn complete_playlist_row() {
        let MediaItem::Track(t) =
            list_item(&playlist_row("yQd4xQBlBa0", "Obsesion")["musicResponsiveListItemRenderer"])
                .unwrap()
        else {
            panic!("should be a track")
        };
        assert_eq!(t.id.0, "yQd4xQBlBa0");
        assert_eq!(t.title, "Obsesion");
        assert_eq!(t.artists[0].name, "Aventura");
        assert_eq!(
            t.artists[0].id.as_ref().unwrap().0,
            "UCDaQK4nrsvz27YzufrhJfgg"
        );
        assert_eq!(t.album.as_ref().unwrap().name, "We Broke The Rules");
        assert_eq!(t.duration_ms, Some(255_000));
        assert_eq!(t.set_video_id.as_deref(), Some("56B44F6D10557CC6"));
        assert_eq!(t.like_status, Some(LikeStatus::Like));
        assert!(t.explicit);
        assert_eq!(t.thumbnails.len(), 1);
    }

    #[test]
    fn search_row_with_type_and_metrics() {
        // "Song • Aventura" | "1.2B plays": with no links, the loose text is the artist.
        let row = json!({
            "flexColumns": [
                { "musicResponsiveListItemFlexColumnRenderer": { "text": { "runs": [ { "text": "El Perdedor", "navigationEndpoint": { "watchEndpoint": { "videoId": "abcdefghijk" } } } ] } } },
                { "musicResponsiveListItemFlexColumnRenderer": { "text": { "runs": [ { "text": "Song" }, { "text": " • " }, { "text": "Aventura" }, { "text": " • " }, { "text": "3:45" } ] } } },
                { "musicResponsiveListItemFlexColumnRenderer": { "text": { "runs": [ { "text": "1B plays" } ] } } }
            ]
        });
        let Some(MediaItem::Track(t)) = list_item(&row) else {
            panic!()
        };
        assert_eq!(t.artists.len(), 1);
        assert_eq!(t.artists[0].name, "Aventura");
        assert_eq!(t.duration_ms, Some(225_000));
    }

    #[test]
    fn unavailable_row_is_skipped_and_list_continues() {
        let mut bad = playlist_row("zzzzzzzzzzz", "Deleted");
        bad["musicResponsiveListItemRenderer"]["musicItemRendererDisplayPolicy"] =
            json!("MUSIC_ITEM_RENDERER_DISPLAY_POLICY_GREY_OUT");
        let broken =
            json!({ "musicResponsiveListItemRenderer": { "flexColumns": "this is not a list" } });
        let page = json!({ "contents": { "musicPlaylistShelfRenderer": { "contents": [
            playlist_row("aaaaaaaaaaa", "One"), bad, broken, playlist_row("bbbbbbbbbbb", "Two"),
            { "continuationItemRenderer": { "continuationEndpoint": { "continuationCommand": { "token": "TOKEN" } } } }
        ] } } });
        let p = track_page(&page);
        assert_eq!(
            p.items.iter().map(|t| t.title.as_str()).collect::<Vec<_>>(),
            ["One", "Two"]
        );
        assert_eq!(p.continuation.as_deref(), Some("c:TOKEN"));
    }

    #[test]
    fn legacy_shelf_continuation() {
        let page = json!({ "musicShelfRenderer": { "contents": [], "continuations": [ { "nextContinuationData": { "continuation": "OLD" } } ] },
                           "header": { "chipCloudRenderer": { "continuation": "NOT-THIS-ONE" } } });
        assert_eq!(list_continuation(&page).as_deref(), Some("n:OLD"));
    }

    #[test]
    fn two_row_cards() {
        let album = json!({
            "title": { "runs": [ { "text": "The Last" } ] },
            "subtitle": { "runs": [ { "text": "Album" }, { "text": " • " }, artist_run("Aventura", "UC1"), { "text": " • " }, { "text": "2009" } ] },
            "navigationEndpoint": { "browseEndpoint": { "browseId": "MPREb_x", "browseEndpointContextSupportedConfigs": { "browseEndpointContextMusicConfig": { "pageType": "MUSIC_PAGE_TYPE_ALBUM" } } } },
            "thumbnailRenderer": { "musicThumbnailRenderer": { "thumbnail": { "thumbnails": [ { "url": "u", "width": 226, "height": 226 } ] } } }
        });
        let Some(MediaItem::Album(a)) = two_row_item(&album) else {
            panic!()
        };
        assert_eq!(a.title, "The Last");
        assert_eq!(a.artists[0].name, "Aventura");

        let playlist = json!({
            "title": { "runs": [ { "text": "alternatives" } ] },
            "subtitle": { "runs": [ { "text": "Alex Rivera" }, { "text": " • " }, { "text": "10 tracks" } ] },
            "navigationEndpoint": { "browseEndpoint": { "browseId": "VLPLxyz", "browseEndpointContextSupportedConfigs": { "browseEndpointContextMusicConfig": { "pageType": "MUSIC_PAGE_TYPE_PLAYLIST" } } } }
        });
        let Some(MediaItem::Playlist(p)) = two_row_item(&playlist) else {
            panic!()
        };
        assert_eq!(p.id.0, "VLPLxyz");
        assert_eq!(p.author.as_deref(), Some("Alex Rivera"));
        assert_eq!(p.track_count, Some(10));

        let new_playlist_button = json!({ "title": { "runs": [ { "text": "New playlist" } ] }, "navigationEndpoint": { "createPlaylistEndpoint": {} } });
        assert!(two_row_item(&new_playlist_button).is_none());

        let video = json!({ "title": { "runs": [ { "text": "Obsesion" } ] }, "subtitle": { "runs": [ artist_run("Aventura", "UC1"), { "text": " • " }, { "text": "1B views" } ] },
                            "navigationEndpoint": { "watchEndpoint": { "videoId": "Lg_Pn45gyMs" } } });
        let Some(MediaItem::Track(v)) = two_row_item(&video) else {
            panic!()
        };
        assert_eq!(v.artists[0].name, "Aventura");
    }

    #[test]
    fn radio_panel() {
        let r = json!({ "videoId": "yQd4xQBlBa0", "title": { "runs": [ { "text": "Obsesion" } ] }, "lengthText": { "runs": [ { "text": "4:15" } ] },
                        "longBylineText": { "runs": [ artist_run("Aventura", "UC1"), { "text": " • " }, album_run("We Broke The Rules", "MPREb_1"), { "text": " • " }, { "text": "2002" } ] } });
        let t = panel_item(&r).unwrap();
        assert_eq!(t.duration_ms, Some(255_000));
        assert_eq!(t.album.unwrap().name, "We Broke The Rules");
        assert_eq!(t.artists.len(), 1);
    }

    #[test]
    fn radio_skips_music_video_counterparts() {
        let row = |id: &str, title: &str| json!({ "videoId": id, "title": { "runs": [ { "text": title } ] } });
        let v = json!({ "contents": [
            { "playlistPanelVideoRenderer": row("seed", "Seed") },
            { "playlistPanelVideoWrapperRenderer": {
                "primaryRenderer": { "playlistPanelVideoRenderer": row("u4mzqZg1i20", "Chi-Partner") },
                "counterpart": [ { "counterpartRenderer": { "playlistPanelVideoRenderer": row("lSXi7k72X_g", "Chi-Partner") } } ]
            } },
            { "playlistPanelVideoRenderer": row("seed", "Seed") }
        ] });
        let ids: Vec<String> = radio_page(&v).items.into_iter().map(|t| t.id.0).collect();
        assert_eq!(ids, ["seed", "u4mzqZg1i20"]);
    }

    #[test]
    fn durations_and_counts() {
        assert_eq!(parse_duration("1:02:03"), Some(3_723_000));
        assert_eq!(parse_duration("2002"), None);
        assert_eq!(parse_duration("a:bc"), None);
        assert_eq!(count_tracks("103 views • 41 tracks • 2 hours"), Some(41));
        assert_eq!(count_tracks("1,234 canciones"), Some(1234));
        assert_eq!(count_tracks("Playlist • 2023"), None);
    }

    /// Against real responses dumped locally (they are not in the repo).
    #[test]
    #[ignore]
    fn parses_real_dumps() {
        let dir = dirs::home_dir().unwrap().join(".xp-music").join("dumps");
        let load = |n: &str| -> Value {
            serde_json::from_str(&std::fs::read_to_string(dir.join(format!("{n}.json"))).unwrap())
                .unwrap()
        };

        let pl = playlist_page(
            &load("playlist_bachata"),
            &PlaylistId("VLPLEcMXldRXRhKU8AfZOALMpD6Fz9G_oE9p".into()),
        );
        println!(
            "playlist: {} · {:?} · {} tracks · owned={}",
            pl.summary.title,
            pl.summary.author,
            pl.tracks.items.len(),
            pl.owned
        );
        assert_eq!(pl.tracks.items.len(), 41);
        assert!(
            pl.tracks
                .items
                .iter()
                .all(|t| !t.artists.is_empty() && t.duration_ms.is_some())
        );

        let liked = track_page(&load("liked_songs"));
        println!("liked: {}", liked.items.len());
        assert!(!liked.items.is_empty());

        let album = album_page(&load("album"), &BrowseId("MPREb_WixYTw0z5H2".into()));
        println!(
            "album: {} · {:?} · {} tracks · playlist {:?}",
            album.summary.title,
            album.summary.artists.first().map(|a| &a.name),
            album.tracks.len(),
            album.playlist_id
        );
        assert_eq!(album.tracks.len(), 10);
        assert!(
            album
                .tracks
                .iter()
                .all(|t| t.album.is_some() && !t.thumbnails.is_empty())
        );

        let artist = artist_page(
            &load("artist"),
            &BrowseId("UCDaQK4nrsvz27YzufrhJfgg".into()),
        );
        println!(
            "artist: {} · top {} · sections {:?} · radio {:?}",
            artist.summary.name,
            artist.top_songs.len(),
            artist
                .sections
                .iter()
                .map(|s| format!("{} ({})", s.title, s.items.len()))
                .collect::<Vec<_>>(),
            artist.radio_seed
        );
        assert!(!artist.top_songs.is_empty() && !artist.sections.is_empty());

        let home = home_page(&load("home"));
        println!(
            "home: {:?} · more: {}",
            home.items
                .iter()
                .map(|s| format!("{} ({})", s.title, s.items.len()))
                .collect::<Vec<_>>(),
            home.continuation.is_some()
        );
        assert!(!home.items.is_empty());

        let hist = history_page(&load("history"));
        println!(
            "history: {:?}",
            hist.iter()
                .map(|s| format!("{} ({})", s.title, s.items.len()))
                .collect::<Vec<_>>()
        );
        assert!(hist.iter().map(|s| s.items.len()).sum::<usize>() > 100);

        let lib = grid_items(&load("library_playlists"));
        println!("library playlists: {}", lib.len());
        assert!(lib.len() >= 6);

        let artists: Vec<_> = items_in(&load("library_artists"));
        println!("library artists: {}", artists.len());

        let search = search_page(&load("search"));
        println!(
            "search: {} results, first {:?}",
            search.items.len(),
            search.items.first().map(|i| match i {
                MediaItem::Track(t) => t.title.clone(),
                MediaItem::Album(a) => a.title.clone(),
                MediaItem::Artist(a) => a.name.clone(),
                MediaItem::Playlist(p) => p.title.clone(),
            })
        );
        assert!(search.items.len() > 10);
        let songs = search_page(&load("search_songs"));
        println!(
            "song search: {} · more: {}",
            songs.items.len(),
            songs.continuation.is_some()
        );

        let radio = radio_page(&load("radio"));
        println!(
            "radio: {} tracks · more: {}",
            radio.items.len(),
            radio.continuation.is_some()
        );
        assert!(radio.items.len() >= 40);

        println!("suggestions: {:?}", suggestions(&load("suggestions")));
        let acc = account_info(&load("account_menu"));
        println!(
            "account: {:?}",
            acc.as_ref().map(|a| (&a.0, &a.1, a.2.len()))
        );
        let p = player_info(&load("player"));
        println!(
            "player: tracking={} loudness={:?}",
            p.tracking_url.is_some(),
            p.loudness_db
        );
    }
}
