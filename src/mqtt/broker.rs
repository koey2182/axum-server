use std::{collections::HashMap, sync::Arc};

use bytes::{Bytes, BytesMut};
use mqttbytes::Publish;
use tokio::sync::{RwLock, mpsc};

use super::{topic, HandlerFn, MqttMessage};

// ── 데이터 ────────────────────────────────────────────────────────────────────

pub(crate) struct BrokerState {
    pub clients:       RwLock<HashMap<String, mpsc::Sender<Bytes>>>,
    pub subscriptions: RwLock<Vec<(String, String)>>, // (client_id, topic_filter)
}

// ── 생성 ──────────────────────────────────────────────────────────────────────

pub(crate) fn new_state() -> Arc<BrokerState> {
    Arc::new(BrokerState {
        clients:       RwLock::new(HashMap::new()),
        subscriptions: RwLock::new(Vec::new()),
    })
}

// ── 클라이언트 관리 ───────────────────────────────────────────────────────────

pub(crate) async fn add_client(state: &BrokerState, id: String, tx: mpsc::Sender<Bytes>) {
    state.clients.write().await.insert(id, tx);
}

pub(crate) async fn remove_client(state: &BrokerState, id: &str) {
    state.clients.write().await.remove(id);
    state.subscriptions.write().await.retain(|(cid, _)| cid != id);
}

// ── 구독 관리 ─────────────────────────────────────────────────────────────────

pub(crate) async fn subscribe(state: &BrokerState, id: &str, filters: Vec<String>) {
    let mut subs = state.subscriptions.write().await;
    for filter in filters {
        if !subs.iter().any(|(cid, f)| cid == id && f == &filter) {
            subs.push((id.to_owned(), filter));
        }
    }
}

pub(crate) async fn unsubscribe(state: &BrokerState, id: &str, filters: &[String]) {
    state
        .subscriptions
        .write()
        .await
        .retain(|(cid, f)| !(cid == id && filters.contains(f)));
}

// ── 메시지 디스패치 ───────────────────────────────────────────────────────────

pub(crate) async fn dispatch(
    state:    &BrokerState,
    handlers: &[(String, HandlerFn)],
    publish:  &Publish,
) {
    let msg = MqttMessage {
        topic:   publish.topic.clone(),
        payload: publish.payload.clone(),
        retain:  publish.retain,
    };

    // 등록된 내부 핸들러 실행 — 각각 독립 태스크로 스폰
    for (filter, handler) in handlers {
        if topic::matches(filter, &msg.topic) {
            let handler = Arc::clone(handler);
            let msg     = msg.clone();
            tokio::spawn(handler(msg));
        }
    }

    // 구독 중인 MQTT 클라이언트에 포워딩
    let clients = state.clients.read().await;
    let subs    = state.subscriptions.read().await;

    for (client_id, filter) in subs.iter() {
        if topic::matches(filter, &msg.topic) {
            if let Some(sender) = clients.get(client_id) {
                let mut fwd = Publish::new(
                    &msg.topic,
                    mqttbytes::QoS::AtMostOnce,
                    msg.payload.clone(),
                );
                fwd.retain = false; // 기존 구독자에게는 retain 플래그 제거

                let mut buf = BytesMut::new();
                if fwd.write(&mut buf).is_ok() {
                    let _ = sender.try_send(buf.freeze());
                }
            }
        }
    }
}
