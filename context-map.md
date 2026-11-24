# 上下文地图

## 限界上下文

- [片库目录](./backend/contexts/catalog/context.md) — 支撑域与上游适配：封装上游短剧免登录只读源，提供内容发现、检索、剧目详情、只读评论、封面解密与单集在线播放凭据解析。
- [离线下载](./backend/contexts/offline/context.md) — 核心域：管理用户显式发起的分集离线下载任务队列、断点续传与持久化离线片库资产，与系统播放缓存物理隔离。
- [观影追剧](./backend/contexts/viewing/context.md) — 核心域：管理在线与离线统一播放源调度、在线播放缓存、剧目级观看历史与续播进度以及本地追剧收藏。

## 关系模式

- 片库目录 [OHS, PL] -> 离线下载 [CF]
  - 同步端口 `CatalogMetadataPort`：下载入队时拉取剧目与分集元数据快照及解密后封面字节。
  - 同步端口 `CatalogPlaybackTicketPort`：下载调度与凭据失效刷新时获取单集在线播放凭据。
- 片库目录 [OHS, PL] -> 观影追剧 [CF]
  - 同步端口 `CatalogMetadataPort`：记录观看历史与追剧收藏时获取剧目摘要信息。
  - 同步端口 `CatalogPlaybackTicketPort`：在线起播及签名失效刷新时获取单集播放凭据。
- 离线下载 [OHS, PL] -> 观影追剧 [CF]
  - 异步事件（Transactional Outbox）：发布 `OfflineEpisodeRemoved` 与 `OfflineSeriesRemoved`，下游订阅触发 `StopDeletedOfflinePlayback`，先安全停止当前离线播放并释放回环代理文件句柄。
  - 同步端口 `OfflineAssetQueryPort`：单集起播选源时查询指定分集是否已有完整可用的离线分集资产，以及聚合离线片库总占用体积。
