use std::sync::LazyLock;

use tokio::sync::Mutex;
use ulid::Generator;

static ULID_FACTORY: LazyLock<Mutex<Generator>> = LazyLock::new(|| Mutex::new(Generator::new()));

pub async fn ulid() -> String {
    ULID_FACTORY.lock().await.generate().unwrap().to_string()
}

const fn str_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

macro_rules! define_ids {
    ($($fn_name:ident => $prefix:expr),* $(,)?) => {
        $(const _: () = assert!($prefix.len() <= 5, "prefix는 최대 5자까지 가능합니다");)*

        const _: () = {
            let prefixes: &[&str] = &[$($prefix),*];
            let mut i = 0;
            while i < prefixes.len() {
                let mut j = i + 1;
                while j < prefixes.len() {
                    assert!(!str_eq(prefixes[i], prefixes[j]), "prefix가 중복됩니다");
                    j += 1;
                }
                i += 1;
            }
        };

        $(
            pub async fn $fn_name() -> String {
                format!(
                    "{}-{}",
                    $prefix,
                    ULID_FACTORY.lock().await.generate().unwrap().to_string()
                )
            }
        )*
    };
}

define_ids! {
    user_id => "user",
    jti     => "token",
}
