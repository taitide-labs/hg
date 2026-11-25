use std::future::Future;
use std::num::NonZeroU64;
use std::pin::pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use hg_contracts::ids::SeriesId;
use hg_gateway::contract::AppGateway as _;
use hg_gateway::mock::MockGateway;
use nsh_engine::ColorScheme;

use super::start_on_settings_with;

fn block_ready<F: Future>(future: F) -> F::Output {
    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);
    match pin!(future).poll(&mut cx) {
        Poll::Ready(out) => out,
        Poll::Pending => panic!("mock gateway future should resolve immediately"),
    }
}

#[test]
fn settings_disliked_drawer_lists_undislikes_and_clears_with_popover_confirm() {
    let gw = Arc::new(MockGateway::new());
    let s1001 = SeriesId::new(NonZeroU64::new(1001).unwrap());
    let s1002 = SeriesId::new(NonZeroU64::new(1002).unwrap());
    block_ready(gw.dislike_series(s1001)).unwrap();
    block_ready(gw.dislike_series(s1002)).unwrap();

    let (gw, mut app) = start_on_settings_with(gw, ColorScheme::Light);
    app.wait_for("管理不喜欢").unwrap();
    app.wait_for("当前已隐藏 2 部不喜欢的剧目").unwrap();

    app.press("管理不喜欢").unwrap();
    app.wait_for("云端之上：千金归来").unwrap();
    app.wait_for("长安暗夜行").unwrap();

    app.press("取消不喜欢 云端之上：千金归来").unwrap();
    app.wait_for("确认移除").unwrap();
    app.press("取消").unwrap();
    app.wait_for_gone("确认移除", Duration::from_secs(2))
        .unwrap();
    assert_eq!(gw.snapshot_disliked().len(), 2);

    app.press("取消不喜欢 云端之上：千金归来").unwrap();
    app.wait_for("确认移除").unwrap();
    app.press("确认移除").unwrap();
    app.wait_for_gone("云端之上：千金归来", Duration::from_secs(2))
        .unwrap();
    assert_eq!(gw.snapshot_disliked().len(), 1);

    app.press("清空").unwrap();
    app.wait_for("确认清空").unwrap();
    app.press("确认清空").unwrap();
    app.wait_for("暂无不喜欢的剧目").unwrap();
    assert!(gw.snapshot_disliked().is_empty());
}
