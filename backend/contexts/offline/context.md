# 离线下载

本上下文属于核心域，负责管理用户显式发起的单集与批量离线下载队列、分片级断点续传、凭据过期自动刷新、下载策略偏好以及持久化离线片库资产，确保未完成半成品隔离与离线片库断网自足，并与系统自动管理的临时缓存物理隔离。

## 术语

**下载任务**：针对特定剧目单集的一次持久化获取作业，承载排队、取址、下载、暂停、失败、完成与取消状态及分片断点。

- 等价：`DownloadTask`
- 避免：缓存任务、缓冲任务

**流档位**：单集媒体可供选用的编码质量规格，包含高清（`hevc`）与标准兼容（`main`），排除预览流。

- 等价：`StreamProfile`
- 避免：清晰度链接、预览流

**离线剧目**：离线片库中按剧目聚合的本地实体，包含入队时固化的剧目元数据与本地封面快照。

- 等价：`OfflineSeries`
- 避免：在线剧目、追剧收藏

**离线分集**：经全部媒体分片校验并原子落盘后、可供完全断网播放的持久化本地分集资产。

- 等价：`OfflineEpisode`
- 避免：播放缓存、临时分片

**下载策略**：约束离线下载行为的全局用户偏好，涵盖优先流档位、仅 Wi-Fi 下载开关、并发上限与存储目录。

- 等价：`DownloadPolicy`
- 避免：播放偏好、缓存配置

## Workflow

- `EnqueueEpisodeDownloads`：`RequestEpisodeDownloads` -> `DownloadTasksQueued`
- `StartDownloadTask`：`ScheduleQueuedDownload` -> `DownloadTaskStarted`
- `RefreshDownloadTicket`：`RequestDownloadTicketRefresh` -> `DownloadTicketRefreshed`
- `CompleteDownloadTask`：`CommitDownloadedEpisode` -> `OfflineEpisodeCommitted`
- `FailDownloadTask`：`MarkDownloadTaskFailed` -> `DownloadTaskFailed`
- `PauseDownloadTask`：`PauseDownload` -> `DownloadTaskPaused`
- `ResumeDownloadTask`：`ResumeDownload` -> `DownloadTaskResumed`
- `RetryDownloadTask`：`RetryDownload` -> `DownloadTaskRequeued`
- `CancelDownloadTask`：`CancelDownload` -> `DownloadTaskCancelled`
- `RemoveOfflineEpisode`：`DeleteOfflineEpisode` -> `OfflineEpisodeRemoved`
- `RemoveOfflineSeries`：`DeleteOfflineSeries` -> `OfflineSeriesRemoved`
- `UpdateDownloadPolicy`：`SaveDownloadPolicy` -> `DownloadPolicyUpdated`
