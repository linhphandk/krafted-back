use diesel::prelude::*;
use krafted_back::listing::models::{ListingFilters, ListingSort, NewListing, UpdateListing};
use krafted_back::listing::ports::{CategoryRepository, ListingRepository};
use krafted_back::listing::repository::{DieselCategoryRepository, DieselListingRepository};
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
    DieselCategoryRepository,
    DieselListingRepository,
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
    drop(conn);

    (
        container,
        DieselCategoryRepository::new(pool.clone()),
        DieselListingRepository::new(pool),
        seller_id,
    )
}

#[tokio::test]
async fn test_category_find_all() {
    let docker = Cli::default();
    let (_container, cat_repo, _listing_repo, _seller_id) = setup(&docker);

    let categories = cat_repo.find_all().await.unwrap();
    assert_eq!(categories.len(), 15);
}

#[tokio::test]
async fn test_category_find_by_id() {
    let docker = Cli::default();
    let (_container, cat_repo, _listing_repo, _seller_id) = setup(&docker);

    let all = cat_repo.find_all().await.unwrap();
    let first = all.first().unwrap();
    let found = cat_repo.find_by_id(first.id).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, first.id);
}

#[tokio::test]
async fn test_category_find_by_id_not_found() {
    let docker = Cli::default();
    let (_container, cat_repo, _listing_repo, _seller_id) = setup(&docker);

    let result = cat_repo.find_by_id(Uuid::new_v4()).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_category_find_by_kind_craft() {
    let docker = Cli::default();
    let (_container, cat_repo, _listing_repo, _seller_id) = setup(&docker);

    let categories = cat_repo.find_by_kind("craft").await.unwrap();
    assert_eq!(categories.len(), 8);
    for cat in &categories {
        assert_eq!(cat.kind, "craft");
    }
}

#[tokio::test]
async fn test_category_find_by_kind_supply() {
    let docker = Cli::default();
    let (_container, cat_repo, _listing_repo, _seller_id) = setup(&docker);

    let categories = cat_repo.find_by_kind("supply").await.unwrap();
    assert_eq!(categories.len(), 7);
    for cat in &categories {
        assert_eq!(cat.kind, "supply");
    }
}

#[tokio::test]
async fn test_listing_create() {
    let docker = Cli::default();
    let (_container, cat_repo, listing_repo, seller_id) = setup(&docker);

    let category_id = cat_repo.find_all().await.unwrap().first().unwrap().id;

    let listing = NewListing {
        seller_id,
        title: "Test Listing".to_string(),
        description: "A great item".to_string(),
        price_cents: 2500,
        category_id,
        status: "draft".to_string(),
        condition: "handmade".to_string(),
        quantity: 1,
    };
    let created = listing_repo.create(listing).await.unwrap();
    assert_eq!(created.title, "Test Listing");
    assert_eq!(created.seller_id, seller_id);
    assert_eq!(created.status, "draft");
}

#[tokio::test]
async fn test_listing_find_by_id() {
    let docker = Cli::default();
    let (_container, cat_repo, listing_repo, seller_id) = setup(&docker);

    let category_id = cat_repo.find_all().await.unwrap().first().unwrap().id;

    let new_listing = NewListing {
        seller_id,
        title: "Find Me".to_string(),
        description: "".to_string(),
        price_cents: 1000,
        category_id,
        status: "active".to_string(),
        condition: "handmade".to_string(),
        quantity: 1,
    };
    let created = listing_repo.create(new_listing).await.unwrap();

    let found = listing_repo.find_by_id(created.id).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().title, "Find Me");
}

#[tokio::test]
async fn test_listing_find_by_id_not_found() {
    let docker = Cli::default();
    let (_container, _cat_repo, listing_repo, _seller_id) = setup(&docker);

    let result = listing_repo.find_by_id(Uuid::new_v4()).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_listing_find_all_default_filters() {
    let docker = Cli::default();
    let (_container, cat_repo, listing_repo, seller_id) = setup(&docker);

    let category_id = cat_repo.find_all().await.unwrap().first().unwrap().id;

    for i in 0..3 {
        let listing = NewListing {
            seller_id,
            title: format!("Listing {}", i),
            description: "".to_string(),
            price_cents: (i + 1) * 1000,
            category_id,
            status: "active".to_string(),
            condition: "handmade".to_string(),
            quantity: 1,
        };
        listing_repo.create(listing).await.unwrap();
    }

    let result = listing_repo
        .find_all(ListingFilters::default(), 1, 10)
        .await
        .unwrap();
    assert_eq!(result.items.len(), 3);
    assert_eq!(result.total, 3);
}

#[tokio::test]
async fn test_listing_find_all_filter_by_status() {
    let docker = Cli::default();
    let (_container, cat_repo, listing_repo, seller_id) = setup(&docker);

    let category_id = cat_repo.find_all().await.unwrap().first().unwrap().id;

    let draft = NewListing {
        seller_id,
        title: "Draft".to_string(),
        description: "".to_string(),
        price_cents: 1000,
        category_id,
        status: "draft".to_string(),
        condition: "handmade".to_string(),
        quantity: 1,
    };
    listing_repo.create(draft).await.unwrap();

    let active = NewListing {
        seller_id,
        title: "Active".to_string(),
        description: "".to_string(),
        price_cents: 2000,
        category_id,
        status: "active".to_string(),
        condition: "handmade".to_string(),
        quantity: 1,
    };
    listing_repo.create(active).await.unwrap();

    let filters = ListingFilters {
        status: Some("draft".to_string()),
        category_id: None,
        kind: None,
        search: None,
        sort: ListingSort::Newest,
    };
    let result = listing_repo.find_all(filters, 1, 10).await.unwrap();
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.items[0].title, "Draft");
}

#[tokio::test]
async fn test_listing_find_all_with_search() {
    let docker = Cli::default();
    let (_container, cat_repo, listing_repo, seller_id) = setup(&docker);

    let category_id = cat_repo.find_all().await.unwrap().first().unwrap().id;

    let listing = NewListing {
        seller_id,
        title: "Handmade Wooden Bowl".to_string(),
        description: "".to_string(),
        price_cents: 3500,
        category_id,
        status: "active".to_string(),
        condition: "handmade".to_string(),
        quantity: 1,
    };
    listing_repo.create(listing).await.unwrap();

    let filters = ListingFilters {
        status: None,
        category_id: None,
        kind: None,
        search: Some("Wooden".to_string()),
        sort: ListingSort::Newest,
    };
    let result = listing_repo.find_all(filters, 1, 10).await.unwrap();
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.items[0].title, "Handmade Wooden Bowl");
}

#[tokio::test]
async fn test_listing_find_all_with_pagination() {
    let docker = Cli::default();
    let (_container, cat_repo, listing_repo, seller_id) = setup(&docker);

    let category_id = cat_repo.find_all().await.unwrap().first().unwrap().id;

    for i in 0..5 {
        let listing = NewListing {
            seller_id,
            title: format!("Listing {}", i),
            description: "".to_string(),
            price_cents: (i + 1) * 1000,
            category_id,
            status: "active".to_string(),
            condition: "handmade".to_string(),
            quantity: 1,
        };
        listing_repo.create(listing).await.unwrap();
    }

    let result = listing_repo
        .find_all(ListingFilters::default(), 1, 2)
        .await
        .unwrap();
    assert_eq!(result.items.len(), 2);
    assert_eq!(result.total, 5);
    assert_eq!(result.page, 1);
    assert_eq!(result.per_page, 2);
}

#[tokio::test]
async fn test_listing_find_all_kind_filter() {
    let docker = Cli::default();
    let (_container, cat_repo, listing_repo, seller_id) = setup(&docker);

    let all_cats = cat_repo.find_all().await.unwrap();
    let craft_cat = all_cats.iter().find(|c| c.kind == "craft").unwrap();
    let supply_cat = all_cats.iter().find(|c| c.kind == "supply").unwrap();

    let craft_listing = NewListing {
        seller_id,
        title: "Craft Item".to_string(),
        description: "".to_string(),
        price_cents: 1000,
        category_id: craft_cat.id,
        status: "active".to_string(),
        condition: "handmade".to_string(),
        quantity: 1,
    };
    listing_repo.create(craft_listing).await.unwrap();

    let supply_listing = NewListing {
        seller_id,
        title: "Supply Item".to_string(),
        description: "".to_string(),
        price_cents: 500,
        category_id: supply_cat.id,
        status: "active".to_string(),
        condition: "new".to_string(),
        quantity: 5,
    };
    listing_repo.create(supply_listing).await.unwrap();

    let filters = ListingFilters {
        status: None,
        category_id: None,
        kind: Some("craft".parse().unwrap()),
        search: None,
        sort: ListingSort::Newest,
    };
    let result = listing_repo.find_all(filters, 1, 10).await.unwrap();
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.items[0].title, "Craft Item");
}

#[tokio::test]
async fn test_listing_find_by_seller() {
    let docker = Cli::default();
    let (_container, cat_repo, listing_repo, seller_id) = setup(&docker);

    let category_id = cat_repo.find_all().await.unwrap().first().unwrap().id;

    for i in 0..3 {
        let listing = NewListing {
            seller_id,
            title: format!("Seller Item {}", i),
            description: "".to_string(),
            price_cents: (i + 1) * 1000,
            category_id,
            status: "active".to_string(),
            condition: "handmade".to_string(),
            quantity: 1,
        };
        listing_repo.create(listing).await.unwrap();
    }

    let result = listing_repo.find_by_seller(seller_id, 1, 10).await.unwrap();
    assert_eq!(result.items.len(), 3);
    assert_eq!(result.total, 3);
}

#[tokio::test]
async fn test_listing_update() {
    let docker = Cli::default();
    let (_container, cat_repo, listing_repo, seller_id) = setup(&docker);

    let category_id = cat_repo.find_all().await.unwrap().first().unwrap().id;

    let new_listing = NewListing {
        seller_id,
        title: "Original".to_string(),
        description: "Old description".to_string(),
        price_cents: 1000,
        category_id,
        status: "draft".to_string(),
        condition: "handmade".to_string(),
        quantity: 1,
    };
    let created = listing_repo.create(new_listing).await.unwrap();

    let updated = listing_repo
        .update(
            created.id,
            UpdateListing {
                title: Some("Updated".to_string()),
                description: None,
                price_cents: None,
                category_id: None,
                status: Some("active".to_string()),
                condition: None,
                quantity: None,
            },
        )
        .await
        .unwrap();

    assert_eq!(updated.title, "Updated");
    assert_eq!(updated.status, "active");
    assert_eq!(updated.price_cents, 1000);
}

#[tokio::test]
async fn test_listing_update_not_found() {
    let docker = Cli::default();
    let (_container, _cat_repo, listing_repo, _seller_id) = setup(&docker);

    let result = listing_repo
        .update(
            Uuid::new_v4(),
            UpdateListing {
                title: Some("Nope".to_string()),
                description: None,
                price_cents: None,
                category_id: None,
                status: None,
                condition: None,
                quantity: None,
            },
        )
        .await;

    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn test_listing_delete() {
    let docker = Cli::default();
    let (_container, cat_repo, listing_repo, seller_id) = setup(&docker);

    let category_id = cat_repo.find_all().await.unwrap().first().unwrap().id;

    let new_listing = NewListing {
        seller_id,
        title: "To Delete".to_string(),
        description: "".to_string(),
        price_cents: 1000,
        category_id,
        status: "draft".to_string(),
        condition: "handmade".to_string(),
        quantity: 1,
    };
    let created = listing_repo.create(new_listing).await.unwrap();

    listing_repo.delete(created.id).await.unwrap();
    let found = listing_repo.find_by_id(created.id).await.unwrap();
    assert!(found.is_none());
}

#[tokio::test]
async fn test_listing_delete_not_found() {
    let docker = Cli::default();
    let (_container, _cat_repo, listing_repo, _seller_id) = setup(&docker);

    let result = listing_repo.delete(Uuid::new_v4()).await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn test_listing_count_by_seller() {
    let docker = Cli::default();
    let (_container, cat_repo, listing_repo, seller_id) = setup(&docker);

    let category_id = cat_repo.find_all().await.unwrap().first().unwrap().id;

    for i in 0..3 {
        let listing = NewListing {
            seller_id,
            title: format!("Item {}", i),
            description: "".to_string(),
            price_cents: (i + 1) * 1000,
            category_id,
            status: "active".to_string(),
            condition: "handmade".to_string(),
            quantity: 1,
        };
        listing_repo.create(listing).await.unwrap();
    }

    let count = listing_repo.count_by_seller(seller_id).await.unwrap();
    assert_eq!(count, 3);
}
