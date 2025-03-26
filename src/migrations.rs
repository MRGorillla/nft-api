use sqlx::{SqlitePool, Error};

pub async fn run_migrations(pool: &SqlitePool) -> Result<(), Error> {
    println!("Running database migrations...");

    // Create tables if they don't exist
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL
        )
    "#).execute(pool).await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS nfts (
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
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(r#"
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
    "#).execute(pool).await?;

    println!("Database migrations completed");
    Ok(())
}