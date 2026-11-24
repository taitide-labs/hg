# Android Media3 原生播放器模块

本目录为 `hg-native-player` 的 Android 宿主 Gradle Library 模块（包名 `com.taitidelabs.hg.player`），基于 AndroidX Media3 `ExoPlayer` 与 `TextureView` 实例池实现 NSH `hg-video-player` 原生组件。

## 模块结构

- [HgVideoPlayerProtocol.kt](src/main/kotlin/com/taitidelabs/hg/player/HgVideoPlayerProtocol.kt)：组件名、8 个属性键名、3 类事件载荷定义、`VideoAspect` 枚举、属性状态机与 `TextureView` 缩放矩阵计算。
- [HgVideoPlayerPool.kt](src/main/kotlin/com/taitidelabs/hg/player/HgVideoPlayerPool.kt)：主线程 `ExoPlayer` 实例池（默认容量 `3`），复用 `HlsMediaSource.Factory` 并在借还时统一执行状态重置与超容销毁。
- [HgVideoPlayerComponent.kt](src/main/kotlin/com/taitidelabs/hg/player/HgVideoPlayerComponent.kt)：实现 `com.longbridge.nsh.NshComponent`，以 `FrameLayout` 包裹 `TextureView` 避免 `SurfaceView` 穿透 NSH 叠层，并在主线程定时上报 `progress`、监听 `ended` 与 `error`。
- [HgVideoPlayerRegistration.kt](src/main/kotlin/com/taitidelabs/hg/player/HgVideoPlayerRegistration.kt)：提供 `register(nshView)` 一键注册与 `releasePool()` 池回收入口。

## 宿主接入步骤

1. 在宿主 Android 工程的 `settings.gradle.kts` 中引入本模块：

```kotlin
include(":hg-player")
project(":hg-player").projectDir = file("path/to/clients/native-player/android")
```

2. 在宿主 `app/build.gradle.kts` 中添加模块依赖，并保证运行时存在 NSH Android SDK（`nsh.aar`）：

```kotlin
dependencies {
    implementation(project(":hg-player"))
}
```

若需单独构建本模块，可将 `nsh.aar` 放入本模块的 `libs/` 目录，或通过 `-Pnsh.sdkJar=/path/to/classes.jar`（或环境变量 `NSH_ANDROID_SDK_JAR`）提供编译期依赖。

3. 在挂载 `NshView` 时注册播放器组件，并在页面或应用销毁时释放实例池：

```kotlin
import com.longbridge.nsh.NshView
import com.taitidelabs.hg.player.HgVideoPlayerRegistration

HgVideoPlayerRegistration.register(nshView)
```

在 `Activity.onDestroy()` 释放共享池：

```kotlin
HgVideoPlayerRegistration.releasePool()
```

## 协议与状态约束

- 所有组件生命周期、属性更新及实例池调用均在 Android 主线程执行。
- 播放源来自本地回环 HLS 代理（`127.0.0.1`），宿主清单需允许本地回环明文流量（已在 [nsh.json](../../app/nsh.json) 配置）。
- 进度回传间隔固定为 `500ms`，与 Rust 协议层 [protocol.rs](../src/protocol.rs) 及 [android.rs](../src/android.rs) 保持一致。
