use std::collections::{BTreeMap, HashMap};
use std::num::{NonZeroU32, NonZeroU64};

use hg_contracts::catalog::{
    Category, CategoryRecommendation, CommentReply, EpisodeComment, EpisodeSummary, HomeBanner,
    HomeFeed, HotKeyword, HotTag, SeriesDetail, SeriesStats, SeriesSummary, TagGroup, TagItem,
};
use hg_contracts::ids::{CommentId, EpisodeKey, EpisodeNumber, SeriesId, StreamProfile};
use hg_contracts::offline::{
    DownloadPolicy, DownloadTaskState, DownloadTaskView, OfflineEpisodeView, OfflineSeriesView,
    PauseReason,
};
use hg_contracts::viewing::{
    PlaybackPreference, SearchHistoryEntry, SeriesBookmark, WatchHistoryEntry, WatchProgress,
};

pub(super) struct MockState {
    pub(super) categories: Vec<Category>,
    pub(super) tag_groups: Vec<TagGroup>,
    pub(super) hot_keywords: Vec<HotKeyword>,
    pub(super) hot_tags: Vec<HotTag>,
    pub(super) details: BTreeMap<SeriesId, SeriesDetail>,
    pub(super) comments: HashMap<EpisodeKey, Vec<EpisodeComment>>,
    pub(super) watch_history: Vec<WatchHistoryEntry>,
    pub(super) episode_progress: HashMap<EpisodeKey, WatchProgress>,
    pub(super) bookmarks: Vec<SeriesBookmark>,
    pub(super) search_history: Vec<SearchHistoryEntry>,
    pub(super) offline_library: Vec<OfflineSeriesView>,
    pub(super) download_tasks: Vec<DownloadTaskView>,
    pub(super) playback_preference: PlaybackPreference,
    pub(super) download_policy: DownloadPolicy,
    pub(super) playback_cache_bytes: u64,
    pub(super) catalog_cache_bytes: u64,
    pub(super) next_timestamp: u64,
}

fn series_id(raw: u64) -> SeriesId {
    SeriesId::new(NonZeroU64::new(raw).unwrap_or(NonZeroU64::MIN))
}

fn ep_num(raw: u32) -> EpisodeNumber {
    EpisodeNumber::new(NonZeroU32::new(raw).unwrap_or(NonZeroU32::MIN))
}

fn comment_id(raw: u64) -> CommentId {
    CommentId::new(NonZeroU64::new(raw).unwrap_or(NonZeroU64::MIN))
}

fn make_episodes(count: u32, prefix: &str, base_duration: u32) -> Vec<EpisodeSummary> {
    (1..=count)
        .map(|n| EpisodeSummary {
            ep_num: ep_num(n),
            title: format!("第 {n} 集 · {prefix}转折 {n}"),
            duration_secs: base_duration + (n % 5) * 12,
            playable: true,
        })
        .collect()
}

struct SeedSeriesSpec {
    id: u64,
    title: &'static str,
    cover_url: &'static str,
    score: f32,
    tags: &'static [&'static str],
    description: &'static str,
    episode_count: u32,
    is_finished: bool,
    hot: u64,
    is_original: bool,
    is_hot: bool,
    is_new: bool,
    ep_prefix: &'static str,
}

fn build_detail(spec: &SeedSeriesSpec) -> SeriesDetail {
    SeriesDetail {
        summary: SeriesSummary {
            id: series_id(spec.id),
            title: spec.title.to_owned(),
            cover_url: spec.cover_url.to_owned(),
            score: spec.score,
            tags: spec.tags.iter().map(|t| (*t).to_owned()).collect(),
            description: spec.description.to_owned(),
            episode_count: spec.episode_count,
            is_finished: spec.is_finished,
            hot: spec.hot,
            is_original: spec.is_original,
            is_hot: spec.is_hot,
            is_new: spec.is_new,
        },
        total_episodes: spec.episode_count,
        stats: SeriesStats {
            play_heat: spec.hot,
            likes: spec.hot / 9,
            bookmarks: spec.hot / 16,
            comments: spec.hot / 42,
        },
        episodes: make_episodes(spec.episode_count, spec.ep_prefix, 115),
    }
}

fn build_seed_details() -> BTreeMap<SeriesId, SeriesDetail> {
    let specs = [
        SeedSeriesSpec {
            id: 1001,
            title: "云端之上：千金归来",
            cover_url: "poster://amber-noir",
            score: 9.4,
            tags: &["都市", "逆袭", "女频"],
            description: "蛰伏三年的家族继承人重返云城，在董事会风云与情感试探间步步为营，揭开当年真相。",
            episode_count: 68,
            is_finished: true,
            hot: 984_200,
            is_original: true,
            is_hot: true,
            is_new: false,
            ep_prefix: "云城风云",
        },
        SeedSeriesSpec {
            id: 1002,
            title: "长安暗夜行",
            cover_url: "poster://crimson-ink",
            score: 9.2,
            tags: &["古风", "悬疑", "权谋"],
            description: "一纸密诏牵出十二年前旧案，大理寺少卿与机敏仵作联手，在长安百坊间层层抽丝剥茧。",
            episode_count: 80,
            is_finished: true,
            hot: 862_500,
            is_original: true,
            is_hot: false,
            is_new: false,
            ep_prefix: "长安迷局",
        },
        SeedSeriesSpec {
            id: 1003,
            title: "重返 1998：科技大亨",
            cover_url: "poster://slate-gold",
            score: 9.0,
            tags: &["年代", "商战", "男频"],
            description: "芯片架构师意外回到九十年代末，从一家小作坊起步，抢占移动互联网浪潮先机。",
            episode_count: 72,
            is_finished: false,
            hot: 745_000,
            is_original: true,
            is_hot: false,
            is_new: true,
            ep_prefix: "时代浪潮",
        },
        SeedSeriesSpec {
            id: 1004,
            title: "星穹机甲纪元",
            cover_url: "poster://indigo-spark",
            score: 8.9,
            tags: &["漫剧", "科幻", "热血"],
            description: "近地轨道防线告急，底层机修少年意外唤醒初代原型机，与宿敌并肩守护最后防线。",
            episode_count: 48,
            is_finished: false,
            hot: 689_300,
            is_original: false,
            is_hot: true,
            is_new: true,
            ep_prefix: "轨道突围",
        },
        SeedSeriesSpec {
            id: 1005,
            title: "雾港七日",
            cover_url: "poster://teal-mist",
            score: 9.1,
            tags: &["悬疑", "反转", "都市"],
            description: "暴风雨封锁海岛渡轮，七名陌生乘客各自隐瞒秘密，每一集都在推翻上一集的推理。",
            episode_count: 36,
            is_finished: true,
            hot: 612_800,
            is_original: false,
            is_hot: true,
            is_new: false,
            ep_prefix: "渡轮迷雾",
        },
        SeedSeriesSpec {
            id: 1006,
            title: "春风十里不及卿",
            cover_url: "poster://rose-warm",
            score: 8.8,
            tags: &["甜宠", "轻喜", "都市"],
            description: "毒舌建筑师与元气咖啡师因老街改造方案针锋相对，却在日常烟火气中双向奔赴。",
            episode_count: 54,
            is_finished: true,
            hot: 548_100,
            is_original: false,
            is_hot: false,
            is_new: false,
            ep_prefix: "老街晴日",
        },
    ];

    let mut details = BTreeMap::new();
    for spec in &specs {
        let detail = build_detail(spec);
        details.insert(detail.summary.id, detail);
    }
    details
}

fn build_seed_comments() -> HashMap<EpisodeKey, Vec<EpisodeComment>> {
    let mut map = HashMap::new();
    let key = EpisodeKey::new(series_id(1001), ep_num(1));
    map.insert(
        key,
        vec![
            EpisodeComment {
                id: comment_id(5001),
                nickname: "林深见鹿".to_owned(),
                avatar_color: "#C2410C".to_owned(),
                text: "第一集节奏好干脆，三分钟就把核心冲突立住了，镜头质感很干净。".to_owned(),
                relative_time: "2 小时前".to_owned(),
                like_count: 428,
                is_pinned: true,
                ip_region: Some("上海".to_owned()),
                reply_count: 2,
                preview_replies: vec![
                    CommentReply {
                        id: comment_id(5101),
                        nickname: "南风知意".to_owned(),
                        avatar_color: "#0F766E".to_owned(),
                        text: "同意，而且到第 12 集反转更狠，一口气刷停不下来。".to_owned(),
                        relative_time: "1 小时前".to_owned(),
                        like_count: 96,
                        reply_to_nickname: Some("林深见鹿".to_owned()),
                    },
                    CommentReply {
                        id: comment_id(5102),
                        nickname: "周游记".to_owned(),
                        avatar_color: "#4338CA".to_owned(),
                        text: "已经把前 20 集离线下载到平板了，出差路上看。".to_owned(),
                        relative_time: "35 分钟前".to_owned(),
                        like_count: 41,
                        reply_to_nickname: None,
                    },
                ],
            },
            EpisodeComment {
                id: comment_id(5002),
                nickname: "山月不知心底事".to_owned(),
                avatar_color: "#B45309".to_owned(),
                text: "女主眼神戏很稳，没有浮夸配音，难得一部不降智的都市短剧。".to_owned(),
                relative_time: "5 小时前".to_owned(),
                like_count: 185,
                is_pinned: false,
                ip_region: Some("广东".to_owned()),
                reply_count: 0,
                preview_replies: Vec::new(),
            },
        ],
    );
    map
}

fn build_seed_watch_history() -> Vec<WatchHistoryEntry> {
    vec![
        WatchHistoryEntry {
            series_id: series_id(1001),
            title: "云端之上：千金归来".to_owned(),
            cover_url: "poster://amber-noir".to_owned(),
            last_episode: ep_num(3),
            position_secs: 64,
            duration_secs: 139,
            episode_completed: false,
            total_episodes: 68,
            watched_at_epoch_secs: 1_763_968_000,
        },
        WatchHistoryEntry {
            series_id: series_id(1002),
            title: "长安暗夜行".to_owned(),
            cover_url: "poster://crimson-ink".to_owned(),
            last_episode: ep_num(1),
            position_secs: 125,
            duration_secs: 127,
            episode_completed: true,
            total_episodes: 80,
            watched_at_epoch_secs: 1_763_964_000,
        },
    ]
}

fn build_seed_episode_progress() -> HashMap<EpisodeKey, WatchProgress> {
    let mut map = HashMap::new();
    for entry in build_seed_watch_history() {
        let key = EpisodeKey::new(entry.series_id, entry.last_episode);
        map.insert(
            key,
            WatchProgress {
                key,
                position_secs: entry.position_secs,
                duration_secs: entry.duration_secs,
                completed: entry.episode_completed,
            },
        );
    }
    map
}

fn build_seed_bookmarks() -> Vec<SeriesBookmark> {
    vec![
        SeriesBookmark {
            series_id: series_id(1001),
            title: "云端之上：千金归来".to_owned(),
            cover_url: "poster://amber-noir".to_owned(),
            episode_count: 68,
            is_finished: true,
            score: 9.4,
            bookmarked_at_epoch_secs: 1_763_960_000,
        },
        SeriesBookmark {
            series_id: series_id(1005),
            title: "雾港七日".to_owned(),
            cover_url: "poster://teal-mist".to_owned(),
            episode_count: 36,
            is_finished: true,
            score: 9.1,
            bookmarked_at_epoch_secs: 1_763_955_000,
        },
    ]
}

fn build_seed_offline_library() -> Vec<OfflineSeriesView> {
    vec![OfflineSeriesView {
        series_id: series_id(1001),
        title: "云端之上：千金归来".to_owned(),
        cover_local_uri: "poster://amber-noir".to_owned(),
        total_episodes: 68,
        total_size_bytes: 58_400_000,
        episodes: vec![
            OfflineEpisodeView {
                ep_num: ep_num(1),
                title: "第 1 集 · 云城风云转折 1".to_owned(),
                duration_secs: 127,
                profile: StreamProfile::Hevc,
                size_bytes: 19_200_000,
                downloaded_at_epoch_secs: 1_763_950_000,
            },
            OfflineEpisodeView {
                ep_num: ep_num(2),
                title: "第 2 集 · 云城风云转折 2".to_owned(),
                duration_secs: 139,
                profile: StreamProfile::Hevc,
                size_bytes: 20_100_000,
                downloaded_at_epoch_secs: 1_763_950_120,
            },
            OfflineEpisodeView {
                ep_num: ep_num(3),
                title: "第 3 集 · 云城风云转折 3".to_owned(),
                duration_secs: 151,
                profile: StreamProfile::Hevc,
                size_bytes: 19_100_000,
                downloaded_at_epoch_secs: 1_763_950_240,
            },
        ],
    }]
}

fn build_seed_download_tasks() -> Vec<DownloadTaskView> {
    vec![
        DownloadTaskView {
            key: EpisodeKey::new(series_id(1001), ep_num(4)),
            series_title: "云端之上：千金归来".to_owned(),
            episode_title: "第 4 集 · 云城风云转折 4".to_owned(),
            cover_url: "poster://amber-noir".to_owned(),
            profile: StreamProfile::Hevc,
            state: DownloadTaskState::Downloading {
                completed_segments: 14,
                total_segments: 24,
                downloaded_bytes: 11_400_000,
            },
        },
        DownloadTaskView {
            key: EpisodeKey::new(series_id(1002), ep_num(2)),
            series_title: "长安暗夜行".to_owned(),
            episode_title: "第 2 集 · 长安迷局转折 2".to_owned(),
            cover_url: "poster://crimson-ink".to_owned(),
            profile: StreamProfile::Hevc,
            state: DownloadTaskState::Paused {
                reason: PauseReason::UserRequested,
                completed_segments: 6,
                total_segments: 22,
            },
        },
    ]
}

fn build_seed_categories() -> Vec<Category> {
    vec![
        Category {
            id: 24,
            name: "短剧".to_owned(),
            route: "duanju".to_owned(),
        },
        Category {
            id: 25,
            name: "漫剧".to_owned(),
            route: "manju".to_owned(),
        },
        Category {
            id: 26,
            name: "悬疑".to_owned(),
            route: "xuanyi".to_owned(),
        },
        Category {
            id: 27,
            name: "古风".to_owned(),
            route: "gufeng".to_owned(),
        },
    ]
}

fn build_seed_tag_groups() -> Vec<TagGroup> {
    vec![
        TagGroup {
            id: 1,
            code: "genre".to_owned(),
            name: "题材".to_owned(),
            tags: vec![
                TagItem {
                    id: 101,
                    name: "都市".to_owned(),
                    slug: "dushi".to_owned(),
                    is_hot: true,
                },
                TagItem {
                    id: 102,
                    name: "古风".to_owned(),
                    slug: "gufeng".to_owned(),
                    is_hot: true,
                },
                TagItem {
                    id: 103,
                    name: "悬疑".to_owned(),
                    slug: "xuanyi".to_owned(),
                    is_hot: true,
                },
                TagItem {
                    id: 104,
                    name: "科幻".to_owned(),
                    slug: "kehuan".to_owned(),
                    is_hot: false,
                },
            ],
        },
        TagGroup {
            id: 2,
            code: "tone".to_owned(),
            name: "看点".to_owned(),
            tags: vec![
                TagItem {
                    id: 201,
                    name: "逆袭".to_owned(),
                    slug: "nixi".to_owned(),
                    is_hot: true,
                },
                TagItem {
                    id: 202,
                    name: "反转".to_owned(),
                    slug: "fanzhuan".to_owned(),
                    is_hot: true,
                },
                TagItem {
                    id: 203,
                    name: "甜宠".to_owned(),
                    slug: "tianchong".to_owned(),
                    is_hot: false,
                },
            ],
        },
    ]
}

pub(super) fn build_initial_state() -> MockState {
    MockState {
        categories: build_seed_categories(),
        tag_groups: build_seed_tag_groups(),
        hot_keywords: vec![
            HotKeyword {
                word: "千金归来".to_owned(),
                heat: 98_400,
            },
            HotKeyword {
                word: "长安暗夜行".to_owned(),
                heat: 86_200,
            },
            HotKeyword {
                word: "重返 1998".to_owned(),
                heat: 74_500,
            },
            HotKeyword {
                word: "雾港七日".to_owned(),
                heat: 61_200,
            },
        ],
        hot_tags: vec![
            HotTag {
                name: "都市".to_owned(),
                heat: 92_000,
            },
            HotTag {
                name: "反转".to_owned(),
                heat: 84_500,
            },
            HotTag {
                name: "古风".to_owned(),
                heat: 79_100,
            },
            HotTag {
                name: "漫剧".to_owned(),
                heat: 68_300,
            },
        ],
        details: build_seed_details(),
        comments: build_seed_comments(),
        watch_history: build_seed_watch_history(),
        episode_progress: build_seed_episode_progress(),
        bookmarks: build_seed_bookmarks(),
        search_history: vec![SearchHistoryEntry {
            keyword: "千金".to_owned(),
            searched_at_epoch_secs: 1_763_969_000,
        }],
        offline_library: build_seed_offline_library(),
        download_tasks: build_seed_download_tasks(),
        playback_preference: PlaybackPreference::default(),
        download_policy: DownloadPolicy {
            preferred_profile: StreamProfile::Hevc,
            wifi_only: true,
            max_concurrent_tasks: 2,
            storage_directory: None,
        },
        playback_cache_bytes: 134_217_728,
        catalog_cache_bytes: 25_165_824,
        next_timestamp: 1_763_970_000,
    }
}

pub(super) fn build_home_feed(state: &MockState) -> HomeFeed {
    let all: Vec<SeriesSummary> = state.details.values().map(|d| d.summary.clone()).collect();
    let banners: Vec<HomeBanner> = all
        .iter()
        .take(3)
        .map(|s| HomeBanner {
            series_id: s.id,
            title: s.title.clone(),
            image_url: s.cover_url.clone(),
            score: s.score,
            description: s.description.clone(),
        })
        .collect();

    let drama_items: Vec<SeriesSummary> = all
        .iter()
        .filter(|s| !s.tags.iter().any(|t| t == "漫剧"))
        .cloned()
        .collect();
    let anime_items: Vec<SeriesSummary> = all
        .iter()
        .filter(|s| s.tags.iter().any(|t| t == "漫剧" || t == "悬疑"))
        .cloned()
        .collect();

    let mut latest = all.clone();
    latest.sort_by_key(|s| std::cmp::Reverse(s.id.get()));

    HomeFeed {
        banners,
        featured: all,
        latest,
        category_recommendations: vec![
            CategoryRecommendation {
                category_id: 24,
                category_name: "热门短剧".to_owned(),
                items: drama_items,
            },
            CategoryRecommendation {
                category_id: 25,
                category_name: "漫剧与悬疑".to_owned(),
                items: anime_items,
            },
        ],
    }
}
