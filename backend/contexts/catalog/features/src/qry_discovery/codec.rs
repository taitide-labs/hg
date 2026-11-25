use hg_contracts::catalog::{
    Category, CategoryRecommendation, HomeBanner, HomeFeed, HotKeyword, HotTag, PagedResult,
    RankingEntry, RankingKind, SearchSuggestion, SeriesSummary, TagGroup, TagItem,
};
use hg_contracts::error::ContractError;
use hg_contracts::ids::{PageInfo, PageRequest, SeriesId};
use serde::Deserialize;
use serde::de::Deserializer;
use serde_json::{Map, Value};

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum FlexBool {
    Bool(bool),
    Int(i64),
    Text(String),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum FlexU64 {
    Int(u64),
    Float(f64),
    Text(String),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum FlexF32 {
    Float(f32),
    Text(String),
}

fn de_bool<'de, D: Deserializer<'de>>(deserializer: D) -> Result<bool, D::Error> {
    match Option::<FlexBool>::deserialize(deserializer)? {
        None => Ok(false),
        Some(FlexBool::Bool(flag)) => Ok(flag),
        Some(FlexBool::Int(num)) => Ok(num != 0),
        Some(FlexBool::Text(text)) => {
            let normalized = text.trim();
            Ok(normalized.eq_ignore_ascii_case("true") || normalized == "1")
        }
    }
}

fn parse_flex_u64(val: FlexU64) -> Option<u64> {
    match val {
        FlexU64::Int(num) => Some(num),
        FlexU64::Float(num) if num.is_finite() && num >= 0.0 => {
            let rounded = num.round();
            if rounded <= 9_007_199_254_740_991.0 {
                format!("{rounded:.0}").parse::<u64>().ok()
            } else {
                None
            }
        }
        FlexU64::Float(_) => None,
        FlexU64::Text(text) => text.trim().parse::<u64>().ok(),
    }
}

fn de_u64_opt<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<u64>, D::Error> {
    Ok(Option::<FlexU64>::deserialize(deserializer)?.and_then(parse_flex_u64))
}

fn de_u64<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
    Ok(de_u64_opt(deserializer)?.unwrap_or(0))
}

fn de_u32_opt<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<u32>, D::Error> {
    Ok(de_u64_opt(deserializer)?.and_then(|v| u32::try_from(v).ok()))
}

fn de_u32<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u32, D::Error> {
    Ok(de_u32_opt(deserializer)?.unwrap_or(0))
}

fn de_f32<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f32, D::Error> {
    match Option::<FlexF32>::deserialize(deserializer)? {
        None => Ok(0.0),
        Some(FlexF32::Float(num)) => Ok(if num.is_finite() { num } else { 0.0 }),
        Some(FlexF32::Text(text)) => Ok(text
            .trim()
            .parse::<f32>()
            .ok()
            .filter(|v| v.is_finite())
            .unwrap_or(0.0)),
    }
}

fn parse_series_id(raw: Option<u64>, context: &str) -> Result<SeriesId, ContractError> {
    let id = raw.ok_or_else(|| ContractError::UpstreamRejected {
        detail: format!("{context} 缺少剧目标识"),
    })?;
    SeriesId::try_from(id).map_err(|detail| ContractError::UpstreamRejected {
        detail: format!("{context} 剧目标识非法 ({id}): {detail}"),
    })
}

fn extract_map_error_message(map: &Map<String, Value>) -> Option<String> {
    ["detail", "msg", "error", "title"]
        .into_iter()
        .find_map(|key| {
            map.get(key)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
        })
}

pub fn extract_data(envelope: Value, context: &str) -> Result<Value, ContractError> {
    let Value::Object(mut obj) = envelope else {
        return Err(ContractError::UpstreamRejected {
            detail: format!("{context} 响应格式非 JSON 对象"),
        });
    };

    if let Some(status) = obj.get("status").and_then(Value::as_i64)
        && status != 1
    {
        let detail = extract_map_error_message(&obj)
            .unwrap_or_else(|| format!("{context} 上游业务拒绝，状态码 {status}"));
        return Err(ContractError::UpstreamRejected { detail });
    }

    obj.remove("data")
        .filter(|val| !val.is_null())
        .ok_or_else(|| ContractError::UpstreamRejected {
            detail: format!("{context} 响应缺少 data 字段"),
        })
}

fn decode_data<T: serde::de::DeserializeOwned>(
    data: Value,
    context: &str,
) -> Result<T, ContractError> {
    serde_json::from_value(data).map_err(|err| ContractError::UpstreamRejected {
        detail: format!("{context} 响应解析失败: {err}"),
    })
}

#[derive(Debug, Deserialize)]
struct RawVideoItem {
    #[serde(default, deserialize_with = "de_u64_opt")]
    id: Option<u64>,
    #[serde(default, deserialize_with = "de_u64_opt")]
    video_id: Option<u64>,
    #[serde(default, deserialize_with = "de_u64_opt")]
    series_id: Option<u64>,
    #[serde(default)]
    title: String,
    #[serde(default, alias = "cover_url", alias = "image")]
    cover: String,
    #[serde(default, deserialize_with = "de_f32")]
    score: f32,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default, alias = "desc")]
    description: String,
    #[serde(default, deserialize_with = "de_u32_opt")]
    episode_count: Option<u32>,
    #[serde(default, deserialize_with = "de_u32_opt")]
    total_episodes: Option<u32>,
    #[serde(default, deserialize_with = "de_bool")]
    is_finished: bool,
    #[serde(default, deserialize_with = "de_u64")]
    hot: u64,
    #[serde(default)]
    badge: Option<String>,
    #[serde(default, deserialize_with = "de_bool")]
    is_original: bool,
    #[serde(default, deserialize_with = "de_bool")]
    is_hot: bool,
    #[serde(default, deserialize_with = "de_bool")]
    is_new: bool,
    #[serde(default, deserialize_with = "de_u64")]
    new_at: u64,
}

fn resolve_original_flag(is_original: bool, badge: Option<&str>) -> bool {
    is_original || badge.is_some_and(|raw| !raw.trim().is_empty())
}

fn resolve_new_flag(is_new: bool, new_at: u64) -> bool {
    is_new || new_at > 0
}

fn resolve_episode_count(episode_count: Option<u32>, total_episodes: Option<u32>) -> u32 {
    episode_count
        .filter(|&n| n > 0)
        .or_else(|| total_episodes.filter(|&n| n > 0))
        .unwrap_or(0)
}

fn normalize_cover_url(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.starts_with("//") {
        return format!("https:{trimmed}");
    }
    hg_infra::edge_crypto::resolve_cover_url(trimmed)
}

impl RawVideoItem {
    fn into_summary(self, context: &str) -> Result<SeriesSummary, ContractError> {
        let id = parse_series_id(self.id.or(self.video_id).or(self.series_id), context)?;
        let is_original = resolve_original_flag(self.is_original, self.badge.as_deref());
        let is_new = resolve_new_flag(self.is_new, self.new_at);
        Ok(SeriesSummary {
            id,
            title: self.title.trim().to_owned(),
            cover_url: normalize_cover_url(&self.cover),
            score: self.score,
            tags: self.tags,
            description: self.description.trim().to_owned(),
            episode_count: resolve_episode_count(self.episode_count, self.total_episodes),
            is_finished: self.is_finished,
            hot: self.hot,
            is_original,
            is_hot: self.is_hot,
            is_new,
        })
    }
}

#[derive(Debug, Deserialize)]
struct RawHomeBanner {
    #[serde(default, deserialize_with = "de_u64_opt")]
    video_id: Option<u64>,
    #[serde(default, deserialize_with = "de_u64_opt")]
    series_id: Option<u64>,
    #[serde(default, deserialize_with = "de_u64_opt")]
    id: Option<u64>,
    #[serde(default)]
    title: String,
    #[serde(default, alias = "image_url", alias = "cover")]
    image: String,
    #[serde(default, deserialize_with = "de_f32")]
    score: f32,
    #[serde(default, alias = "description")]
    desc: String,
}

#[derive(Debug, Deserialize)]
struct RawCategoryRec {
    #[serde(
        default,
        alias = "category_id",
        alias = "id",
        deserialize_with = "de_u32"
    )]
    cate_id: u32,
    #[serde(default, alias = "category_name", alias = "name")]
    cate_name: String,
    #[serde(default, alias = "items")]
    videos: Vec<RawVideoItem>,
}

#[derive(Debug, Deserialize)]
struct RawHomeData {
    #[serde(default)]
    banners: Vec<RawHomeBanner>,
    #[serde(default)]
    featured: Vec<RawVideoItem>,
    #[serde(default, alias = "category_recommendations")]
    cate_rec: Vec<RawCategoryRec>,
}

pub fn parse_home_feed(data: Value) -> Result<HomeFeed, ContractError> {
    let raw: RawHomeData = decode_data(data, "首页聚合")?;
    let banners = raw
        .banners
        .into_iter()
        .map(|banner| {
            Ok(HomeBanner {
                series_id: parse_series_id(
                    banner.video_id.or(banner.series_id).or(banner.id),
                    "首页横幅",
                )?,
                title: banner.title.trim().to_owned(),
                image_url: normalize_cover_url(&banner.image),
                score: banner.score,
                description: banner.desc.trim().to_owned(),
            })
        })
        .collect::<Result<Vec<_>, ContractError>>()?;

    let featured = raw
        .featured
        .into_iter()
        .map(|item| item.into_summary("首页推荐"))
        .collect::<Result<Vec<_>, ContractError>>()?;

    let category_recommendations = raw
        .cate_rec
        .into_iter()
        .map(|rec| {
            let items = rec
                .videos
                .into_iter()
                .map(|item| item.into_summary("分类推荐"))
                .collect::<Result<Vec<_>, ContractError>>()?;
            Ok(CategoryRecommendation {
                category_id: rec.cate_id,
                category_name: rec.cate_name.trim().to_owned(),
                items,
            })
        })
        .collect::<Result<Vec<_>, ContractError>>()?;

    Ok(HomeFeed {
        banners,
        featured,
        latest: Vec::new(),
        category_recommendations,
    })
}

pub fn parse_home_latest(data: Value) -> Result<Vec<SeriesSummary>, ContractError> {
    let raw: RawPagedVideosData = decode_data(data, "首页最近更新")?;
    raw.items
        .into_iter()
        .map(|item| item.into_summary("首页最近更新"))
        .collect()
}

#[derive(Debug, Deserialize)]
struct RawCategoryItem {
    #[serde(default, deserialize_with = "de_u32")]
    id: u32,
    #[serde(default)]
    name: String,
    #[serde(default)]
    route: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawCategoriesData {
    Wrapped { items: Vec<RawCategoryItem> },
    List(Vec<RawCategoryItem>),
}

pub fn parse_categories(data: Value) -> Result<Vec<Category>, ContractError> {
    let items = match decode_data::<RawCategoriesData>(data, "频道分类")? {
        RawCategoriesData::Wrapped { items } | RawCategoriesData::List(items) => items,
    };

    Ok(items
        .into_iter()
        .map(|item| Category {
            id: item.id,
            name: item.name.trim().to_owned(),
            route: item.route.trim().to_owned(),
        })
        .collect())
}

#[derive(Debug, Deserialize)]
struct RawTagItem {
    #[serde(default, deserialize_with = "de_u32")]
    id: u32,
    #[serde(default)]
    name: String,
    #[serde(default)]
    slug: String,
    #[serde(default, deserialize_with = "de_bool")]
    is_hot: bool,
}

#[derive(Debug, Deserialize)]
struct RawTagGroup {
    #[serde(default, deserialize_with = "de_u32")]
    id: u32,
    #[serde(default)]
    code: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    tags: Vec<RawTagItem>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawTagGroupsData {
    Wrapped {
        #[serde(alias = "items")]
        categories: Vec<RawTagGroup>,
    },
    List(Vec<RawTagGroup>),
}

pub fn parse_tag_groups(data: Value) -> Result<Vec<TagGroup>, ContractError> {
    let groups = match decode_data::<RawTagGroupsData>(data, "分组标签")? {
        RawTagGroupsData::Wrapped { categories } | RawTagGroupsData::List(categories) => categories,
    };

    Ok(groups
        .into_iter()
        .map(|group| TagGroup {
            id: group.id,
            code: group.code.trim().to_owned(),
            name: group.name.trim().to_owned(),
            tags: group
                .tags
                .into_iter()
                .map(|tag| TagItem {
                    id: tag.id,
                    name: tag.name.trim().to_owned(),
                    slug: tag.slug.trim().to_owned(),
                    is_hot: tag.is_hot,
                })
                .collect(),
        })
        .collect())
}

#[derive(Debug, Deserialize)]
struct RawPagination {
    #[serde(default, deserialize_with = "de_u32")]
    page: u32,
    #[serde(default, deserialize_with = "de_u32")]
    pages: u32,
    #[serde(default, deserialize_with = "de_u32")]
    size: u32,
    #[serde(default, deserialize_with = "de_u64")]
    total: u64,
}

#[derive(Debug, Deserialize)]
struct RawPagedVideosData {
    #[serde(default)]
    items: Vec<RawVideoItem>,
    #[serde(default)]
    pagination: Option<RawPagination>,
    #[serde(default, deserialize_with = "de_u64_opt")]
    total: Option<u64>,
}

fn build_page_info(
    pagination: Option<RawPagination>,
    total_override: Option<u64>,
    request_page: PageRequest,
    item_count: usize,
) -> PageInfo {
    if let Some(p) = pagination {
        return PageInfo {
            page: p.page,
            pages: p.pages,
            size: p.size,
            total: p.total,
        };
    }

    let size = u32::from(request_page.size.get());
    let total = total_override.unwrap_or_else(|| u64::try_from(item_count).unwrap_or(u64::MAX));
    let pages = if total == 0 {
        0
    } else {
        u32::try_from(total.div_ceil(u64::from(size))).unwrap_or(u32::MAX)
    };
    PageInfo {
        page: request_page.page.get(),
        pages,
        size,
        total,
    }
}

pub fn parse_paged_series(
    data: Value,
    request_page: PageRequest,
    context: &str,
) -> Result<PagedResult<SeriesSummary>, ContractError> {
    let raw: RawPagedVideosData = decode_data(data, context)?;
    let items = raw
        .items
        .into_iter()
        .map(|item| item.into_summary(context))
        .collect::<Result<Vec<_>, ContractError>>()?;
    let pagination = build_page_info(raw.pagination, raw.total, request_page, items.len());
    Ok(PagedResult { items, pagination })
}

#[derive(Debug, Deserialize)]
struct RawRankItem {
    #[serde(default, deserialize_with = "de_u32_opt")]
    rank: Option<u32>,
    #[serde(default, deserialize_with = "de_u64_opt")]
    video_id: Option<u64>,
    #[serde(default, deserialize_with = "de_u64_opt")]
    series_id: Option<u64>,
    #[serde(default, deserialize_with = "de_u64_opt")]
    id: Option<u64>,
    #[serde(default)]
    title: String,
    #[serde(default, alias = "cover_url")]
    cover: String,
    #[serde(default, deserialize_with = "de_f32")]
    score: f32,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default, alias = "desc")]
    description: String,
    #[serde(default, deserialize_with = "de_u32_opt")]
    episode_count: Option<u32>,
    #[serde(default, deserialize_with = "de_u32_opt")]
    total_episodes: Option<u32>,
    #[serde(default, deserialize_with = "de_bool")]
    is_finished: bool,
    #[serde(default, deserialize_with = "de_u64")]
    hot: u64,
    #[serde(default)]
    badge: Option<String>,
    #[serde(default, deserialize_with = "de_bool")]
    is_original: bool,
    #[serde(default, deserialize_with = "de_bool")]
    is_hot: bool,
    #[serde(default, deserialize_with = "de_bool")]
    is_new: bool,
    #[serde(default, deserialize_with = "de_u64")]
    new_at: u64,
    #[serde(default)]
    metric_label: Option<String>,
    #[serde(default, deserialize_with = "de_u64_opt")]
    metric_value: Option<u64>,
    #[serde(default)]
    metrics: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct RawRankingData {
    #[serde(default)]
    items: Vec<RawRankItem>,
    #[serde(default)]
    pagination: Option<RawPagination>,
    #[serde(default, deserialize_with = "de_u64_opt")]
    total: Option<u64>,
}

fn default_metric_label(kind: RankingKind) -> &'static str {
    match kind {
        RankingKind::Hot => "热度",
        RankingKind::Potential => "飙升指数",
        RankingKind::Recommend => "口碑推荐",
    }
}

fn extract_metric_from_value(metrics: Option<&Value>) -> Option<u64> {
    let obj = metrics?.as_object()?;
    for key in ["value", "play", "hot", "heat", "score"] {
        if let Some(val) = obj.get(key) {
            if let Some(n) = val.as_u64() {
                return Some(n);
            }
            if let Some(f) = val.as_f64()
                && f.is_finite()
                && f >= 0.0
            {
                return format!("{:.0}", f.round()).parse::<u64>().ok();
            }
        }
    }
    None
}

pub fn parse_ranking(
    data: Value,
    kind: RankingKind,
    request_page: PageRequest,
) -> Result<PagedResult<RankingEntry>, ContractError> {
    let raw: RawRankingData = decode_data(data, "榜单")?;
    let page_offset = request_page
        .page
        .get()
        .saturating_sub(1)
        .saturating_mul(u32::from(request_page.size.get()));

    let items = raw
        .items
        .into_iter()
        .enumerate()
        .map(|(idx, item)| {
            let id = parse_series_id(item.video_id.or(item.series_id).or(item.id), "榜单条目")?;
            let metric_value = item
                .metric_value
                .or_else(|| extract_metric_from_value(item.metrics.as_ref()))
                .unwrap_or(item.hot);
            let metric_label = item
                .metric_label
                .map(|s| s.trim().to_owned())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| default_metric_label(kind).to_owned());
            let rank = item.rank.unwrap_or_else(|| {
                let pos = u32::try_from(idx).unwrap_or(u32::MAX);
                page_offset.saturating_add(pos).saturating_add(1)
            });
            let is_original = resolve_original_flag(item.is_original, item.badge.as_deref());
            let is_new = resolve_new_flag(item.is_new, item.new_at);
            let series = SeriesSummary {
                id,
                title: item.title.trim().to_owned(),
                cover_url: normalize_cover_url(&item.cover),
                score: item.score,
                tags: item.tags,
                description: item.description.trim().to_owned(),
                episode_count: resolve_episode_count(item.episode_count, item.total_episodes),
                is_finished: item.is_finished,
                hot: if item.hot > 0 { item.hot } else { metric_value },
                is_original,
                is_hot: item.is_hot,
                is_new,
            };
            Ok(RankingEntry {
                rank,
                series,
                metric_label,
                metric_value,
            })
        })
        .collect::<Result<Vec<_>, ContractError>>()?;

    let pagination = build_page_info(raw.pagination, raw.total, request_page, items.len());
    Ok(PagedResult { items, pagination })
}

#[derive(Debug, Deserialize)]
struct RawSearchSuggestData {
    #[serde(default, alias = "series")]
    content: Vec<RawVideoItem>,
    #[serde(default)]
    phrases: Option<Vec<String>>,
    #[serde(default)]
    items: Option<Vec<String>>,
    #[serde(default)]
    tags: Vec<String>,
}

pub fn parse_search_suggestions(data: Value) -> Result<SearchSuggestion, ContractError> {
    let raw: RawSearchSuggestData = decode_data(data, "搜索联想")?;
    let series = raw
        .content
        .into_iter()
        .map(|item| item.into_summary("搜索联想"))
        .collect::<Result<Vec<_>, ContractError>>()?;
    let phrases = raw
        .phrases
        .filter(|list| !list.is_empty())
        .or(raw.items)
        .unwrap_or_default();
    Ok(SearchSuggestion {
        series,
        phrases,
        tags: raw.tags,
    })
}

#[derive(Debug, Deserialize)]
struct RawHotKeywordItem {
    #[serde(default, alias = "keyword")]
    word: String,
    #[serde(default, alias = "hot", alias = "count", deserialize_with = "de_u64")]
    heat: u64,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawHotKeywordsData {
    Wrapped { items: Vec<RawHotKeywordItem> },
    List(Vec<RawHotKeywordItem>),
}

pub fn parse_hot_keywords(data: Value) -> Result<Vec<HotKeyword>, ContractError> {
    let items = match decode_data::<RawHotKeywordsData>(data, "热门搜索词")? {
        RawHotKeywordsData::Wrapped { items } | RawHotKeywordsData::List(items) => items,
    };
    Ok(items
        .into_iter()
        .map(|item| HotKeyword {
            word: item.word.trim().to_owned(),
            heat: item.heat,
        })
        .collect())
}

#[derive(Debug, Deserialize)]
struct RawHotTagItem {
    #[serde(default, alias = "tag")]
    name: String,
    #[serde(default, alias = "hot", alias = "count", deserialize_with = "de_u64")]
    heat: u64,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawHotTagsData {
    Wrapped { items: Vec<RawHotTagItem> },
    List(Vec<RawHotTagItem>),
}

pub fn parse_hot_tags(data: Value) -> Result<Vec<HotTag>, ContractError> {
    let items = match decode_data::<RawHotTagsData>(data, "热门标签")? {
        RawHotTagsData::Wrapped { items } | RawHotTagsData::List(items) => items,
    };
    Ok(items
        .into_iter()
        .map(|item| HotTag {
            name: item.name.trim().to_owned(),
            heat: item.heat,
        })
        .collect())
}
