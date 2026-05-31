use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Queryable, Selectable, Serialize, Clone, Debug)]
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

#[derive(Debug, Clone, Serialize)]
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

#[derive(AsChangeset)]
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

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
