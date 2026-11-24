use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use hg_gateway::contract::AppGateway;
use hg_gateway::local::{LocalGateway, LocalGatewayConfig};
use nsh_core::{EventKind, EventPayload, ViewKind};
use nsh_engine::{ColorScheme, EngineConfig, Environment, Os, Points, Size};
use nsh_testing::Headless;
use reqwest::StatusCode;
use tempfile::TempDir;
use tokio::runtime::Runtime;

use super::fake_upstream::FakeUpstreamServer;

const EXTERNAL_PLAYER_PREFIX: &str = "已准备外部播放地址：";

pub struct E2eHarness {
    pub _temp_dir: TempDir,
    pub io_rt: Runtime,
    pub upstream: FakeUpstreamServer,
    pub gateway: Arc<LocalGateway>,
    pub http: reqwest::Client,
    pub app: Headless,
}

impl E2eHarness {
    pub fn start() -> Result<Self, String> {
        let temp_dir = tempfile::tempdir().map_err(|err| format!("创建临时目录失败: {err}"))?;
        let io_rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("hg-e2e-io")
            .build()
            .map_err(|err| format!("创建测试异步运行时失败: {err}"))?;

        let upstream = io_rt.block_on(FakeUpstreamServer::start())?;
        let data_root = temp_dir.path().join("data");
        let cache_root = temp_dir.path().join("cache");
        let config = LocalGatewayConfig {
            db_path: data_root.join("hg.sqlite"),
            data_root,
            cache_root,
            upstream_base_url: upstream.base_url(),
        };
        let gateway = Arc::new(
            LocalGateway::open_with_config(&config)
                .map_err(|err| format!("启动 LocalGateway 失败: {err}"))?,
        );
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|err| format!("创建 HTTP 验证客户端失败: {err}"))?;

        let gateway_trait: Arc<dyn AppGateway> = Arc::clone(&gateway) as Arc<dyn AppGateway>;
        let environment = Environment {
            os: Os::MacOs,
            color_scheme: ColorScheme::Light,
            ..Environment::default()
        };
        let app = Headless::start(EngineConfig {
            viewport: Size::new(Points(1240.0), Points(800.0)),
            environment,
            ..EngineConfig::new(hg_app::root_with_gateway(gateway_trait))
        })
        .map_err(|err| format!("启动 Headless 客户端失败: {err}"))?;

        Ok(Self {
            _temp_dir: temp_dir,
            io_rt,
            upstream,
            gateway,
            http,
            app,
        })
    }

    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        self.io_rt.block_on(future)
    }

    pub fn capture_active_playable_uri(&mut self) -> Result<String, String> {
        self.app.hover("视频画面")?;
        self.app.wait_for("播放设置")?;
        self.app.press("播放设置")?;
        self.app.wait_for("外部播放器打开")?;
        self.app.press("外部播放器打开")?;
        self.app.wait_for("text:/已准备外部播放地址/")?;

        self.app
            .screen()
            .texts()
            .into_iter()
            .find_map(|line| line.strip_prefix(EXTERNAL_PLAYER_PREFIX).map(str::to_owned))
            .ok_or_else(|| "未在屏幕文本中发现外部播放地址".to_owned())
    }

    pub fn emit_video_progress(
        &mut self,
        position_secs: u32,
        duration_secs: u32,
    ) -> Result<(), String> {
        let video_node = self
            .app
            .screen()
            .layout_nodes()
            .into_iter()
            .find(|node| node.kind == ViewKind::Native)
            .map(|node| node.id)
            .ok_or_else(|| "未找到原生视频播放器节点".to_owned())?;

        self.app.engine().control().dispatch_event(
            video_node,
            EventKind::NativeEvent,
            EventPayload::Native {
                name: Arc::from("progress"),
                data: serde_json::json!({
                    "position_secs": position_secs,
                    "duration_secs": duration_secs,
                }),
            },
        );
        self.app
            .wait(Duration::from_millis(50))
            .map_err(|err| format!("等待播放进度同步失败: {err}"))
    }

    pub fn fetch_text(&self, url: &str) -> Result<(StatusCode, String), String> {
        let client = self.http.clone();
        let target = url.to_owned();
        self.block_on(async move {
            let resp = client
                .get(&target)
                .send()
                .await
                .map_err(|err| format!("请求 {target} 失败: {err}"))?;
            let status = resp.status();
            let text = resp
                .text()
                .await
                .map_err(|err| format!("读取 {target} 响应失败: {err}"))?;
            Ok((status, text))
        })
    }

    pub fn fetch_status(&self, url: &str) -> Result<StatusCode, String> {
        let client = self.http.clone();
        let target = url.to_owned();
        self.block_on(async move {
            let resp = client
                .get(&target)
                .send()
                .await
                .map_err(|err| format!("请求 {target} 失败: {err}"))?;
            Ok(resp.status())
        })
    }

    pub fn verify_hls_stream_ok(&self, playlist_url: &str) -> Result<(), String> {
        let (status, playlist) = self.fetch_text(playlist_url)?;
        if status != StatusCode::OK {
            return Err(format!("回环播放列表 {playlist_url} 返回状态码 {status}"));
        }
        if !playlist.contains("#EXTM3U") {
            return Err(format!("播放列表缺少 #EXTM3U: {playlist}"));
        }
        let seg_line = playlist
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty() && !line.starts_with('#'))
            .ok_or_else(|| "播放列表缺少分片路径".to_owned())?;
        let base =
            url::Url::parse(playlist_url).map_err(|err| format!("解析播放列表地址失败: {err}"))?;
        let seg_url = base
            .join(seg_line)
            .map_err(|err| format!("拼接分片地址失败: {err}"))?;
        let seg_status = self.fetch_status(seg_url.as_str())?;
        if seg_status != StatusCode::OK {
            return Err(format!("分片地址 {} 返回状态码 {seg_status}", seg_url));
        }
        Ok(())
    }

    pub fn drain_download_queue(&self) -> Result<usize, String> {
        let mut completed = 0_usize;
        loop {
            let next = self
                .block_on(self.gateway.process_next_download())
                .map_err(|err| format!("执行离线下载队列失败: {err}"))?;
            match next {
                Some(_) => completed = completed.saturating_add(1),
                None => return Ok(completed),
            }
        }
    }
}

impl Drop for E2eHarness {
    fn drop(&mut self) {
        self.upstream.shutdown();
    }
}
