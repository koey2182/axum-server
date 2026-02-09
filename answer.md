Rust의 폴더 구조와 모듈 참조 관계는 아래 4개 키워드로 이해하면 가장 정확합니다.

1. `crate` (컴파일 단위)
2. `module` (코드 네임스페이스)
3. `mod` (모듈 선언/연결)
4. `use` (이름 가져오기)

---

## 1) 크레이트와 기본 폴더 구조

`Cargo.toml` 기준으로 보통 2가지 크레이트가 만들어집니다.

- Binary crate: `src/main.rs`
  - 실행 파일(`fn main`)을 만듭니다.
- Library crate: `src/lib.rs`
  - 재사용 가능한 API를 만듭니다.

정석 구조 예:

```text
project/
  Cargo.toml
  src/
    main.rs      # binary crate root
    lib.rs       # library crate root
    auth/
      mod.rs
      token.rs
```

핵심:
- `main.rs`와 `lib.rs`는 서로 "다른 crate root"입니다.
- `main.rs`에서 `lib.rs`의 코드를 쓸 때는 보통 `crate 이름`으로 접근합니다.

예:
```rust
use axum_server::auth::token::Token;
```
(패키지명이 `axum_server`라고 가정)

---

## 2) `mod`와 파일 연결 규칙

`mod`는 "이 모듈을 컴파일에 포함해라"라는 선언입니다.

예:
```rust
mod token;
```

위 선언이 `src/main.rs`에 있으면 컴파일러는 기본적으로 아래를 찾습니다.
- `src/token.rs`
- 또는 `src/token/mod.rs`

하위 모듈 예:
```rust
mod auth;
```

이 경우 컴파일러는
- `src/auth.rs`
- 또는 `src/auth/mod.rs`
를 찾고, 그 안에서 다시 `mod token;`을 선언하면 `src/auth/token.rs`로 이어집니다.

즉, `mod`는 모듈 트리(부모-자식 관계)를 만드는 문법입니다.

---

## 3) `use`는 "가져오기", `pub`는 "공개 범위"

- `use`: 긴 경로를 현재 스코프에서 짧게 쓰기 위한 이름 바인딩
- `pub`: 다른 모듈에서 접근 가능하게 공개

예:
```rust
// src/token.rs
pub struct Token {
    pub user_id: String,
}
```

```rust
// src/main.rs
mod token;
use crate::token::Token;
```

설명:
- `mod token;` 없으면 `token.rs` 자체가 트리에 안 들어와서 사용 불가
- `Token`에 `pub` 없으면 `main.rs`에서 접근 불가
- `user_id` 필드에 `pub` 없으면 `Token`은 보여도 필드 접근 불가

---

## 4) 경로 문법(`crate`, `self`, `super`)

모듈 경로는 보통 3개 기준으로 씁니다.

- `crate::...` : 현재 crate 루트부터 시작 (가장 명확, 권장)
- `self::...` : 현재 모듈부터 시작
- `super::...` : 부모 모듈부터 시작

예:
```rust
use crate::auth::token::Token;
use self::helper::parse;
use super::config::Settings;
```

---

## 5) 현재 프로젝트 구조에 대한 정석 가이드

현재 파일:
- `src/main.rs`
- `src/lib/token.rs`

여기서 `src/lib/` 폴더명은 가능하지만, Rust의 `src/lib.rs`(library crate root)와 이름이 겹쳐 초반 학습에서 혼동이 큽니다.

정석적으로는 아래 둘 중 하나를 추천합니다.

### 방법 A: Binary crate만 유지 (가장 단순)

구조:
```text
src/
  main.rs
  token.rs
```

`main.rs`:
```rust
mod token;
use crate::token::Token;
```

### 방법 B: Library crate 분리 (실무에서 많이 씀)

구조:
```text
src/
  main.rs
  lib.rs
  token.rs
```

`lib.rs`:
```rust
pub mod token;
```

`main.rs`:
```rust
use axum_server::token::Token;
```

장점:
- 비즈니스 로직을 `lib.rs` 쪽으로 분리 가능
- 테스트/재사용/확장 시 구조가 깔끔함

---

## 6) 자주 헷갈리는 포인트 정리

1. `mod`는 "파일 include"가 아니라 "모듈 선언"입니다.
2. `use`만으로는 파일이 연결되지 않습니다. 먼저 `mod`(또는 `lib.rs`에서 `pub mod`)가 필요합니다.
3. 접근 에러가 나면 대부분 `pub` 누락 또는 모듈 트리(`mod`) 누락입니다.
4. Rust 2018+에서는 `crate::` 경로를 명시하는 습관이 가장 읽기 쉽고 안전합니다.

---

한 줄 요약:
- `mod`로 트리를 만들고,
- `pub`로 경계를 열고,
- `use`로 이름을 가져와 사용합니다.
