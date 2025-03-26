use dotenv::dotenv;
use std::env;
use sqlx::{sqlite::SqlitePoolOptions, Error, SqlitePool};
use std::path::PathBuf;
use std::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenv().ok();
    
    // Get current directory for reference
    let current_dir = env::current_dir()?;
    println!("Current working directory: {}", current_dir.display());
    
    // Create data directory using platform-specific path handling
    let mut data_path = PathBuf::from(current_dir);
    data_path.push("data");
    
    println!("Creating data directory: {}", data_path.display());
    fs::create_dir_all(&data_path)?;
    
    // Ensure the directory was actually created
    if !data_path.exists() {
        return Err(format!("Failed to create directory: {}", data_path.display()).into());
    }
    
    // Build database path using platform-specific handling
    let mut db_path = data_path.clone();
    db_path.push("nft.db");
    
    // Convert to SQLite connection string - use proper format with driver prefix
    // Use sqlite:// for file paths and add ?mode=rwc to ensure we can create/write
    let database_url = format!("sqlite://{}?mode=rwc", db_path.to_string_lossy());
    
    println!("Initializing database at: {}", database_url);
    
    // Try to connect with more robust error handling
    let pool = match SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await {
            Ok(p) => p,
            Err(e) => {
                println!("Failed to connect to database: {}", e);
                println!("Database path: {}", db_path.display());
                println!("Is path writable? {}", is_writable(&data_path));
                return Err(e.into());
            }
        };
    
    println!("Running migrations...");
    
    // Create users table
    println!("Creating users table...");
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL
        )
    "#).execute(&pool).await?;

    // Create nfts table with all required columns
    println!("Creating nfts table...");
    sqlx::query(r#"
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
    "#).execute(&pool).await?;

    // Create transfers table
    println!("Creating transfers table...");
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
    "#).execute(&pool).await?;

    println!("Database initialization complete!");
    Ok(())
}

// Helper function to check if a directory is writable
fn is_writable(path: &PathBuf) -> bool {
    match fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(path.join("write_test")) {
            Ok(_) => {
                // Clean up the test file
                let _ = fs::remove_file(path.join("write_test"));
                true
            },
            Err(_) => false,
        }
}