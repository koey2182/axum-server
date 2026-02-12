# 유저 생성
```sql
CREATE ROLE <유저 명> WITH LOGIN PASSWORD '<로그인 비밀번호>';
```

# 특정 데이터베이스 접근 권한 부여
```sql
GRANT CONNECT ON DATABASE <데이터베이스 명> TO <유저 명>;
```

# 데이터베이스에서 스키마 권한 부여
* <데이터베이스 명>에 먼저 접속
```sql
GRANT USAGE, CREATE ON SCHEMA public TO <유저 명>;
```

# 데이터베이스 내 모든 테이블에 대해 CRUD 권한 부여
```sql
-- 이미 존재하는 테이블에 대해 CRUD 권한 부여
GRANT SELECT, INSERT, UPDATE, DELETE, REFERENCES ON ALL TABLES IN SCHEMA public TO <유저 명>;

-- 시퀀스 권한 부여 (SERIAL/IDENTITY 사용 시 필요)
GRANT USAGE, SELECT, UPDATE ON ALL SEQUENCES IN SCHEMA public TO <유저 명>;

-- 앞으로 새로 만들어질 테이블/시퀀스에도 자동으로 권한 설정
ALTER DEFAULT PRIVILEGES IN SCHEMA public
GRANT SELECT, INSERT, UPDATE, DELETE, REFERENCES ON TABLES TO <유저 명>,
GRANT USAGE, SELECT, UPDATE ON SEQUENCES TO <유저 명>;
```
