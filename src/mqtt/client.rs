use std::sync::Arc;

use bytes::{Bytes, BytesMut};
use mqttbytes::{
    ConnAck, ConnectReturnCode, Packet, Publish, QoS, SubAck, SubscribeReasonCode, UnsubAck,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::mpsc,
};

use std::collections::HashMap;

use super::{
    broker::{self, BrokerState},
    HandlerFn,
};

const MAX_PACKET: usize = 64 * 1024; // 64 KB

// ── 진입점 ────────────────────────────────────────────────────────────────────

pub(crate) async fn handle(
    stream:   TcpStream,
    state:    Arc<BrokerState>,
    handlers: Arc<HashMap<String, HandlerFn>>,
) {
    let (mut reader, mut writer) = tokio::io::split(stream);
    let (tx, mut rx) = mpsc::channel::<Bytes>(64);

    // 아웃바운드 쓰기 태스크
    tokio::spawn(async move {
        while let Some(data) = rx.recv().await {
            if writer.write_all(&data).await.is_err() {
                break;
            }
        }
    });

    let mut buf = BytesMut::with_capacity(4096);

    // CONNECT 패킷 수신
    let connect = match next_packet(&mut reader, &mut buf).await {
        Ok(Packet::Connect(c)) => c,
        Ok(_) => return,
        Err(e) => {
            eprintln!("[mqtt] CONNECT 오류: {e}");
            return;
        }
    };

    let client_id = connect.client_id.clone();
    broker::add_client(&state, client_id.clone(), tx.clone()).await;

    // CONNACK 전송
    if tx.send(encode_connack()).await.is_err() {
        broker::remove_client(&state, &client_id).await;
        return;
    }

    // 패킷 수신 루프
    let mut clean_disconnect = false;

    loop {
        match next_packet(&mut reader, &mut buf).await {
            Ok(Packet::Publish(p)) => {
                broker::dispatch(&state, &handlers, &p);
            }

            Ok(Packet::Subscribe(sub)) => {
                let filters: Vec<String> = sub.filters.iter().map(|f| f.path.clone()).collect();
                broker::subscribe(&state, &client_id, filters.clone()).await;
                let _ = tx.send(encode_suback(sub.pkid, filters.len())).await;
            }

            Ok(Packet::Unsubscribe(unsub)) => {
                broker::unsubscribe(&state, &client_id, &unsub.topics).await;
                let _ = tx.send(encode_unsuback(unsub.pkid)).await;
            }

            Ok(Packet::PingReq) => {
                let _ = tx.send(Bytes::from_static(&[0xD0, 0x00])).await;
            }

            Ok(Packet::Disconnect) => {
                clean_disconnect = true;
                break;
            }

            Err(_) => break,
            Ok(_)  => {}
        }
    }

    // Will 메시지 — 비정상 종료 시에만 발행
    if !clean_disconnect {
        if let Some(will) = connect.last_will {
            let p = Publish {
                dup:     false,
                qos:     QoS::AtMostOnce,
                retain:  will.retain,
                topic:   will.topic,
                pkid:    0,
                payload: will.message,
            };
            broker::dispatch(&state, &handlers, &p);
        }
    }

    broker::remove_client(&state, &client_id).await;
}

// ── 패킷 읽기 ─────────────────────────────────────────────────────────────────

async fn next_packet<R: AsyncReadExt + Unpin>(
    reader: &mut R,
    buf:    &mut BytesMut,
) -> std::io::Result<Packet> {
    loop {
        match mqttbytes::read(buf, MAX_PACKET) {
            Ok(packet) => return Ok(packet),

            Err(mqttbytes::Error::InsufficientBytes(_)) => {
                if reader.read_buf(buf).await? == 0 {
                    return Err(std::io::ErrorKind::UnexpectedEof.into());
                }
            }

            Err(e) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    e.to_string(),
                ))
            }
        }
    }
}

// ── 아웃바운드 패킷 인코딩 ────────────────────────────────────────────────────

fn encode_connack() -> Bytes {
    let mut buf = BytesMut::new();
    ConnAck::new(ConnectReturnCode::Success, false)
        .write(&mut buf)
        .expect("connack encoding failed");
    buf.freeze()
}

fn encode_suback(pkid: u16, count: usize) -> Bytes {
    let mut buf = BytesMut::new();
    SubAck::new(pkid, vec![SubscribeReasonCode::QoS0; count])
        .write(&mut buf)
        .expect("suback encoding failed");
    buf.freeze()
}

fn encode_unsuback(pkid: u16) -> Bytes {
    let mut buf = BytesMut::new();
    UnsubAck::new(pkid)
        .write(&mut buf)
        .expect("unsuback encoding failed");
    buf.freeze()
}
