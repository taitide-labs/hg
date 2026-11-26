# Changelog

## [0.4.1](https://github.com/taitide-labs/hg/compare/v0.4.0...v0.4.1) (2026-11-26)


### Bug Fixes

* **discover:** 修复分类浏览分页尺寸漂移及后续页加载中断问题 ([#95](https://github.com/taitide-labs/hg/issues/95)) ([ab0dff3](https://github.com/taitide-labs/hg/commit/ab0dff303e51126b6efe0633958f6fbd5639015d))

## [0.4.0](https://github.com/taitide-labs/hg/compare/v0.3.0...v0.4.0) (2026-11-25)


### Features

* **app:** 实现缓存与首屏流畅度优化、不喜欢闭环及播放器交互增强 ([#92](https://github.com/taitide-labs/hg/issues/92)) ([2f87a3f](https://github.com/taitide-labs/hg/commit/2f87a3f77df5882cd65e2bb2054bbfc0e68a8699))

## [0.3.0](https://github.com/taitide-labs/hg/compare/v0.2.1...v0.3.0) (2026-11-25)


### Features

* **app:** 优化发现搜索体验、深色沉浸视觉、窗口持久化与 NSH 性能 ([#90](https://github.com/taitide-labs/hg/issues/90)) ([ab00d18](https://github.com/taitide-labs/hg/commit/ab00d181cb2ce2b8d3f605927f622feac003408f))

## [0.2.1](https://github.com/taitide-labs/hg/compare/v0.2.0...v0.2.1) (2026-11-25)


### Bug Fixes

* **app:** 修复 Android 视频编译错误并重构前端测试分层 ([#88](https://github.com/taitide-labs/hg/issues/88)) ([e141c7a](https://github.com/taitide-labs/hg/commit/e141c7af9fc46298d1393a7b4f429ccb17e3030a))

## [0.2.0](https://github.com/taitide-labs/hg/compare/v0.1.2...v0.2.0) (2026-11-25)


### Features

* **app:** 修复播放器与持久化缺陷并重构导航搜索离线布局 ([#83](https://github.com/taitide-labs/hg/issues/83)) ([91d7b95](https://github.com/taitide-labs/hg/commit/91d7b95af3cbe459e5406cbf2ddeadd87db607b8))


### Bug Fixes

* **app:** 补齐响应式扩容、播放器交互打磨、离线分集弹窗与二次确认 ([#87](https://github.com/taitide-labs/hg/issues/87)) ([4146a5e](https://github.com/taitide-labs/hg/commit/4146a5ecc00a80cc7287a2fa1dc6e56c927dec0e))
* **workspace:** 修复重试任务取消缺陷并接线仅 Wi-Fi 策略与播放缓存淘汰 ([#86](https://github.com/taitide-labs/hg/issues/86)) ([3e6471f](https://github.com/taitide-labs/hg/commit/3e6471f4a07f2a234861e36ae2a9adacfe1ff746))

## [0.1.2](https://github.com/taitide-labs/hg/compare/v0.1.1...v0.1.2) (2026-11-24)


### Bug Fixes

* **release:** 同步全部工作区成员至 release-please 锁文件配置 ([#81](https://github.com/taitide-labs/hg/issues/81)) ([479c85f](https://github.com/taitide-labs/hg/commit/479c85f03b682f6912be47b5826ea814b27a92ea))

## [0.1.1](https://github.com/taitide-labs/hg/compare/v0.1.0...v0.1.1) (2026-11-24)


### Bug Fixes

* **player-windows:** 修复 MpvEngine 跨线程 Send 约束与事件线程句柄捕获 ([#79](https://github.com/taitide-labs/hg/issues/79)) ([e23a9fe](https://github.com/taitide-labs/hg/commit/e23a9fee9cd32a1776a52dba3ac4a10404536078))

## 0.1.0 (2026-11-24)


### Features

* **backend:** 接入本地真实网关、完成跨上下文集成终验并移除后端过渡开关 ([#63](https://github.com/taitide-labs/hg/issues/63)) ([bedc7d1](https://github.com/taitide-labs/hg/commit/bedc7d1511c4abc4592c7b884e1ca18ffa2d3d7d))
* **backend:** 落盘后端基建、共享领域、示范切片与全量待激活黑盒验收基线 ([#45](https://github.com/taitide-labs/hg/issues/45)) ([dd256e2](https://github.com/taitide-labs/hg/commit/dd256e2a6fda920e1f3714acfd21e54fcc6d1fa8))
* **client:** 开放全量客户端功能、切换本地网关并接入原生视频协议壳 ([#64](https://github.com/taitide-labs/hg/issues/64)) ([b88061e](https://github.com/taitide-labs/hg/commit/b88061e39fd83a7924e65efdf0c4308a73a4f56a))
* **client:** 新增回环封面解密落盘缓存并接入全站真图渲染 ([#76](https://github.com/taitide-labs/hg/issues/76)) ([02c1e8f](https://github.com/taitide-labs/hg/commit/02c1e8f3ba4dc4b4ed345dacd5c6594c75ac899c))
* **client:** 落盘南北向契约、客户端基建、示范切片与前置验收测试 ([#4](https://github.com/taitide-labs/hg/issues/4)) ([f8bd149](https://github.com/taitide-labs/hg/commit/f8bd1499b477db892bc766c5d809840f09437885))
* **player-android:** 实现 Android Media3 ExoPlayer 与 TextureView 实例池组件 ([#71](https://github.com/taitide-labs/hg/issues/71)) ([7e088d7](https://github.com/taitide-labs/hg/commit/7e088d7b5fc6c97bc37735a4b6fd359ab2aa0b34))
* **player-macos:** 实现 macOS AVPlayerLayer 实例池原生播放器组件 ([#70](https://github.com/taitide-labs/hg/issues/70)) ([a66e9fb](https://github.com/taitide-labs/hg/commit/a66e9fb885317ab078e299c75f94878d61c3fc7b))
* **player-windows:** 实现 Windows libmpv 纹理合成原生播放器组件 ([#73](https://github.com/taitide-labs/hg/issues/73)) ([3edbada](https://github.com/taitide-labs/hg/commit/3edbada20bb33a49b67d5faac77bed383f7d9704))


### Bug Fixes

* **infra:** 对齐边缘加密通道握手、信封编码与公共快路径线协议 ([#75](https://github.com/taitide-labs/hg/issues/75)) ([bc7dfea](https://github.com/taitide-labs/hg/commit/bc7dfea17c3c0c80f05a8efd11af108e9f0f4610))
