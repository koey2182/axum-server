use std::{collections::HashMap, sync::Arc};

use bytes::{Bytes, BytesMut};
use mqttbytes::Publish;
use tokio::sync::{RwLock, mpsc};

use super::{topic, HandlerFn, MqttMessage};

// ── 데이터 ────────────────────────────────────────────────────────────────────

pub struct BrokerState {
    pub clients:       RwLock<HashMap<String, mpsc::Sender<Bytes>>>,
    pub subscriptions: RwLock<Vec<(String, String)>>, // (client_id, topic_filter)
}

// ── 생성 ──────────────────────────────────────────────────────────────────────

pub fn new_state() -> Arc<BrokerState> {
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

/// 핸들러를 각각 독립 태스크로 스폰한다.
/// 핸들러가 true를 반환하면 해당 태스크가 직접 구독자에게 전달한다.
/// 매칭 핸들러가 없으면 모든 구독자에게 기본 전달한다.
pub(crate) fn dispatch(
    state:    &Arc<BrokerState>,
    handlers: &HashMap<String, HandlerFn>,
    publish:  &Publish,
) {
    let msg = MqttMessage {
        topic:   publish.topic.clone(),
        payload: publish.payload.clone(),
        retain:  publish.retain,
    };

    for (filter, handler) in handlers {
        if topic::matches(filter, &msg.topic) {
            let filter  = filter.clone();
            let handler = Arc::clone(handler);
            let msg     = msg.clone();
            let state   = Arc::clone(state);
            tokio::spawn(async move {
                if handler(msg.clone()).await {
                    forward(&state, &filter, &msg).await;
                }
            });
        }
    }

}

/// 서버가 직접 발행 — 토픽과 매칭되는 sub_filter를 가진 구독자에게 전달
pub async fn publish(state: &BrokerState, msg: &MqttMessage) {
    let clients = state.clients.read().await;
    let subs    = state.subscriptions.read().await;

    for (client_id, sub_filter) in subs.iter() {
        if topic::matches(sub_filter, &msg.topic) {
            send_publish(clients.get(client_id), msg);
        }
    }
}

/// h_filter를 포함하는 sub_filter를 가진 구독자에게만 전달
async fn forward(state: &BrokerState, h_filter: &str, msg: &MqttMessage) {
    let clients = state.clients.read().await;
    let subs    = state.subscriptions.read().await;

    for (client_id, sub_filter) in subs.iter() {
        if topic::matches(sub_filter, &msg.topic) && topic::matches(h_filter, sub_filter) {
            send_publish(clients.get(client_id), msg);
        }
    }
}


fn send_publish(sender: Option<&mpsc::Sender<Bytes>>, msg: &MqttMessage) {
    if let Some(sender) = sender {
        let mut fwd = Publish::new(&msg.topic, mqttbytes::QoS::AtMostOnce, msg.payload.clone());
        fwd.retain = false;

        let mut buf = BytesMut::new();
        if fwd.write(&mut buf).is_ok() {
            let _ = sender.try_send(buf.freeze());
        }
    }
}
