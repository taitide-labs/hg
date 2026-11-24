# 本地回环流媒体代理与三端原生播放器组件架构

Status: accepted

上游视频源采用带自定义鉴权头的 HLS 流（含 HEVC 与 H.264），且产品同时要求竖屏短剧滑屏秒切与横屏电影精播。实测 NSH 内置 `Video` 不支持自定义 HTTP 头、每次切集销毁重建，且 Windows Media Foundation 缺少系统默认 HEVC 解码、Android 传统 `MediaPlayer` 对 HLS 弱网缓冲与精准定位表现不足。决定在进程内运行 Rust 本地回环 HLS 代理（`127.0.0.1`），并按平台注册高性能 `NativeComponent` 播放器（macOS 自封 `AVPlayerLayer` 实例池、Windows `libmpv` D3D11VA 硬解纹理合成、Android Media3 `ExoPlayer` + `TextureView` 实例池），同时在桌面电影模式提供可选外部播放器打开能力。

## 备选方案

- 直接复用 NSH 内置 `Video`：开发成本最低，但 Windows 默认无法硬解 HEVC、三端换集销毁重建导致短剧滑屏黑屏卡顿。
- 以桌面外部播放器（Movist Pro / PotPlayer）作为主播放路径：此类播放器缺少标准进度回传与连播控制 IPC，且独立窗口无法承载竖屏短剧模式。
- 全平台统一自研 `FFmpeg + wgpu` 播放器或全端编译 `libmpv`：前者音画同步与 ABR 维护成本极高，后者在 Android 端 `MediaCodec` 碎片化与零拷贝渲染风险明显高于 Media3 `ExoPlayer`。

## 影响

- Windows 端严禁使用子 `HWND` 嵌入视频（存在 Win32 Airspace 限制，会永久遮挡所有 NSH 自绘浮层与弹窗），必须通过 `NativeComponent::draw`（D3D11 共享纹理转 Direct2D）或 `NativeComponent::composed`（DirectComposition 分层树）输出画面。
- `nsh.json` 须在 macOS 配置 `NSAllowsLocalNetworking`、在 Android 仅放行 `127.0.0.1` 明文回环流量，并始终绑定 `127.0.0.1` 避免触发 Windows 防火墙弹窗。
- 本地回环代理同时统一承接 CDN 鉴权注入、签名续期、在线播放缓存、邻集预取与离线分集读取，使在线、离线与外部播放器对流地址完全同构。
