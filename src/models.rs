use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::NaiveDateTime;
// Remove unused import
// use std::str::FromStr;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct User {
    pub id: String,
    pub name: String,
    pub aadhaar_number: Option<String>,
    pub phone_number: Option<String>,
    pub email: Option<String>,
    pub owner_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewUser {
    pub name: String,
    pub aadhaar_number: Option<String>,
    pub phone_number: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NFT {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub image_path: String,
    pub owner_id: String,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewNFT {
    pub name: String,
    pub description: Option<String>,
    pub owner_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<Vec<NFTAttribute>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Transfer {
    pub id: String,
    pub nft_id: String,
    pub from_user_id: String,
    pub to_user_id: String,
    pub transferred_at: NaiveDateTime,
    pub transaction_hash: Option<String>,
    pub property_data: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransferRequest {
    pub to_user_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NFTAttribute {
    pub trait_type: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NFTMetadata {
    pub name: String,
    pub description: String,
    pub image: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<Vec<NFTAttribute>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NFTQueryParams {
    #[serde(default)]
    pub owner_id: Option<String>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}