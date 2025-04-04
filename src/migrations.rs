use sqlx::{SqlitePool, Error};

pub async fn run_migrations(pool: &SqlitePool) -> Result<(), Error> {
    println!("Running database migrations...");

    // Check if users table exists
    let table_exists = sqlx::query!(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='users'"
    )
    .fetch_optional(pool)
    .await?
    .is_some();

    if !table_exists {
        println!("Creating users table...");
        // Create the users table with all required columns
        sqlx::query(r#"
            CREATE TABLE users (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                aadhaar_number TEXT UNIQUE,
                phone_number TEXT,
                email TEXT
            )
        "#).execute(pool).await?;
    } else {
        // Check and add columns if they don't exist
        let columns = ["aadhaar_number", "phone_number", "email"];
        
        for column in columns.iter() {
            let column_exists = sqlx::query!(
                "SELECT COUNT(*) as count FROM pragma_table_info('users') WHERE name = ?",
                column
            )
            .fetch_one(pool)
            .await?;
            
            if column_exists.count == 0 {
                println!("Adding {} column to users table...", column);
                let unique_constraint = if *column == "aadhaar_number" { "UNIQUE" } else { "" };
                let query = format!("ALTER TABLE users ADD COLUMN {} TEXT {}", column, unique_constraint);
                sqlx::query(&query).execute(pool).await?;
            }
        }
    }
    
    // Similar approach for the nfts table
    let table_exists = sqlx::query!(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='nfts'"
    )
    .fetch_optional(pool)
    .await?
    .is_some();

    if !table_exists {
        println!("Creating nfts table...");
        sqlx::query(r#"
            CREATE TABLE nfts (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                image_path TEXT NOT NULL,
                owner_id TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                token_id TEXT,
                ipfs_image_cid TEXT,
                ipfs_metadata_cid TEXT,
                blockchain_tx_hash TEXT,
                FOREIGN KEY (owner_id) REFERENCES users(id)
            )
        "#).execute(pool).await?;
    } else {
        // Check and add blockchain columns if they don't exist
        let columns = ["token_id", "ipfs_image_cid", "ipfs_metadata_cid", "blockchain_tx_hash"];
        
        for column in columns.iter() {
            let column_exists = sqlx::query!(
                "SELECT COUNT(*) as count FROM pragma_table_info('nfts') WHERE name = ?",
                column
            )
            .fetch_one(pool)
            .await?;
            
            if column_exists.count == 0 {
                println!("Adding {} column to nfts table...", column);
                let query = format!("ALTER TABLE nfts ADD COLUMN {} TEXT", column);
                sqlx::query(&query).execute(pool).await?;
            }
        }
    }
    
    // Check if transfers table exists
    let table_exists = sqlx::query!(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='transfers'"
    )
    .fetch_optional(pool)
    .await?
    .is_some();

    if !table_exists {
        println!("Creating transfers table...");
        sqlx::query(r#"
            CREATE TABLE transfers (
                id TEXT PRIMARY KEY,
                nft_id TEXT NOT NULL,
                from_user_id TEXT NOT NULL,
                to_user_id TEXT NOT NULL,
                transferred_at INTEGER NOT NULL,
                transaction_hash TEXT,
                property_data TEXT,
                FOREIGN KEY (nft_id) REFERENCES nfts(id),
                FOREIGN KEY (from_user_id) REFERENCES users(id),
                FOREIGN KEY (to_user_id) REFERENCES users(id)
            )
        "#).execute(pool).await?;
    } else {
        // Check and add columns if they don't exist
        let columns = ["transaction_hash", "property_data"];
        
        for column in columns.iter() {
            let column_exists = sqlx::query!(
                "SELECT COUNT(*) as count FROM pragma_table_info('transfers') WHERE name = ?",
                column
            )
            .fetch_one(pool)
            .await?;
            
            if column_exists.count == 0 {
                println!("Adding {} column to transfers table...", column);
                let query = format!("ALTER TABLE transfers ADD COLUMN {} TEXT", column);
                sqlx::query(&query).execute(pool).await?;
            }
        }
    }

    println!("Database migrations completed successfully");
    Ok(())
}