# 观影追剧

本上下文属于核心域，负责在线与离线统一播放源解析、播放会话控制、在线播放缓存（自动淘汰与手动清理）、剧目级观看历史与尾段自动连播进度记录，以及本地追剧收藏管理。

## 术语

**播放源**：单集起播时的解析结果，分为来自离线片库的本地离线源与来自上游播放凭据的在线流源。

- 等价：`PlaybackSource`
- 避免：下载任务、媒体分片

**播放缓存**：在线播放过程中由系统自动维护、受容量上限淘汰且可一键清空的临时流媒体缓冲数据。

- 等价：`PlaybackCache`
- 避免：离线下载、离线分集

**观看进度**：单集在当前设备上的已播放时长位置与完播标记。

- 等价：`WatchProgress`
- 避免：播放会话、历史列表

**观看历史**：按剧目聚合的本地观影记录，标识该剧最近观看的分集、续播位置与最后观看时间。

- 等价：`WatchHistoryEntry`
- 避免：播放日志、追剧收藏

**追剧收藏**：用户显式标记关注的本地剧目清单项，包含剧目摘要快照。

- 等价：`SeriesBookmark`
- 避免：离线剧目、观看历史

**播放偏好**：约束观影体验的全局用户偏好，涵盖在线优先流档位与播放缓存容量上限。

- 等价：`PlaybackPreference`
- 避免：下载策略

## Workflow

- `PrepareEpisodePlayback`：`ResolveEpisodePlayback` -> `EpisodePlaybackPrepared`
- `RefreshOnlinePlayback`：`RenewExpiredPlaybackTicket` -> `OnlinePlaybackRefreshed`
- `StopDeletedOfflinePlayback`：`HaltPlaybackForRemovedOffline` -> `OfflinePlaybackHalted`
- `RecordWatchProgress`：`ReportWatchProgress` -> `WatchProgressRecorded`
- `CompleteEpisode`：`MarkEpisodeCompleted` -> `EpisodeCompleted`
- `RemoveWatchHistory`：`DeleteWatchHistoryEntry` -> `WatchHistoryEntryRemoved`
- `ClearWatchHistory`：`RequestClearWatchHistory` -> `WatchHistoryCleared`
- `BookmarkSeries`：`AddSeriesBookmark` -> `SeriesBookmarked`
- `UnbookmarkSeries`：`RemoveSeriesBookmark` -> `SeriesUnbookmarked`
- `ClearPlaybackCache`：`RequestClearPlaybackCache` -> `PlaybackCacheCleared`
- `UpdatePlaybackPreference`：`SavePlaybackPreference` -> `PlaybackPreferenceUpdated`
