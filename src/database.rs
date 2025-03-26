use sqlx::{SqlitePool, Error};
use crate::models::{NFT}; 

#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {    
    pub async fn run_migrations_for_instance(&self) -> Result<(), Error> {
        Self::run_migrations(&self.pool).await
    }

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
        token_id: Option<&str>,
        ipfs_image_cid: Option<&str>,
        ipfs_metadata_cid: Option<&str>,
        blockchain_tx_hash: Option<&str>,
    ) -> Result<(), Error> {
        // Ensure blockchain columns exist
        let column_exists = sqlx::query!(
            "SELECT COUNT(*) as count FROM pragma_table_info('nfts') WHERE name = ?",
            "token_id"
        )
        .fetch_one(&self.pool)
        .await?;
    
        if column_exists.count == 0 {
            sqlx::query("ALTER TABLE nfts ADD COLUMN token_id TEXT").execute(&self.pool).await.ok();
            sqlx::query("ALTER TABLE nfts ADD COLUMN ipfs_image_cid TEXT").execute(&self.pool).await.ok();
            sqlx::query("ALTER TABLE nfts ADD COLUMN ipfs_metadata_cid TEXT").execute(&self.pool).await.ok();
            sqlx::query("ALTER TABLE nfts ADD COLUMN blockchain_tx_hash TEXT").execute(&self.pool).await.ok();
        }
    
        // Now try the insertion
        sqlx::query!(
            r#"
            INSERT INTO nfts (
                id, name, description, image_path, owner_id, created_at,
                token_id, ipfs_image_cid, ipfs_metadata_cid, blockchain_tx_hash
            )
            VALUES (?, ?, ?, ?, ?, strftime('%s', 'now'), ?, ?, ?, ?)
            "#,
            id,
            name,
            description,
            image_path,
            owner_id,
            token_id,
            ipfs_image_cid,
            ipfs_metadata_cid,
            blockchain_tx_hash
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
        property_data: Option<&str>,
        transaction_hash: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // First get the NFT data for logging purposes
        let nft = self.get_nft_by_id(nft_id).await?;
        
        // Store the transfer record
        sqlx::query(
            "INSERT INTO transfers (id, nft_id, from_user_id, to_user_id, property_data, transaction_hash, transferred_at) 
             VALUES ($1, $2, $3, $4, $5, $6, $7)"
        )
        .bind(transfer_id)
        .bind(nft_id)
        .bind(from_user_id)
        .bind(to_user_id)
        .bind(property_data)
        .bind(transaction_hash)
        .bind(chrono::Local::now().naive_local())
        .execute(&self.pool)
        .await?;
        
        // Update the NFT ownership
        sqlx::query("UPDATE nfts SET owner_id = $1 WHERE id = $2")
            .bind(to_user_id)
            .bind(nft_id)
            .execute(&self.pool)
            .await?;
        
        Ok(())
    }
    
    // Add a method to retrieve transfer history for an NFT
    pub async fn get_nft_transfer_history(
        &self,
        nft_id: &str
    ) -> Result<Vec<Transfer>, Box<dyn std::error::Error>> {
        let transfers = sqlx::query_as!(
            Transfer,
            "SELECT id, nft_id, from_user_id, to_user_id, transferred_at, 
             transaction_hash, property_data FROM transfers 
             WHERE nft_id = $1 ORDER BY transferred_at DESC",
            nft_id
        )
        .fetch_all(&self.pool)
        .await?;
        
        Ok(transfers)
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
    pub async fn get_nft_owner(&self, nft_id: &str) -> Result<Option<String>, Error> {
        let result = sqlx::query!(
            r#"
            SELECT owner_id FROM nfts
            WHERE id = ?
            "#,
            nft_id
        )
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(result.map(|r| r.owner_id))
    }
    pub async fn run_migrations(pool: &SqlitePool) -> Result<(), Error> {
        println!("Running database migrations...");
    
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL
            )
            "#,
        )
        .execute(pool)
        .await?;
    
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS nfts (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                image_path TEXT NOT NULL,
                owner_id TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (owner_id) REFERENCES users(id)
            )
            "#,
        )
        .execute(pool)
        .await?;
    
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS transfers (
                id TEXT PRIMARY KEY,
                nft_id TEXT NOT NULL,
                from_user_id TEXT NOT NULL,
                to_user_id TEXT NOT NULL,
                transferred_at INTEGER NOT NULL,
                FOREIGN KEY (nft_id) REFERENCES nfts(id),
                FOREIGN KEY (from_user_id) REFERENCES users(id),
                FOREIGN KEY (to_user_id) REFERENCES users(id)
            )
            "#,
        )
        .execute(pool)
        .await?;
    
        let column_exists = sqlx::query!(
            "SELECT COUNT(*) as count FROM pragma_table_info('nfts') WHERE name = ?",
            "token_id"
        )
        .fetch_one(pool)
        .await?;
    
        if column_exists.count == 0 {
            println!("Adding blockchain columns to nfts table...");
    
            sqlx::query("ALTER TABLE nfts ADD COLUMN IF NOT EXISTS token_id TEXT")
                .execute(pool)
                .await?;
            sqlx::query("ALTER TABLE nfts ADD COLUMN IF NOT EXISTS ipfs_image_cid TEXT")
                .execute(pool)
                .await?;
            sqlx::query("ALTER TABLE nfts ADD COLUMN IF NOT EXISTS ipfs_metadata_cid TEXT")
                .execute(pool)
                .await?;
            sqlx::query("ALTER TABLE nfts ADD COLUMN IF NOT EXISTS blockchain_tx_hash TEXT")
                .execute(pool)
                .await?;
            
            println!("Blockchain columns added successfully");
        } else {
            println!("Blockchain columns already exist");
        }
        
        println!("Database migrations completed");
        Ok(())
    }
}