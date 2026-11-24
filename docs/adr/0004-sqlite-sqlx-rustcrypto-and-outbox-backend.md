# 后端采用 SQLite（sqlx）、纯 Rust 加密传输与进程内 Outbox

Status: accepted

端侧本地自治架构要求 Android、Windows、macOS 三端共享同一套高性能、可交叉编译且零系统依赖的业务与数据底座，并严格保证离线片库与系统缓存物理隔离、跨上下文单向解耦以及 Windows 删除文件句柄安全释放。决定采用 `sqlx`（SQLite + WAL 模式 + 只追加迁移）、`reqwest`（`rustls`）+ 纯 Rust RustCrypto 加密族 + `hyper 1.x` 本地回环代理，并通过进程内 Transactional Outbox 协调离线资产删除后的播放释放。

## 备选方案

- `rusqlite` 同步直连加手写迁移：需额外维护异步桥接线程池，与 `tokio` 异步下载调度和回环代理割裂。
- `ureq` 同步客户端或绑定系统 OpenSSL：同步池并发分片下载效率差；OpenSSL 在 Android 与 Windows 交叉编译链路复杂。
- 删除离线分集前同步回调 `viewing` 停播：会引入 `offline -> viewing` 逆向依赖，违背单向限界上下文地图。

## 影响

- 持久化表按上下文前缀隔离且严禁跨上下文声明外键约束，所有表结构变更均通过 `migrations/` 只追加脚本演进。
- 本地存储强制分为目录缓存、播放缓存与离线片库三个独立目录，清缓存路径在结构上无法触碰离线片库目录。
- 删除离线资产在数据库事务内移除元数据并写入 `offline_outbox_events`，事务提交后由进程内 Outbox 立即通知 `viewing` 关闭回环代理句柄并异步回收磁盘目录（Windows 瞬时占用短退避重试）。
