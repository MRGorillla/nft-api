use sqlx::{SqlitePool, Error};
use crate::models::{NFT, Transfer}; 
use crate::models::User;

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

    pub async f(&self, id: &str, name: &str, aadhaar_number: Option<&str>, phone_number: Option<&str>, email: Option<&str>, owner_id: &str) -> Result<(), Error> {
        sqlx::query!(r#"INSERT INTO users (id, name, aadhaar_number, phone_number, email, owner_id)VALUES (?, ?, ?, ?, ?, ?)"#,id,name,aadhaar_number,phone_number,email,owner_id).execute(&self.pool).await?;
        Ok(())
    }

    
    pub async fn get_user_by_aadhaar(&self, aadhaar_number: &str) -> Result<Option<User>, Error> {
        let result = sqlx::query!(r#"SELECT id as "id!", name as "name!", aadhaar_number,phone_number,email,owner_id FROM users WHERE aadhaar_number = ?"#,aadhaar_number).fetch_optional(&self.pool).await?;
        
        match result {
            Some(row) => Ok(Some(User {
                id: row.id,
                name: row.name,
                aadhaar_number: row.aadhaar_number,
                phone_number: row.phone_number,
                email: row.email,
                owner_id: row.owner_id,
            })),
            None => Ok(None)
        }
    }
    
    pub async fn get_user_by_id(&self, user_id: &str) -> Result<User, Error> {
        let row = sqlx::query!(r#"SELECT id as "id!", name as "name!",aadhaar_number,phone_number,email,owner_id FROM users WHERE id = ?"#,user_id).fetch_one(&self.pool).await?;
        Ok(User {
            id: row.id,
            name: row.name,
            aadhaar_number: row.aadhaar_number,
            phone_number: row.phone_number,
            email: row.email,
            owner_id: row.owner_id,
        })
    }
    pub async fn create_nft(&self,id: &str,name: &str,description: Option<&str>,image_path: &str,owner_id: &str,token_id: Option<&str>,ipfs_image_cid: Option<&str>,ipfs_metadata_cid: Option<&str>,blockchain_tx_hash: Option<&str>) -> Result<(), Error> {
        // Ensure blockchain columns exist
        let column_exists = sqlx::query!("SELECT COUNT(*) as count FROM pragma_table_info('nfts') WHERE name = ?","token_id").fetch_one(&self.p.await?;
    
        if column_exists.count == 0 {
            sqlx::query("ALTER TABLE nfts ADD COLUMN token_id TEXT").execute(&self.pool).await.ok();
            sqlx::query("ALTER TABLE nfts ADD COLUMN ipfs_image_cid TEXT").execute(&self.pool).await.ok();
            sqlx::query("ALTER TABLE nfts ADD COLUMN ipfs_metadata_cid TEXT").execute(&self.pool).await.ok();
            sqlx::query("ALTER TABLE nfts ADD COLUMN blockchain_tx_hash TEXT").execute(&self.pool).await.ok();
        }
    
        // Now try the insertion
        sqlx::query!(r#"INSERT INTO nfts (id, name, description, image_path, owner_id, created_at,token_id, ipfs_image_cid, ipfs_metadata_cid, blockchain_tx_hash)VALUES (?, ?, ?, ?, ?, strftime('%s', 'now'), ?, ?, ?, ?)"#,id,name,description,image_path,owner_id,token_id,ipfs_image_cid,ipfs_metadata_cid,blockchain_tx_h.execute(&self.p.await?;
        Ok(())
    }
    pub async fn get_nfts_by_owner(&self, owner_id: &str) -> Result<Vec<NFT>, Error> {
        // Fix the query to handle NULL fields properly and use direct type annotation
        let rows = sqlx::query!(r#"SELECT id as "id!", name as "name!", description, image_path as "image_path!", owner_id as "owner_id!",created_at as "created_at!: i64" FROM nfts WHERE owner_id = ? "#,owner_id).fetch_all(&self.pool).await?;

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
        // Change 'nft' to '_nft' to indicate it's intentionally unused
        let _nft = self.get_nft_by_id(nft_id).await?;
        
        // Store the transfer record
        sqlx::query("INSERT INTO transfers (id, nft_id, from_user_id, to_user_id, property_data, transaction_hash, transferred_at)  VALUES ($1, $2, $3, $4, $5, $6, $7)").bind(transfer_id).bind(nft_id).bind(from_user_id).bind(to_user_id).bind(property_data).bind(transaction_hash).bind(chrono::Local::now().naive_local()).execute(&self.pool).await?;
        
        // Update the NFT ownership
        sqlx::query("UPDATE nfts SET owner_id = $1 WHERE id = $2").bind(to_user_id).bind(nft_id).execute(&self.pool).await?;
        
        Ok(())
    }
    
    pub async fn get_nft_transfer_history(&self,nft_id: &str) -> Result<Vec<Transfer>, Box<dyn std::error::Error>> {
        let rows = sqlx::query!(r#"SELECT id as "id!", nft_id as "nft_id!", from_user_id as "from_user_id!", to_user_id as "to_user_id!",transferred_at as "transferred_at!: i64",transaction_hash,property_data FROM transfers WHERE nft_id = ? ORDER BY transferred_at DESC "#, nft_id).fetch_all(&self.pool).await?;
    
    let transfers = rows.into_iter().map(|row| {Transfer {id: row.id,nft_id: row.nft_id,from_user_id: row.from_user_id,to_user_id: row.to_user_id,transferred_at: chrono::DateTime::from_timestamp(row.transferred_at, 0).unwrap_or_else(|| chrono::Utc::now()).naive_utc(),transaction_hash: row.transaction_hash,property_data: row.property_data,}}).collect();
    Ok(transfers)
}

        pub async fn get_nft_by_id(&self, nft_id: &str) -> Result<NFT, Error> {
        let row = sqlx::query!(r#"SELECT id as "id!", name as "name!", description, image_path as "image_path!", owner_id as "owner_id!",created_at as "created_at!: i64" FROM nfts WHERE id = ? "#, nft_id).fetch_one(&self.pool).await?;
    
        Ok(NFT {id: row.id,name: row.name,description: row.description,image_path: row.image_path,owner_id: row.owner_id,created_at: chrono::DateTime::from_timestamp(row.created_at, 0).unwrap_or_else(|| chrono::Utc::now()).naive_utc()})
        }


    pub async fn get_user_transfer_history(&self,user_id: &str) -> Result<Vec<Transfer>, Box<dyn std::error::Error>> {
        let rows = sqlx::query!(r#"SELECT id as "id!", nft_id as "nft_id!", from_user_id as "from_user_id!", to_user_id as "to_user_id!",transferred_at as "transferred_at!: i64",transaction_hash,property_data FROM transfers WHERE from_user_id = ? OR to_user_id = ? ORDER BY transferred_at DESC "#, user_id, user_id).fetch_all(&self.pool).await?;
        
        let transfers = rows.into_iter().map(|row| {
            Transfer {id: row.id,nft_id: row.nft_id,from_user_id: row.from_user_id,to_user_id: row.to_user_id,transferred_at: chrono::DateTime::from_timestamp(row.transferred_at, 0).unwrap_or_else(|| chrono::Utc::now()).naive_utc(),transaction_hash: row.transaction_hash,property_data: row.property_data,}}).collect();
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
        let result = sqlx::query!(r#"SELECT owner_id FROM nftsWHERE id = ?"#,nft_id).fetch_optional(&self.pool).await?;
        Ok(result.map(|r| r.owner_id))
    }
    pub async fn run_migrations(pool: &SqlitePool) -> Result<(), Error> {
        println!("Running database migrations...");
    
        sqlx::query(r#"CREATE TABLE IF NOT EXISTS users (id TEXT PRIMARY KEY,name TEXT NOT aadhaar_number TEXT UNIQUE,phone_number TEXT,email TEXT)"#,).execute(pool).await?;
    
        sqlx::query(r#"CREATE TABLE IF NOT EXISTS nfts (id TEXT PRIMARY KEY,name TEXT NOT NULL,description TEXT,image_path TEXT NOT NULL,owner_id TEXT NOT NULL,created_at INTEGER NOT NULL,FOREIGN KEY (owner_id) REFERENCES users(id))"#,).execute(pool).await?;

        sqlx::query(r#"CREATE TABLE IF NOT EXISTS transfers (id TEXT PRIMARY KEY,nft_id TEXT NOT NULL,from_user_id TEXT NOT NULL,to_user_id TEXT NOT NULL,transferred_at INTEGER NOT NULL,FOREIGN KEY (nft_id) REFERENCES nfts(id),FOREIGN KEY (from_user_id) REFERENCES users(id),FOREIGN KEY (to_user_id) REFERENCES users(id))"#,).execute(pool).await?;
    
        let column_exists = sqlx::query!("SELECT COUNT(*) as count FROM pragma_table_info('nfts') WHERE name = ?","token_id").fetch_one(pool).await?;
    
        if column_exists.count == 0 {
            println!("Adding blockchain columns to nfts table...");
    
            sqlx::query("ALTER TABLE nfts ADD COLUMN IF NOT EXISTS token_id TEXT").execute(pool).await?;

            sqlx::query("ALTER TABLE nfts ADD COLUMN IF NOT EXISTS ipfs_image_cid TEXT").execute(pool).await?;

            sqlx::query("ALTER TABLE nfts ADD COLUMN IF NOT EXISTS ipfs_metadata_cid TEXT").execute(pool).await?;

            sqlx::query("ALTER TABLE nfts ADD COLUMN IF NOT EXISTS blockchain_tx_hash TEXT").execute(pool).await?;
            
            println!("Blockchain columns added successfully");
        } else {
            println!("Blockchain columns already exist");
        }
        
        let column_exists = sqlx::query!("SELECT COUNT(*) as count FROM pragma_table_info('users') WHERE name = ?","phone_number").fetch_one(pool).await?;
        
        if column_exists.count == 0 {
            println!("Adding phone_number column to users table...");
            sqlx::query("ALTER TABLE users ADD COLUMN phone_number TEXT").execute(pool).await?;
        }

        let column_exists = sqlx::query!("SELECT COUNT(*) as count FROM pragma_table_info('users') WHERE name = ?","aadhaar_number").fetch_one(pool).await?;
        
        if column_exists.count == 0 {
            println!("Adding aadhaar_number column to users table...");
            sqlx::query("ALTER TABLE users ADD COLUMN aadhaar_number TEXT UNIQUE").execute(pool).await?;
        }
        
        let column_exists = sqlx::query!("SELECT COUNT(*) as count FROM pragma_table_info('users') WHERE name = ?","email").fetch_one(pool).await?;
        
        if column_exists.count == 0 {
            println!("Adding email column to users table...");
            sqlx::query("ALTER TABLE users ADD COLUMN email TEXT").execute(pool).await?;
        }

        let column_exists = sqlx::query!("SELECT COUNT(*) as count FROM pragma_table_info('users') WHERE name = ?","owner_id").fetch_one(pool).await?;
        
        if column_exists.count == 0 {
            println!("Adding owner_id column to users table...");
            sqlx::query("ALTER TABLE users ADD COLUMN owner_id TEXT").execute(pool).await?;
        }


        println!("Database migrations completed");
        Ok(())
    }
    pub async fn get_token_id(&self, _nft_id: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        // In a real implementation, you would query your database to get the on-chain token ID
        // For this example, we'll return a dummy value
        Ok(Some("1".to_string()))
    }
    
    pub async fn get_user_wallet_address(&self, _user_id: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        // In a real implementation, you would lookup the user's wallet address
        // For now, we'll just return a dummy address
        Ok(Some("0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266".to_string()))
    }
}