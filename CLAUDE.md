# CLAUDE.md

이 파일은 이 저장소에서 작업하는 Claude Code(claude.ai/code)에게 안내를 제공합니다.

## 명령어

```bash
# 빌드
cargo build

# 실행 (.env 필요)
cargo run

# 검사 (바이너리 생성 없이 빠르게)
cargo check

# 전체 테스트 실행
cargo test

# 특정 테스트 실행
cargo test <테스트명>

# DB 마이그레이션 적용
sqlx migrate run

# 새 마이그레이션 추가
sqlx migrate add <마이그레이션명>
```

## 환경 변수 (.env)

서버 시작 시 모두 필요 — 하나라도 없으면 패닉 발생:

```
PORT=
DATABASE_URL=
ACCESS_SECRET=
REFRESH_SECRET=
ACCESS_MINUTES=
REFRESH_DAYS=
MQTT_PORT=        # 선택. 기본값 1883
```

## 아키텍처

axum + sqlx(PostgreSQL) 기반의 JWT 인증 API 서버.

**요청 흐름:**
1. `main.rs`가 환경변수를 읽고 `PgPool` 생성 및 `AppState` 조립, MQTT 브로커 spawn 후 `create_app(state)` 호출
2. `app.rs`의 `create_app(state: AppState) -> Router`가 `api::route()`로 axum `Router` 구성 (동기 함수, DB 연결 책임 없음)
3. `AppState { pool, mqtt }`이 axum의 `State` extractor를 통해 모든 핸들러에 공유됨

**인증 흐름:**
- `POST /api/auth/authorize` — `client_id`/`client_secret` 수신 (현재 `foo`/`bar` 하드코딩), access + refresh 토큰 발급
- `POST /api/auth/refresh` — `RefreshClaims` extractor가 Bearer refresh 토큰을 검증하고, 핸들러가 `jti`를 `refresh_tokens` DB 테이블에서 확인
- `GET /api/auth/protected` — `AccessClaims` extractor가 Bearer access 토큰을 검증

**토큰 추출 패턴:** `AccessClaims`와 `RefreshClaims`가 `FromRequestParts`를 구현하므로 핸들러 파라미터로 직접 사용 가능. 둘 다 `axum-extra`의 `TypedHeader`를 통해 `Authorization: Bearer <token>` 헤더를 추출.

**ID 생성 (`id.rs`):** `LazyLock<Mutex<Generator>>`로 비동기 동시 요청 상황에서 ULID 단조성 충돌 방지. `define_ids!` 매크로가 접두사 있는 ID 함수를 생성 (예: `user_id()` → `"user-<ULID>"`). 접두사 최대 5자, 중복 검사는 컴파일 타임에 수행.

**JWT 시크릿**은 최초 사용 시 `LazyLock<Keys>`에 한 번만 로딩됨. `TOKEN_LIFE`도 동일한 방식.

**MQTT 브로커 (`src/mqtt/`):**
`main.rs`에서 `mqtt::new_state()`로 `Arc<BrokerState>` 생성 후 `tokio::spawn(mqtt::serve(Arc::clone(&mqtt), mqtt_port, handlers! {}))`로 HTTP 서버와 병렬 실행. `serve()`는 직접 `async fn`이므로 호출만 하면 `Future`가 반환됨. 구현 철학은 **함수형** — 데이터(`BrokerState`)와 동작(자유 함수)을 분리하여 메서드 없이 구현.

- `mod.rs` — `MqttMessage`, `HandlerFn` 타입, `handler()` 헬퍼, `serve()` 진입점
- `broker.rs` — `BrokerState` (데이터 전용) + 자유 함수: `add_client`, `remove_client`, `subscribe`, `unsubscribe`, `dispatch`
- `client.rs` — `handle()`: TCP 연결 1개 담당. `next_packet()`으로 스트리밍 파싱, writer 분리 태스크, Will 메시지 지원
- `topic.rs` — `matches(filter, topic)`: `+`/`#` 와일드카드, `$` 시스템 토픽 보호

패킷 파싱은 `mqttbytes` 0.1.0 크레이트 사용 (`mqttbytes::v4::` 서브모듈 없음; 모든 타입이 루트에 노출). QoS 0만 지원 (MQTT 3.1.1).

**디스패치 로직 (`broker::dispatch`):**
- `serve()`는 `Arc<BrokerState>`, `u16` 포트, `HashMap<String, HandlerFn>` 수신
- 클라이언트가 publish하면 `dispatch()`가 토픽과 매칭되는 모든 핸들러를 각각 독립 `tokio::spawn` 태스크로 실행 (병렬, 비블로킹)
- 핸들러가 `true` 반환 → 해당 핸들러의 filter를 구독 중인 클라이언트에게만 메시지 전달
- 핸들러가 `false` 반환 → 전달 억제
- 매칭 핸들러가 없으면 아무 클라이언트에도 전달하지 않음
- **전달 범위 결정:** `topic::matches(h_filter, sub_filter)` — 핸들러 filter가 구독자의 sub_filter를 포함(cover)할 때만 전달

핸들러 등록 패턴:
```rust
let mqtt = mqtt::new_state();
tokio::spawn(mqtt::serve(Arc::clone(&mqtt), 1883, handlers! {
    "sensors/#" => mqtt::handler(|msg| async move {
        // 처리 로직
        true  // true → 해당 filter 구독자에게 전달, false → 억제
    }),
}));
// 이후 mqtt::publish(&mqtt, "sensors/temp", "25").await;
```

`handlers!` 매크로는 중복 필터를 컴파일 타임에 감지하며, `HashMap<String, HandlerFn>`을 반환.

## 미완성 영역

- `authorize`: 자격증명 하드코딩 (`foo`/`bar`), DB 유저 조회 없음; **발급한 refresh 토큰을 `refresh_tokens`에 INSERT하지 않아** `refresh` 엔드포인트가 항상 401 반환
- `refresh`: 기존 refresh 토큰을 DB에서 삭제하지 않음 (토큰 로테이션 미구현); 새 refresh 토큰도 DB에 INSERT하지 않음
- `users` 핸들러: 더미 쿼리 결과 반환
- `refresh_tokens` 테이블에 PK 없음 (`owner_id`에 unique index만 존재)
- MQTT: retained 메시지 미지원 (DB 영속화 미구현), QoS 1/2 미지원, 인증 없음

## 데이터베이스 스키마

- `users`: `id VARCHAR(32)` PK, `created_at TIMESTAMPTZ`
- `refresh_tokens`: `jti VARCHAR(32)`, `exp`/`iat TIMESTAMPTZ`, `owner_id VARCHAR(32)`
  - `UQ_refresh_tokens_owner`: `owner_id` unique index (유저당 refresh 토큰 1개)
  - PK 없음
