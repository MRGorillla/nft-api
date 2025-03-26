use sqlx::{SqlitePool, Error};
use crate::models::{NFT}; 

#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self, Error> {
        let pool = SqlitePool::connect(database_url).await?;
        Ok(Self { pool })
    }

    pub async fn create_user(&self, id: &str, name: &str) -> Result<(), Error> {
        sqlx::query!(
            r#"
            INSERT INTO users (id, name)
            VALUES (?, ?)
            "#,
            id,
            name
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn create_nft(
        &self,
        id: &str,
        name: &str,
        description: Option<&str>,
        image_path: &str,
        owner_id: &str,
    ) -> Result<(), Error> {
        // Add created_at with SQLite's timestamp
        sqlx::query!(
            r#"
            INSERT INTO nfts (id, name, description, image_path, owner_id, created_at)
            VALUES (?, ?, ?, ?, ?, strftime('%s', 'now'))
            "#,
            id,
            name,
            description,
            image_path,
            owner_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_nfts_by_owner(&self, owner_id: &str) -> Result<Vec<NFT>, Error> {
        // Fix the query to handle NULL fields properly and use direct type annotation
        let rows = sqlx::query!(
            r#"
            SELECT 
                id as "id!", 
                name as "name!", 
                description, 
                image_path as "image_path!", 
                owner_id as "owner_id!",
                created_at as "created_at!: i64"
            FROM nfts
            WHERE owner_id = ?
            "#,
            owner_id
        )
        .fetch_all(&self.pool)
        .await?;

        // Convert the raw SQL rows to NFT structs
        let nfts = rows.into_iter().map(|row| {
            NFT {
                id: row.id,
                name: row.name,
                description: row.description,
                image_path: row.image_path,
                owner_id: row.owner_id,
                // Use chrono::DateTime::from_timestamp instead of deprecated method
                created_at: chrono::DateTime::from_timestamp(row.created_at, 0)
                    .unwrap_or_else(|| chrono::Utc::now())
                    .naive_utc(),
            }
        }).collect();

        Ok(nfts)
    }

    pub async fn transfer_nft(
        &self,
        transfer_id: &str,
        nft_id: &str,
        from_user_id: &str,
        to_user_id: &str,
    ) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;
        
        // Record the transfer with timestamp
        sqlx::query!(
            r#"
            INSERT INTO transfers (id, nft_id, from_user_id, to_user_id, transferred_at)
            VALUES (?, ?, ?, ?, strftime('%s', 'now'))
            "#,
            transfer_id,
            nft_id,
            from_user_id,
            to_user_id
        )
        .execute(&mut *tx)
        .await?;

        // Update NFT ownership
        sqlx::query!(
            r#"
            UPDATE nfts
            SET owner_id = ?
            WHERE id = ?
            "#,
            to_user_id,
            nft_id
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn user_exists(&self, user_id: &str) -> Result<bool, Error> {
        let result = sqlx::query!(
            "SELECT COUNT(*) as count FROM users WHERE id = ?",
            user_id
        )
        .fetch_one(&self.pool)
        .await?;
        
        Ok(result.count > 0)
    }
}