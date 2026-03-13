mod broker;
mod client;
mod topic;

use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc};

use bytes::Bytes;
use tokio::net::TcpListener;

pub use broker::{new_state, BrokerState};

// ── 매크로 지원 ───────────────────────────────────────────────────────────────

/// `handlers!` 매크로의 const 중복 검사에서 사용하는 문자열 비교 함수
#[doc(hidden)]
pub const fn str_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() { return false; }
    let a = a.as_bytes();
    let b = b.as_bytes();
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] { return false; }
        i += 1;
    }
    true
}

/// 토픽 필터 → 핸들러 맵을 생성한다. 중복 필터는 컴파일 에러.
///
/// ```rust
/// let h = handlers! {
///     "sensors/#" => mqtt::handler(|msg| async move { true }),
///     "cmd/+"     => mqtt::handler(|msg| async move { false }),
/// };
/// ```
#[macro_export]
macro_rules! handlers {
    ($( $filter:literal => $handler:expr ),* $(,)?) => {{
        const FILTERS: &[&str] = &[$( $filter ),*];
        const _: () = {
            let mut i = 0;
            while i < FILTERS.len() {
                let mut j = i + 1;
                while j < FILTERS.len() {
                    assert!(
                        !$crate::mqtt::str_eq(FILTERS[i], FILTERS[j]),
                        "duplicate MQTT topic filter"
                    );
                    j += 1;
                }
                i += 1;
            }
        };
        let mut map: ::std::collections::HashMap<
            ::std::string::String,
            $crate::mqtt::HandlerFn,
        > = ::std::collections::HashMap::new();
        $( map.insert($filter.to_owned(), $handler); )*
        map
    }};
}

// ── 공개 타입 ─────────────────────────────────────────────────────────────────

/// 핸들러로 전달되는 MQTT 메시지
#[derive(Clone)]
pub struct MqttMessage {
    pub topic:   String,
    pub payload: bytes::Bytes,
    pub retain:  bool,
}

pub type HandlerFn =
    Arc<dyn Fn(MqttMessage) -> Pin<Box<dyn Future<Output = bool> + Send + 'static>> + Send + Sync>;

/// 클로저를 `HandlerFn`으로 변환하는 헬퍼
///
/// 반환값: `true` → 구독자에게 전달, `false` → 전달 억제
pub fn handler<F, Fut>(f: F) -> HandlerFn
where
    F:   Fn(MqttMessage) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = bool> + Send + 'static,
{
    Arc::new(move |msg| Box::pin(f(msg)))
}

// ── publish ───────────────────────────────────────────────────────────────────

/// 입력한 토픽과 매칭되는 토픽을 구독 중인 모든 클라이언트에게 메시지를 발송한다.
pub async fn publish(state: &BrokerState, topic: impl Into<String>, payload: impl Into<Bytes>) {
    let msg = MqttMessage {
        topic:   topic.into(),
        payload: payload.into(),
        retain:  false,
    };
    broker::publish(state, &msg).await;
}

// ── serve ─────────────────────────────────────────────────────────────────────

/// 외부에서 생성한 `BrokerState`로 MQTT 브로커를 시작한다.
/// 반환된 Future를 `tokio::spawn`에 넘기면 브로커가 동작한다.
///
/// ```rust
/// let mqtt = mqtt::new_state();
/// tokio::spawn(mqtt::serve(Arc::clone(&mqtt), 1883, handlers! {}));
/// // 이후 mqtt::publish(&mqtt, "sensors/temp", "25").await;
/// ```
pub async fn serve(
    state:    Arc<BrokerState>,
    port:     u16,
    handlers: HashMap<String, HandlerFn>,
) {
    let handlers = Arc::new(handlers);

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
