use std::sync::LazyLock;

use tokio::sync::Mutex;
use ulid::Generator;

static ULID_FACTORY: LazyLock<Mutex<Generator>> = LazyLock::new(|| {
    Mutex::new(Generator::new())
});

pub async fn ulid() -> String {
    let mut  factory = ULID_FACTORY.lock().await;
    factory.generate().unwrap().to_string()
}