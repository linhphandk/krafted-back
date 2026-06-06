use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Queryable, Selectable, Serialize, ToSchema, Clone, Debug)]
#[diesel(table_name = crate::schema::categories)]
pub struct Category {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub kind: String,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::categories)]
pub struct NewCategory {
    pub name: String,
    pub slug: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub enum CategoryKind {
    Craft,
    Supply,
}

impl std::fmt::Display for CategoryKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CategoryKind::Craft => write!(f, "craft"),
            CategoryKind::Supply => write!(f, "supply"),
        }
    }
}

impl std::str::FromStr for CategoryKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "craft" => Ok(CategoryKind::Craft),
            "supply" => Ok(CategoryKind::Supply),
            _ => Err(format!("invalid category kind: {}", s)),
        }
    }
}

impl<'de> Deserialize<'de> for CategoryKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[derive(Queryable, Selectable, Serialize, Clone, Debug)]
#[diesel(table_name = crate::schema::listings)]
pub struct Listing {
    pub id: Uuid,
    pub seller_id: Uuid,
    pub title: String,
    pub description: String,
    pub price_cents: i32,
    pub category_id: Uuid,
    pub status: String,
    pub condition: String,
    pub quantity: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::listings)]
pub struct NewListing {
    pub seller_id: Uuid,
    pub title: String,
    pub description: String,
    pub price_cents: i32,
    pub category_id: Uuid,
    pub status: String,
    pub condition: String,
    pub quantity: i32,
}

#[derive(Default, AsChangeset)]
#[diesel(table_name = crate::schema::listings)]
#[diesel(treat_none_as_null = false)]
pub struct UpdateListing {
    pub title: Option<String>,
    pub description: Option<String>,
    pub price_cents: Option<i32>,
    pub category_id: Option<Uuid>,
    pub status: Option<String>,
    pub condition: Option<String>,
    pub quantity: Option<i32>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub enum ListingStatus {
    Draft,
    Active,
    Paused,
    Closed,
}

impl std::fmt::Display for ListingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ListingStatus::Draft => write!(f, "draft"),
            ListingStatus::Active => write!(f, "active"),
            ListingStatus::Paused => write!(f, "paused"),
            ListingStatus::Closed => write!(f, "closed"),
        }
    }
}

impl std::str::FromStr for ListingStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "draft" => Ok(ListingStatus::Draft),
            "active" => Ok(ListingStatus::Active),
            "paused" => Ok(ListingStatus::Paused),
            "closed" => Ok(ListingStatus::Closed),
            _ => Err(format!("invalid listing status: {}", s)),
        }
    }
}

impl<'de> Deserialize<'de> for ListingStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub enum ListingCondition {
    Handmade,
    New,
    Vintage,
    Refurbished,
}

impl std::fmt::Display for ListingCondition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ListingCondition::Handmade => write!(f, "handmade"),
            ListingCondition::New => write!(f, "new"),
            ListingCondition::Vintage => write!(f, "vintage"),
            ListingCondition::Refurbished => write!(f, "refurbished"),
        }
    }
}

impl std::str::FromStr for ListingCondition {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "handmade" => Ok(ListingCondition::Handmade),
            "new" => Ok(ListingCondition::New),
            "vintage" => Ok(ListingCondition::Vintage),
            "refurbished" => Ok(ListingCondition::Refurbished),
            _ => Err(format!("invalid listing condition: {}", s)),
        }
    }
}

impl<'de> Deserialize<'de> for ListingCondition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub enum ListingSort {
    #[default]
    #[serde(rename = "newest")]
    Newest,
    #[serde(rename = "price_asc")]
    PriceAsc,
    #[serde(rename = "price_desc")]
    PriceDesc,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ListingFilters {
    pub status: Option<String>,
    pub category_id: Option<Uuid>,
    pub kind: Option<CategoryKind>,
    pub search: Option<String>,
    #[serde(default)]
    pub sort: ListingSort,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaginatedResult<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateListingRequest {
    pub title: String,
    pub description: String,
    pub price_cents: i32,
    pub category_id: Uuid,
    pub condition: ListingCondition,
    pub quantity: Option<i32>,
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct UpdateListingRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub price_cents: Option<i32>,
    pub category_id: Option<Uuid>,
    pub status: Option<ListingStatus>,
    pub condition: Option<ListingCondition>,
    pub quantity: Option<i32>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ListingResponse {
    pub id: String,
    pub seller_id: String,
    pub seller_name: Option<String>,
    pub title: String,
    pub description: String,
    pub price_cents: i32,
    pub category_id: String,
    pub category_name: Option<String>,
    pub status: String,
    pub condition: String,
    pub quantity: i32,
    pub images: Vec<ImageResponse>,
    pub created_at: String,
    pub updated_at: String,
}

impl ListingResponse {
    pub fn from_listing(
        listing: &Listing,
        category_name: Option<String>,
        seller_name: Option<String>,
        images: Vec<ImageResponse>,
    ) -> Self {
        Self {
            id: listing.id.to_string(),
            seller_id: listing.seller_id.to_string(),
            seller_name,
            title: listing.title.clone(),
            description: listing.description.clone(),
            price_cents: listing.price_cents,
            category_id: listing.category_id.to_string(),
            category_name,
            status: listing.status.clone(),
            condition: listing.condition.clone(),
            quantity: listing.quantity,
            images,
            created_at: listing.created_at.to_string(),
            updated_at: listing.updated_at.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PaginatedResponse<T: ToSchema> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

impl<T: ToSchema> PaginatedResponse<T> {
    pub fn from_paginated_result(result: PaginatedResult<T>) -> Self {
        let total_pages = if result.total > 0 {
            (result.total + result.per_page - 1) / result.per_page
        } else {
            0
        };
        Self {
            items: result.items,
            total: result.total,
            page: result.page,
            per_page: result.per_page,
            total_pages,
        }
    }
}

#[derive(Debug, Queryable, Selectable, Clone, Serialize)]
#[diesel(table_name = crate::schema::listing_images)]
pub struct ListingImage {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub url: String,
    pub thumbnail_url: String,
    pub s3_key: String,
    pub position: i32,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = crate::schema::listing_images)]
pub struct NewListingImage {
    pub listing_id: Uuid,
    pub url: String,
    pub thumbnail_url: String,
    pub s3_key: String,
    pub position: i32,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ImageResponse {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub url: String,
    pub thumbnail_url: String,
    pub position: i32,
    pub created_at: NaiveDateTime,
}

impl ImageResponse {
    pub fn from_image(img: &ListingImage) -> Self {
        Self {
            id: img.id,
            listing_id: img.listing_id,
            url: img.url.clone(),
            thumbnail_url: img.thumbnail_url.clone(),
            position: img.position,
            created_at: img.created_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ReorderImagesRequest {
    pub image_ids: Vec<Uuid>,
}
