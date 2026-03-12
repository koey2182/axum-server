mod broker;
mod client;
mod topic;

use std::{future::Future, pin::Pin, sync::Arc};

use tokio::net::TcpListener;

// ── 공개 타입 ─────────────────────────────────────────────────────────────────

/// 핸들러로 전달되는 MQTT 메시지
#[derive(Clone)]
pub struct MqttMessage {
    pub topic:   String,
    pub payload: bytes::Bytes,
    pub retain:  bool,
}

pub(crate) type HandlerFn =
    Arc<dyn Fn(MqttMessage) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> + Send + Sync>;

// ── Router ────────────────────────────────────────────────────────────────────

/// 토픽 필터와 핸들러 매핑 목록 (순수 데이터)
pub struct Router {
    handlers: Vec<(String, HandlerFn)>,
}

impl Router {
    pub fn new() -> Self {
        Self { handlers: vec![] }
    }

    /// 토픽 필터에 핸들러를 등록한다
    ///
    /// ```rust
    /// mqtt::Router::new()
    ///     .route("sensors/+/temp", |msg| async move {
    ///         println!("{}: {:?}", msg.topic, msg.payload);
    ///     });
    /// ```
    pub fn route<F, Fut>(mut self, filter: impl Into<String>, f: F) -> Self
    where
        F:   Fn(MqttMessage) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let boxed: HandlerFn = Arc::new(move |msg| Box::pin(f(msg)));
        self.handlers.push((filter.into(), boxed));
        self
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

// ── serve ─────────────────────────────────────────────────────────────────────

/// MQTT 브로커를 시작한다
///
/// ```rust
/// let router = mqtt::Router::new()
///     .route("sensors/#", handle_sensor);
///
/// tokio::spawn(mqtt::serve(1883, router));
/// ```
pub async fn serve(port: u16, router: Router) {
    let state    = broker::new_state();
    let handlers = Arc::new(router.handlers);

    let listener = TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .unwrap_or_else(|e| panic!("MQTT 리스너 바인딩 실패 (port {port}): {e}"));

    println!("[mqtt] 브로커 시작: {}", listener.local_addr().unwrap());

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                let state    = Arc::clone(&state);
                let handlers = Arc::clone(&handlers);
                tokio::spawn(async move {
                    client::handle(stream, state, handlers).await;
                    println!("[mqtt] 연결 종료: {addr}");
                });
            }
            Err(e) => eprintln!("[mqtt] accept 오류: {e}"),
        }
    }
}
