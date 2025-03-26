use sqlx::{SqlitePool, Error};

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
        sqlx::query!(
            r#"
            INSERT INTO nfts (id, name, description, image_path, owner_id)
            VALUES (?, ?, ?, ?, ?)
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

    pub async fn get_nfts_by_owner(&self, owner_id: &str) -> Result<Vec<crate::models::NFT>, Error> {
        sqlx::query_as(
            r#"
            SELECT 
                id, 
                name, 
                description, 
                image_path, 
                owner_id, 
                strftime('%s', created_at) as created_at
            FROM nfts
            WHERE owner_id = ?
            "#,
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn transfer_nft(
        &self,
        transfer_id: &str,
        nft_id: &str,
        from_user_id: &str,
        to_user_id: &str,
    ) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;
        
        // Record the transfer
        sqlx::query!(
            r#"
            INSERT INTO transfers (id, nft_id, from_user_id, to_user_id)
            VALUES (?, ?, ?, ?)
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

    // pub async fn get_user(&self, user_id: &str) -> Result<Option<crate::models::User>, Error> {
    //     sqlx::query_as(
    //         r#"
    //         SELECT id, name
    //         FROM users
    //         WHERE id = ?
    //         "#,
    //     )
    //     .bind(user_id)
    //     .fetch_optional(&self.pool)
    //     .await
    // }
}