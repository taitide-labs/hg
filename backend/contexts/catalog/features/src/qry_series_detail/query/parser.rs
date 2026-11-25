use hg_backend_contracts::error::PortError;
use hg_contracts::catalog::{
    CommentReply, EpisodeComment, EpisodeSummary, PagedResult, SeriesDetail, SeriesStats,
    SeriesSummary,
};
use hg_contracts::ids::{CommentId, EpisodeNumber, PageInfo, PageRequest, SeriesId};
use serde::Deserialize;
use serde_json::Value;

#[derive(Default, Deserialize)]
struct RawSeriesCounts {
    #[serde(default, alias = "play_heat")]
    hot: Option<u64>,
    #[serde(default, alias = "likes")]
    like: Option<u64>,
    #[serde(default, alias = "bookmarks")]
    collect: Option<u64>,
    #[serde(default, alias = "comments")]
    comment: Option<u64>,
}

#[derive(Deserialize)]
struct RawEpisode {
    #[serde(alias = "ep")]
    ep_num: u32,
    #[serde(default)]
    title: Option<String>,
    #[serde(default, alias = "duration_secs")]
    duration: Option<u32>,
    #[serde(default)]
    playable: Option<bool>,
    #[serde(default)]
    status: Option<i64>,
}

#[derive(Deserialize)]
struct RawSeriesDetail {
    #[serde(default)]
    id: Option<u64>,
    title: String,
    #[serde(default, alias = "desc")]
    description: Option<String>,
    #[serde(default, alias = "cover_url")]
    cover: Option<String>,
    #[serde(default)]
    score: Option<f32>,
    #[serde(default)]
    is_finished: Option<bool>,
    #[serde(default)]
    episode_count: Option<u32>,
    #[serde(default)]
    total_episodes: Option<u32>,
    #[serde(default)]
    tags: Option<Vec<String>>,
    #[serde(default)]
    hot: Option<u64>,
    #[serde(default)]
    badge: Option<String>,
    #[serde(default)]
    is_original: Option<Value>,
    #[serde(default)]
    is_hot: Option<Value>,
    #[serde(default)]
    is_new: Option<Value>,
    #[serde(default)]
    new_at: Option<u64>,
    #[serde(default)]
    counts: Option<RawSeriesCounts>,
    #[serde(default)]
    episodes: Option<Vec<RawEpisode>>,
}

#[derive(Deserialize)]
struct RawCommentReply {
    id: u64,
    #[serde(default)]
    nickname: String,
    #[serde(default)]
    avatar_color: String,
    #[serde(default)]
    text: String,
    #[serde(default, alias = "relative_time")]
    time: String,
    #[serde(default)]
    like_count: Option<u32>,
    #[serde(default)]
    reply_to_nickname: Option<String>,
}

#[derive(Deserialize)]
struct RawEpisodeComment {
    id: u64,
    #[serde(default)]
    nickname: String,
    #[serde(default)]
    avatar_color: String,
    #[serde(default)]
    text: String,
    #[serde(default, alias = "relative_time")]
    time: String,
    #[serde(default)]
    like_count: Option<u32>,
    #[serde(default)]
    is_pinned: Option<bool>,
    #[serde(default)]
    ip_region: Option<String>,
    #[serde(default)]
    reply_count: Option<u32>,
    #[serde(default, alias = "preview_replies")]
    reply_list: Option<Vec<RawCommentReply>>,
}

#[derive(Default, Deserialize)]
struct RawPagination {
    #[serde(default)]
    page: Option<u32>,
    #[serde(default)]
    pages: Option<u32>,
    #[serde(default)]
    size: Option<u32>,
    #[serde(default)]
    total: Option<u64>,
}

#[derive(Deserialize)]
struct RawCommentsData {
    #[serde(default)]
    items: Option<Vec<RawEpisodeComment>>,
    #[serde(default)]
    pagination: Option<RawPagination>,
    #[serde(default)]
    total: Option<u64>,
}

#[derive(Deserialize)]
struct RawRepliesData {
    #[serde(default)]
    items: Option<Vec<RawCommentReply>>,
    #[serde(default)]
    pagination: Option<RawPagination>,
    #[serde(default)]
    total: Option<u64>,
}

pub fn parse_series_detail(
    requested_series_id: SeriesId,
    root: &Value,
) -> Result<SeriesDetail, PortError> {
    let data = extract_data_object(root, "剧目详情")?;
    let raw: RawSeriesDetail =
        serde_json::from_value(data.clone()).map_err(|err| PortError::UpstreamRejected {
            detail: format!("解析剧目详情失败: {err}"),
        })?;

    let series_id = match raw.id {
        Some(id) => SeriesId::try_from(id).map_err(|detail| PortError::UpstreamRejected {
            detail: format!("剧目标识非法: {detail}"),
        })?,
        None => requested_series_id,
    };

    let episodes = raw
        .episodes
        .unwrap_or_default()
        .into_iter()
        .map(map_episode)
        .collect::<Result<Vec<_>, _>>()?;

    let episode_len = u32::try_from(episodes.len()).unwrap_or(u32::MAX);
    let total_episodes = [raw.total_episodes, raw.episode_count, Some(episode_len)]
        .into_iter()
        .flatten()
        .max()
        .unwrap_or(episode_len);
    let episode_count = total_episodes;
    let counts = raw.counts.unwrap_or_default();
    let play_heat = counts.hot.or(raw.hot).unwrap_or(0);
    let hot = raw.hot.or(counts.hot).unwrap_or(0);
    let is_original =
        parse_flag(raw.is_original.as_ref()) || clean_optional_text(raw.badge).is_some();
    let is_hot = parse_flag(raw.is_hot.as_ref());
    let is_new = parse_flag(raw.is_new.as_ref()) || raw.new_at.unwrap_or(0) > 0;

    Ok(SeriesDetail {
        summary: SeriesSummary {
            id: series_id,
            title: raw.title,
            cover_url: raw.cover.unwrap_or_default(),
            score: raw.score.unwrap_or(0.0),
            tags: raw.tags.unwrap_or_default(),
            description: raw.description.unwrap_or_default(),
            episode_count,
            is_finished: raw.is_finished.unwrap_or(false),
            hot,
            is_original,
            is_hot,
            is_new,
        },
        total_episodes,
        stats: SeriesStats {
            play_heat,
            likes: counts.like.unwrap_or(0),
            bookmarks: counts.collect.unwrap_or(0),
            comments: counts.comment.unwrap_or(0),
        },
        episodes,
    })
}

pub fn parse_episode_comments(
    root: &Value,
    fallback_page: PageRequest,
) -> Result<PagedResult<EpisodeComment>, PortError> {
    let data = extract_data_object(root, "分集评论")?;
    let raw: RawCommentsData =
        serde_json::from_value(data.clone()).map_err(|err| PortError::UpstreamRejected {
            detail: format!("解析分集评论失败: {err}"),
        })?;

    let items = raw
        .items
        .unwrap_or_default()
        .into_iter()
        .map(map_comment)
        .collect::<Result<Vec<_>, _>>()?;
    let pagination = build_page_info(
        raw.pagination.as_ref(),
        raw.total,
        fallback_page,
        items.len(),
    );

    Ok(PagedResult { items, pagination })
}

pub fn parse_comment_replies(
    root: &Value,
    fallback_page: PageRequest,
) -> Result<PagedResult<CommentReply>, PortError> {
    let data = extract_data_object(root, "评论回复")?;
    let raw: RawRepliesData =
        serde_json::from_value(data.clone()).map_err(|err| PortError::UpstreamRejected {
            detail: format!("解析评论回复失败: {err}"),
        })?;

    let items = raw
        .items
        .unwrap_or_default()
        .into_iter()
        .map(map_reply)
        .collect::<Result<Vec<_>, _>>()?;
    let pagination = build_page_info(
        raw.pagination.as_ref(),
        raw.total,
        fallback_page,
        items.len(),
    );

    Ok(PagedResult { items, pagination })
}

fn extract_data_object<'a>(root: &'a Value, label: &str) -> Result<&'a Value, PortError> {
    root.get("data")
        .filter(|val| val.is_object())
        .ok_or_else(|| PortError::UpstreamRejected {
            detail: format!("{label}响应缺少有效 data 对象"),
        })
}

fn map_episode(raw: RawEpisode) -> Result<EpisodeSummary, PortError> {
    let ep_num =
        EpisodeNumber::try_from(raw.ep_num).map_err(|detail| PortError::UpstreamRejected {
            detail: format!("分集序号非法: {detail}"),
        })?;
    let title = clean_optional_text(raw.title).unwrap_or_else(|| format!("第{}集", ep_num.get()));
    let playable = raw.playable.unwrap_or_else(|| raw.status == Some(1));

    Ok(EpisodeSummary {
        ep_num,
        title,
        duration_secs: raw.duration.unwrap_or(0),
        playable,
    })
}

fn map_comment(raw: RawEpisodeComment) -> Result<EpisodeComment, PortError> {
    let id = CommentId::try_from(raw.id).map_err(|detail| PortError::UpstreamRejected {
        detail: format!("评论标识非法: {detail}"),
    })?;
    let preview_replies = raw
        .reply_list
        .unwrap_or_default()
        .into_iter()
        .map(map_reply)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(EpisodeComment {
        id,
        nickname: raw.nickname,
        avatar_color: raw.avatar_color,
        text: raw.text,
        relative_time: raw.time,
        like_count: raw.like_count.unwrap_or(0),
        is_pinned: raw.is_pinned.unwrap_or(false),
        ip_region: clean_optional_text(raw.ip_region),
        reply_count: raw.reply_count.unwrap_or(0),
        preview_replies,
    })
}

fn map_reply(raw: RawCommentReply) -> Result<CommentReply, PortError> {
    let id = CommentId::try_from(raw.id).map_err(|detail| PortError::UpstreamRejected {
        detail: format!("回复标识非法: {detail}"),
    })?;

    Ok(CommentReply {
        id,
        nickname: raw.nickname,
        avatar_color: raw.avatar_color,
        text: raw.text,
        relative_time: raw.time,
        like_count: raw.like_count.unwrap_or(0),
        reply_to_nickname: clean_optional_text(raw.reply_to_nickname),
    })
}

fn build_page_info(
    raw: Option<&RawPagination>,
    top_total: Option<u64>,
    fallback: PageRequest,
    item_count: usize,
) -> PageInfo {
    let page_opt = raw.and_then(|p| p.page);
    let pages_opt = raw.and_then(|p| p.pages);
    let size_opt = raw.and_then(|p| p.size);
    let total_opt = raw.and_then(|p| p.total);

    let size = size_opt.unwrap_or_else(|| u32::from(fallback.size.get()));
    let page = page_opt.unwrap_or_else(|| fallback.page.get());
    let total = total_opt
        .or(top_total)
        .unwrap_or_else(|| u64::try_from(item_count).unwrap_or(u64::MAX));
    let pages = pages_opt.unwrap_or_else(|| {
        if size == 0 {
            0
        } else {
            u32::try_from(total.div_ceil(u64::from(size))).unwrap_or(u32::MAX)
        }
    });

    PageInfo {
        page,
        pages,
        size,
        total,
    }
}

fn clean_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_owned())
        }
    })
}

fn parse_flag(value: Option<&Value>) -> bool {
    match value {
        Some(Value::Bool(flag)) => *flag,
        Some(Value::Number(num)) => {
            num.as_i64().is_some_and(|v| v != 0) || num.as_u64().is_some_and(|v| v != 0)
        }
        Some(Value::String(text)) => {
            let trimmed = text.trim();
            trimmed.eq_ignore_ascii_case("true") || trimmed == "1"
        }
        _ => false,
    }
}
