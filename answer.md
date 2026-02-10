Axum에서 `State`로 `PgPool`을 주입한다는 것은,
"전역 변수로 DB를 꺼내 쓰는 방식" 대신
"앱 시작 시 만든 DB 풀을 라우터에 넣고, 핸들러가 필요할 때 안전하게 받아 쓰는 방식"입니다.

이 방식이 실무에서 더 정석인 이유는 다음과 같습니다.

- 테스트가 쉬움: 테스트용 풀(mock/테스트 DB)로 바꿔 주입 가능
- 전역 상태 의존이 줄어듦: 코드 추적이 쉬움
- Axum 구조와 잘 맞음: 라우터가 사용하는 상태를 타입으로 명시

---

## 1) 먼저 `AppState`를 만든다

보통 여러 공용 리소스를 묶어서 상태 구조체를 만듭니다.

```rust
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
}
```

왜 `Clone`?

- Axum은 내부적으로 상태를 라우터/서비스에 복사해서 들고 있어야 할 때가 있습니다.
- `PgPool`의 `clone()`은 "새 DB 연결 생성"이 아니라 "같은 풀 핸들 공유"라서 비용이 작습니다.

---

## 2) `main`에서 풀을 만든 뒤 라우터에 주입한다

현재 `main.rs`는 풀을 생성하지만 라우터에 연결하지 않고 있습니다.
`with_state(...)`를 붙여서 주입해야 핸들러에서 꺼내 쓸 수 있습니다.

```rust
use axum::{Router, routing::{get, post}};
use sqlx::PgPool;

#[derive(Clone)]
struct AppState {
    pool: PgPool,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("postgres is not ready");

    let state = AppState { pool };

    let app = Router::new()
        .route("/", get(handler))
        .route("/authorize", post(authorize))
        .route("/protected", get(protected))
        .route("/refresh", post(refresh))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

---

## 3) 핸들러에서 `State<AppState>`로 꺼내 쓴다

핸들러 인자에 `State(state): State<AppState>`를 추가하면 됩니다.

```rust
use axum::extract::State;

async fn users(State(state): State<AppState>) -> Result<String, AuthError> {
    let row: (i64,) = sqlx::query_as("SELECT 1")
        .fetch_one(&state.pool)
        .await
        .map_err(|_| AuthError::InvalidToken)?;

    Ok(format!("db ok: {}", row.0))
}
```

핵심 포인트:

- `State<T>`의 `T`는 라우터에 `with_state(...)`로 넣은 타입과 같아야 합니다.
- 타입이 다르면 컴파일 에러가 납니다.
- 즉, 런타임에서 터지는 게 아니라 컴파일 시점에 잡아줍니다.

---

## 4) `PgPool`만 직접 상태로 넣는 단순형도 가능

앱이 작다면 `AppState` 없이 바로 `PgPool` 자체를 상태로 써도 됩니다.

```rust
let app = Router::new()
    .route("/users", get(users))
    .with_state(pool);

async fn users(State(pool): State<PgPool>) {
    // pool 사용
}
```

하지만 보통은 나중에 `redis`, `config`, `jwt key` 등이 추가되므로
처음부터 `AppState` 구조체로 묶는 편이 확장성에 유리합니다.

---

## 5) 현재 프로젝트 기준 정리

현재 코드에서는 아래 방향이 가장 자연스럽습니다.

- `src/main.rs`에서 `PgPool::connect(...).await` 수행
- `AppState { pool }` 생성
- `Router::new()...with_state(state)` 적용
- DB가 필요한 핸들러만 `State<AppState>` 인자 추가

즉, `src/db.rs`의 전역 `LazyLock` 패턴보다,
`main`에서 생성한 풀을 Axum `State`로 주입하는 방식이 더 정석적이고 유지보수에 좋습니다.
