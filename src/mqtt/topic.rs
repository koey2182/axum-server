/// MQTT 토픽 필터 매칭
///
/// - `+` : 단일 레벨 와일드카드
/// - `#` : 다중 레벨 와일드카드 (필터 마지막에만 유효)
/// - `$`로 시작하는 시스템 토픽은 일반 와일드카드(+, #)로 매칭되지 않음
pub fn matches(filter: &str, topic: &str) -> bool {
    if topic.starts_with('$') && !filter.starts_with('$') {
        return false;
    }

    let mut fi = filter.split('/');
    let mut ti = topic.split('/');

    loop {
        match (fi.next(), ti.next()) {
            (Some("#"), _)                 => return true,
            (Some("+"), Some(_))           => {}
            (Some(f),   Some(t)) if f == t => {}
            (None,      None)              => return true,
            _                              => return false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::matches;

    #[test]
    fn exact() {
        assert!(matches("a/b/c", "a/b/c"));
        assert!(!matches("a/b/c", "a/b/d"));
    }

    #[test]
    fn single_wildcard() {
        assert!(matches("a/+/c", "a/b/c"));
        assert!(matches("a/+/c", "a/x/c"));
        assert!(!matches("a/+/c", "a/b/d"));
        assert!(!matches("a/+",   "a/b/c")); // + 는 한 레벨만
    }

    #[test]
    fn multi_wildcard() {
        assert!(matches("#",   "a/b/c"));
        assert!(matches("a/#", "a/b/c"));
        assert!(matches("a/#", "a/b"));
        assert!(matches("a/#", "a"));      // # 는 0 레벨도 매칭
        assert!(!matches("b/#", "a/b/c"));
    }

    #[test]
    fn system_topic() {
        assert!(!matches("#",     "$SYS/broker"));
        assert!(!matches("+/x",   "$SYS/x"));
        assert!(matches("$SYS/#", "$SYS/broker"));
    }
}
