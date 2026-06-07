use diesel::prelude::*;
use krafted_back::favorites::models::NewFavorite;
use krafted_back::favorites::ports::FavoriteRepository;
use krafted_back::favorites::repository::DieselFavoriteRepository;
use krafted_back::schema::listings;
use krafted_back::schema::users;
use krafted_back::shared::db::{establish_pool, run_migrations};
use krafted_back::shared::errors::AppError;
use krafted_back::user::models::NewUser;
use testcontainers::clients::Cli;
use testcontainers_modules::postgres::Postgres;
use uuid::Uuid;

fn setup(
    docker: &Cli,
) -> (
    testcontainers::Container<'_, Postgres>,
    DieselFavoriteRepository,
    Uuid,
    Uuid,
) {
    let container = docker.run(Postgres::default());
    let port = container.get_host_port_ipv4(5432);
    let db_url = format!("postgres://postgres:postgres@localhost:{}/postgres", port);
    let pool = establish_pool(&db_url, 4);
    run_migrations(&pool);

    let mut conn = pool.get().unwrap();

    let seller_id = diesel::insert_into(users::table)
        .values(&NewUser {
            email: format!("seller-{}@example.com", Uuid::new_v4()),
            name: "Seller".to_string(),
            password_hash: String::new(),
        })
        .returning(users::id)
        .get_result::<Uuid>(&mut conn)
        .unwrap();

    let buyer_id = diesel::insert_into(users::table)
        .values(&NewUser {
            email: format!("buyer-{}@example.com", Uuid::new_v4()),
            name: "Buyer".to_string(),
            password_hash: String::new(),
        })
        .returning(users::id)
        .get_result::<Uuid>(&mut conn)
        .unwrap();

    use diesel::sql_types::Uuid as UuidType;
    let category_id: Uuid =
        diesel::dsl::sql::<UuidType>("SELECT id FROM categories ORDER BY name LIMIT 1")
            .get_result(&mut conn)
            .unwrap();

    let listing_id: Uuid = diesel::insert_into(listings::table)
        .values(&(
            listings::seller_id.eq(seller_id),
            listings::title.eq("Test Listing"),
            listings::description.eq("A test description"),
            listings::price_cents.eq(1000i32),
            listings::category_id.eq(category_id),
            listings::status.eq("active"),
            listings::condition.eq("handmade"),
            listings::quantity.eq(1i32),
        ))
        .returning(listings::id)
        .get_result(&mut conn)
        .unwrap();

    drop(conn);

    let repo = DieselFavoriteRepository::new(pool);
    (container, repo, buyer_id, listing_id)
}

#[tokio::test]
async fn test_create_favorite() {
    let docker = Cli::default();
    let (_container, repo, user_id, listing_id) = setup(&docker);

    let fav = repo
        .create(NewFavorite {
            user_id,
            listing_id,
        })
        .await
        .unwrap();
    assert_eq!(fav.user_id, user_id);
    assert_eq!(fav.listing_id, listing_id);
}

#[tokio::test]
async fn test_create_duplicate_favorite_returns_bad_request() {
    let docker = Cli::default();
    let (_container, repo, user_id, listing_id) = setup(&docker);

    repo.create(NewFavorite {
        user_id,
        listing_id,
    })
    .await
    .unwrap();

    let result = repo
        .create(NewFavorite {
            user_id,
            listing_id,
        })
        .await;
    assert!(matches!(result, Err(AppError::BadRequest(_))));
}

#[tokio::test]
async fn test_find_by_user_returns_favorites() {
    let docker = Cli::default();
    let (_container, repo, user_id, listing_id) = setup(&docker);

    repo.create(NewFavorite {
        user_id,
        listing_id,
    })
    .await
    .unwrap();

    let favorites = repo.find_by_user(user_id, 1, 20).await.unwrap();
    assert_eq!(favorites.len(), 1);
    assert_eq!(favorites[0].listing_id, listing_id);
}

#[tokio::test]
async fn test_find_by_user_empty_list() {
    let docker = Cli::default();
    let (_container, repo, user_id, _listing_id) = setup(&docker);

    let favorites = repo.find_by_user(user_id, 1, 20).await.unwrap();
    assert!(favorites.is_empty());
}

#[tokio::test]
async fn test_find_by_user_respects_pagination() {
    let docker = Cli::default();
    let (_container, repo, user_id, listing_id) = setup(&docker);

    let mut conn = krafted_back::shared::db::establish_pool(
        &format!(
            "postgres://postgres:postgres@localhost:{}/postgres",
            _container.get_host_port_ipv4(5432)
        ),
        4,
    )
    .get()
    .unwrap();

    let listing2_id: Uuid = {
        use diesel::sql_types::Uuid as UuidType;
        let category_id: Uuid =
            diesel::dsl::sql::<UuidType>("SELECT id FROM categories ORDER BY name LIMIT 1")
                .get_result(&mut conn)
                .unwrap();
        diesel::insert_into(listings::table)
            .values(&(
                listings::seller_id.eq(user_id),
                listings::title.eq("Listing 2"),
                listings::description.eq(""),
                listings::price_cents.eq(2000i32),
                listings::category_id.eq(category_id),
                listings::status.eq("active"),
                listings::condition.eq("handmade"),
                listings::quantity.eq(1i32),
            ))
            .returning(listings::id)
            .get_result(&mut conn)
            .unwrap()
    };
    drop(conn);

    repo.create(NewFavorite {
        user_id,
        listing_id,
    })
    .await
    .unwrap();
    repo.create(NewFavorite {
        user_id,
        listing_id: listing2_id,
    })
    .await
    .unwrap();

    let page1 = repo.find_by_user(user_id, 1, 1).await.unwrap();
    assert_eq!(page1.len(), 1);

    let page2 = repo.find_by_user(user_id, 2, 1).await.unwrap();
    assert_eq!(page2.len(), 1);
    assert_ne!(page1[0].id, page2[0].id);
}

#[tokio::test]
async fn test_count_by_user() {
    let docker = Cli::default();
    let (_container, repo, user_id, listing_id) = setup(&docker);

    repo.create(NewFavorite {
        user_id,
        listing_id,
    })
    .await
    .unwrap();

    let count = repo.count_by_user(user_id).await.unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn test_count_by_user_zero() {
    let docker = Cli::default();
    let (_container, repo, user_id, _listing_id) = setup(&docker);

    let count = repo.count_by_user(user_id).await.unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn test_find_by_user_and_listing_found() {
    let docker = Cli::default();
    let (_container, repo, user_id, listing_id) = setup(&docker);

    repo.create(NewFavorite {
        user_id,
        listing_id,
    })
    .await
    .unwrap();

    let found = repo
        .find_by_user_and_listing(user_id, listing_id)
        .await
        .unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().listing_id, listing_id);
}

#[tokio::test]
async fn test_find_by_user_and_listing_not_found() {
    let docker = Cli::default();
    let (_container, repo, user_id, _listing_id) = setup(&docker);

    let found = repo
        .find_by_user_and_listing(user_id, Uuid::new_v4())
        .await
        .unwrap();
    assert!(found.is_none());
}

#[tokio::test]
async fn test_delete_favorite() {
    let docker = Cli::default();
    let (_container, repo, user_id, listing_id) = setup(&docker);

    let fav = repo
        .create(NewFavorite {
            user_id,
            listing_id,
        })
        .await
        .unwrap();

    repo.delete(fav.id).await.unwrap();

    let found = repo
        .find_by_user_and_listing(user_id, listing_id)
        .await
        .unwrap();
    assert!(found.is_none());
}

#[tokio::test]
async fn test_delete_favorite_not_found() {
    let docker = Cli::default();
    let (_container, repo, _user_id, _listing_id) = setup(&docker);

    let result = repo.delete(Uuid::new_v4()).await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}
