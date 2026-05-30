use async_trait::async_trait;
use chrono::NaiveDateTime;
use krafted_back::auth::models::{Tokens, UserInfo};
use krafted_back::auth::ports::AuthProvider;
use krafted_back::auth::service::AuthService;
use krafted_back::session::models::{NewSession, Session};
use krafted_back::session::ports::SessionRepository;
use krafted_back::shared::errors::{AppError, AppResult};
use krafted_back::user::models::{NewUser, User};
use krafted_back::user::ports::UserRepository;
use mockall::mock;
use uuid::Uuid;

mock! {
    pub MockAuthProvider {}

    #[async_trait]
    impl AuthProvider for MockAuthProvider {
        async fn register(&self, email: &str, name: &str, password: &str) -> AppResult<(UserInfo, Tokens)>;
        async fn login(&self, email: &str, password: &str, password_hash: &str) -> AppResult<(Tokens, UserInfo)>;
        async fn introspect_token(&self, token: &str) -> AppResult<UserInfo>;
        async fn refresh_token(&self, refresh_token: &str) -> AppResult<Tokens>;
        async fn revoke_token(&self, token: &str) -> AppResult<()>;
        async fn generate_access_token(&self, user_id: &str, email: &str) -> AppResult<String>;
        fn token_expiry_seconds(&self) -> u64;
    }
}

mock! {
    pub MockUserRepo {}

    #[async_trait]
    impl UserRepository for MockUserRepo {
        async fn create(&self, user: NewUser) -> AppResult<User>;
        async fn find_by_email(&self, email: &str) -> AppResult<Option<User>>;
        async fn find_by_id(&self, id: Uuid) -> AppResult<Option<User>>;
    }
}

mock! {
    pub MockSessionRepo {}

    #[async_trait]
    impl SessionRepository for MockSessionRepo {
        async fn create(&self, session: NewSession) -> AppResult<Session>;
        async fn find_by_token(&self, token: &str) -> AppResult<Option<Session>>;
        async fn revoke(&self, token: &str) -> AppResult<()>;
    }
}

fn fake_user() -> User {
    User {
        id: Uuid::new_v4(),
        email: "test@example.com".to_string(),
        name: "Test".to_string(),
        created_at: NaiveDateTime::default(),
        updated_at: NaiveDateTime::default(),
        password_hash: "$2b$12$hashed".to_string(),
    }
}

fn fake_session() -> Session {
    Session {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        refresh_token: "test-refresh".to_string(),
        expires_at: NaiveDateTime::default(),
        created_at: NaiveDateTime::default(),
    }
}

fn fake_user_info() -> UserInfo {
    UserInfo {
        sub: "1".to_string(),
        email: "test@example.com".to_string(),
        name: "Test".to_string(),
        password_hash: "$2b$12$hashed".to_string(),
    }
}

fn fake_tokens() -> Tokens {
    Tokens {
        access_token: "fake-jwt".to_string(),
        refresh_token: String::new(),
        id_token: String::new(),
        expires_in: 900,
    }
}

fn new_service() -> AuthService<MockMockAuthProvider, MockMockUserRepo, MockMockSessionRepo> {
    AuthService::new(
        MockMockAuthProvider::new(),
        MockMockUserRepo::new(),
        MockMockSessionRepo::new(),
        7,
    )
}

#[tokio::test]
async fn test_register_success() {
    let mut mock_auth = MockMockAuthProvider::new();
    mock_auth
        .expect_register()
        .returning(|_, _, _| Ok((fake_user_info(), fake_tokens())));

    let mut mock_repo = MockMockUserRepo::new();
    mock_repo.expect_create().returning(|_| Ok(fake_user()));

    let service = AuthService::new(mock_auth, mock_repo, MockMockSessionRepo::new(), 7);
    let result = service
        .register(
            "test@example.com".to_string(),
            "Test".to_string(),
            "password123".to_string(),
        )
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_register_empty_email() {
    let service = new_service();
    let result = service
        .register("".to_string(), "Test".to_string(), "password123".to_string())
        .await;
    assert!(matches!(result, Err(AppError::BadRequest(_))));
}

#[tokio::test]
async fn test_register_empty_name() {
    let service = new_service();
    let result = service
        .register("test@example.com".to_string(), "".to_string(), "password123".to_string())
        .await;
    assert!(matches!(result, Err(AppError::BadRequest(_))));
}

#[tokio::test]
async fn test_register_short_password() {
    let service = new_service();
    let result = service
        .register("test@example.com".to_string(), "Test".to_string(), "short".to_string())
        .await;
    assert!(matches!(result, Err(AppError::BadRequest(_))));
}

#[tokio::test]
async fn test_login_success() {
    let mut mock_auth = MockMockAuthProvider::new();
    mock_auth
        .expect_login()
        .returning(|_, _, _| Ok((fake_tokens(), fake_user_info())));

    let mut mock_repo = MockMockUserRepo::new();
    mock_repo
        .expect_find_by_email()
        .returning(|_| Ok(Some(fake_user())));

    let mut mock_session = MockMockSessionRepo::new();
    mock_session.expect_create().returning(|_| Ok(fake_session()));

    let service = AuthService::new(mock_auth, mock_repo, mock_session, 7);
    let result = service
        .login("test@example.com".to_string(), "password123".to_string())
        .await;
    assert!(result.is_ok());
    let (user, tokens) = result.unwrap();
    assert_eq!(user.email, "test@example.com");
    assert!(!tokens.refresh_token.is_empty());
}

#[tokio::test]
async fn test_login_user_not_found() {
    let mut mock_repo = MockMockUserRepo::new();
    mock_repo.expect_find_by_email().returning(|_| Ok(None));

    let service = AuthService::new(
        MockMockAuthProvider::new(),
        mock_repo,
        MockMockSessionRepo::new(),
        7,
    );
    let result = service
        .login("nope@example.com".to_string(), "password123".to_string())
        .await;
    assert!(matches!(result, Err(AppError::BadRequest(_))));
}

#[tokio::test]
async fn test_logout_success() {
    let mut mock_session = MockMockSessionRepo::new();
    mock_session.expect_revoke().returning(|_| Ok(()));

    let service = AuthService::new(
        MockMockAuthProvider::new(),
        MockMockUserRepo::new(),
        mock_session,
        7,
    );
    let result = service.logout("test-refresh".to_string()).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_refresh_token_success() {
    let mut mock_auth = MockMockAuthProvider::new();
    mock_auth
        .expect_generate_access_token()
        .returning(|_, _| Ok("new-jwt".to_string()));
    mock_auth
        .expect_token_expiry_seconds()
        .returning(|| 900);

    let mut mock_repo = MockMockUserRepo::new();
    mock_repo
        .expect_find_by_id()
        .returning(|_| Ok(Some(fake_user())));

    let mut mock_session = MockMockSessionRepo::new();
    mock_session
        .expect_find_by_token()
        .returning(|_| {
            let mut s = fake_session();
            s.expires_at = chrono::Utc::now()
                .checked_add_signed(chrono::Duration::days(7))
                .unwrap()
                .naive_utc();
            Ok(Some(s))
        });
    mock_session.expect_revoke().returning(|_| Ok(())).times(1);
    mock_session.expect_create().returning(|_| Ok(fake_session()));

    let service = AuthService::new(mock_auth, mock_repo, mock_session, 7);
    let result = service.refresh_token("old-refresh".to_string()).await;
    assert!(result.is_ok());
    let (user, tokens) = result.unwrap();
    assert_eq!(user.email, "test@example.com");
    assert_eq!(tokens.access_token, "new-jwt");
    assert!(!tokens.refresh_token.is_empty());
}

#[tokio::test]
async fn test_refresh_token_invalid() {
    let mut mock_session = MockMockSessionRepo::new();
    mock_session.expect_find_by_token().returning(|_| Ok(None));

    let service = AuthService::new(
        MockMockAuthProvider::new(),
        MockMockUserRepo::new(),
        mock_session,
        7,
    );
    let result = service.refresh_token("bad".to_string()).await;
    assert!(matches!(result, Err(AppError::BadRequest(_))));
}

#[tokio::test]
async fn test_refresh_token_expired() {
    let mut mock_session = MockMockSessionRepo::new();
    mock_session.expect_find_by_token().returning(|_| {
        let mut s = fake_session();
        s.expires_at = chrono::Utc::now()
            .checked_sub_signed(chrono::Duration::days(1))
            .unwrap()
            .naive_utc();
        Ok(Some(s))
    });
    mock_session.expect_revoke().returning(|_| Ok(())).times(1);

    let service = AuthService::new(
        MockMockAuthProvider::new(),
        MockMockUserRepo::new(),
        mock_session,
        7,
    );
    let result = service.refresh_token("expired".to_string()).await;
    assert!(matches!(result, Err(AppError::BadRequest(_))));
}

#[tokio::test]
async fn test_get_current_user_success() {
    let mut mock_auth = MockMockAuthProvider::new();
    mock_auth
        .expect_introspect_token()
        .returning(|_| Ok(fake_user_info()));

    let mut mock_repo = MockMockUserRepo::new();
    mock_repo
        .expect_find_by_email()
        .returning(|_| Ok(Some(fake_user())));

    let service = AuthService::new(
        mock_auth,
        mock_repo,
        MockMockSessionRepo::new(),
        7,
    );
    let result = service.get_current_user("valid-jwt".to_string()).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().email, "test@example.com");
}

#[tokio::test]
async fn test_get_current_user_invalid_token() {
    let mut mock_auth = MockMockAuthProvider::new();
    mock_auth
        .expect_introspect_token()
        .returning(|_| Err(AppError::BadRequest("Invalid token".to_string())));

    let service = AuthService::new(
        mock_auth,
        MockMockUserRepo::new(),
        MockMockSessionRepo::new(),
        7,
    );
    let result = service.get_current_user("bad".to_string()).await;
    assert!(matches!(result, Err(AppError::BadRequest(_))));
}
