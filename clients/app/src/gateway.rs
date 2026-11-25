use std::sync::Arc;

use hg_gateway::contract::AppGateway;
use hg_gateway::mock::MockGateway;
use nsh_core::prelude::*;

#[derive(Clone)]
struct GatewaySlot(Arc<dyn AppGateway>);

impl Default for GatewaySlot {
    fn default() -> Self {
        Self(Arc::new(MockGateway::new()))
    }
}

pub fn install(gateway: Arc<dyn AppGateway>) {
    global::<GatewaySlot>().set(GatewaySlot(gateway));
}

#[must_use]
pub fn current() -> Arc<dyn AppGateway> {
    global::<GatewaySlot>().with_untracked(|slot| Arc::clone(&slot.0))
}

pub async fn join2<A, B, FA, FB>(fut_a: FA, fut_b: FB) -> (A, B)
where
    FA: std::future::Future<Output = A>,
    FB: std::future::Future<Output = B>,
{
    let mut fut_a = std::pin::pin!(fut_a);
    let mut fut_b = std::pin::pin!(fut_b);
    let mut out_a = None;
    let mut out_b = None;
    std::future::poll_fn(|cx| {
        if out_a.is_none()
            && let std::task::Poll::Ready(val) = fut_a.as_mut().poll(cx)
        {
            out_a = Some(val);
        }
        if out_b.is_none()
            && let std::task::Poll::Ready(val) = fut_b.as_mut().poll(cx)
        {
            out_b = Some(val);
        }
        if out_a.is_some()
            && out_b.is_some()
            && let (Some(a), Some(b)) = (out_a.take(), out_b.take())
        {
            std::task::Poll::Ready((a, b))
        } else {
            std::task::Poll::Pending
        }
    })
    .await
}

pub async fn join3<A, B, C, FA, FB, FC>(fut_a: FA, fut_b: FB, fut_c: FC) -> (A, B, C)
where
    FA: std::future::Future<Output = A>,
    FB: std::future::Future<Output = B>,
    FC: std::future::Future<Output = C>,
{
    let ((a, b), c) = join2(join2(fut_a, fut_b), fut_c).await;
    (a, b, c)
}
