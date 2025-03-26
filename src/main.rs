use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use actix_multipart::Multipart;
use futures::{StreamExt, TryStreamExt};
use serde_json::from_str;
use dotenv::dotenv;
use std::env;
use uuid::Uuid;
use chrono;

mod database;
use database::Database;

mod models;
use models::{User, NewUser, NewNFT, TransferRequest};  // Removed unused Transfer import
mod blockchain;
mod ipfs;
use crate::blockchain::BlockchainService;
use crate::ipfs::IpfsStorage;
mod migrations;

struct AppState {
    db: Database,
    storage_path: String,
    blockchain: Option<BlockchainService>,
    ipfs: Option<IpfsStorage>,
}

// Implement your handler functions
async fn create_user(
    data: web::Data<AppState>,
    user: web::Json<NewUser>,
) -> impl Responder {
    let user_id = Uuid::new_v4().to_string();
    match data.db.create_user(&user_id, &user.name).await {
        Ok(_) => HttpResponse::Ok().json(User { id: user_id, name: user.name.clone() }),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

async fn create_nft(
    data: web::Data<AppState>,
    mut payload: Multipart,
) -> impl Responder {
    let mut nft_data: Option<NewNFT> = None;
    let mut image_data: Option<Vec<u8>> = None;
    
    // Extract data from multipart form
    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_disposition = field.content_disposition();
        if let Some(name) = content_disposition.get_name() {
            match name {
                "payload" => {
                    let mut payload_bytes = Vec::new();
                    while let Some(chunk_result) = field.next().await {
                        if let Ok(chunk) = chunk_result {
                            payload_bytes.extend_from_slice(&chunk);
                        }
                    }
                    if let Ok(payload_str) = std::str::from_utf8(&payload_bytes) {
                        if let Ok(parsed_nft) = from_str::<NewNFT>(payload_str) {
                            nft_data = Some(parsed_nft);
                        } else {
                            return HttpResponse::BadRequest().body("Invalid JSON payload");
                        }
                    }
                },
                "image" => {
                    let mut img = Vec::new();
                    while let Some(chunk_result) = field.next().await {
                        if let Ok(chunk) = chunk_result {
                            img.extend_from_slice(&chunk);
                        }
                    }
                    image_data = Some(img);
                },
                _ => {}
            }
        }
    }

    // Validate NFT data
    let nft_payload = match nft_data {
        Some(data) => data,
        None => return HttpResponse::BadRequest().body("Missing NFT metadata"),
    };

    // Verify owner exists
    let owner_id = &nft_payload.owner_id;
    match data.db.user_exists(owner_id).await {
        Ok(true) => {}, // User exists, proceed
        Ok(false) => return HttpResponse::BadRequest()
            .body(format!("User with ID '{}' does not exist", owner_id)),
        Err(e) => return HttpResponse::InternalServerError()
            .body(format!("Failed to verify user: {}", e.to_string())),
    }

    // Validate image data
    let image = match image_data {
        Some(data) => data,
        None => return HttpResponse::BadRequest().body("Missing image data"),
    };

    let nft_id = Uuid::new_v4().to_string();
    
    // Save image locally
    let image_path = format!("{}/{}.jpg", data.storage_path, nft_id);
    if let Err(e) = tokio::fs::write(&image_path, &image).await {
        return HttpResponse::InternalServerError().body(e.to_string());
    }
    
    // Variables for blockchain/IPFS data
    let mut token_id: Option<String> = None;
    let mut ipfs_image_cid: Option<String> = None;
    let mut ipfs_metadata_cid: Option<String> = None;
    let mut blockchain_tx_hash: Option<String> = None;
    
    // If IPFS service is available, upload the image
    if let Some(ref ipfs) = data.ipfs {
        match ipfs.upload_file(&image).await {
            Ok(cid) => {
                ipfs_image_cid = Some(cid.clone());
                println!("Image uploaded to IPFS with CID: {}", cid);
                
                // Create and upload metadata
                if let Ok(metadata_cid) = ipfs.upload_metadata(
                    &nft_payload.name,
                    nft_payload.description.as_deref(),
                    &cid
                ).await {
                    ipfs_metadata_cid = Some(metadata_cid.clone());
                    println!("Metadata uploaded to IPFS with CID: {}", metadata_cid);
                    
                    // If blockchain service is available, mint the NFT
                    if let Some(ref blockchain) = data.blockchain {
                        // The URI for the token metadata
                        let token_uri = ipfs.get_ipfs_uri(&metadata_cid);
                        
                        // Convert wallet address to string for the recipient
                        let recipient = blockchain.wallet_address.to_string();
                        
                        match blockchain.mint_nft(&recipient, &token_uri).await {
                            Ok((id, tx_hash)) => {
                                token_id = Some(id.to_string());
                                blockchain_tx_hash = Some(tx_hash.clone());
                                println!("NFT minted with token ID: {} and TX: {}", id, tx_hash);
                            },
                            Err(e) => {
                                eprintln!("Failed to mint NFT: {}", e);
                            }
                        }
                    }
                }
            },
            Err(e) => {
                eprintln!("Failed to upload image to IPFS: {}", e);
            }
        }
    }
    
    // Update your database schema to include the new fields
    // You might need to modify your database.rs to add these fields
    match data.db.create_nft(
        &nft_id,
        &nft_payload.name,
        nft_payload.description.as_deref(),
        &image_path,
        &nft_payload.owner_id,
        token_id.as_deref(),
        ipfs_image_cid.as_deref(),
        ipfs_metadata_cid.as_deref(),
        blockchain_tx_hash.as_deref(),
    ).await {
        Ok(_) => {
            // Create a valid timestamp
            let now = chrono::Utc::now().naive_utc();
            
            // Respond with the NFT information, including blockchain and IPFS data
            HttpResponse::Ok().json(serde_json::json!({
                "id": nft_id,
                "name": nft_payload.name,
                "description": nft_payload.description,
                "image_path": image_path,
                "owner_id": owner_id.to_string(),
                "created_at": now,
                "token_id": token_id,
                "ipfs_image_cid": ipfs_image_cid,
                "ipfs_metadata_cid": ipfs_metadata_cid,
                "blockchain_tx_hash": blockchain_tx_hash,
                "ipfs_gateway_url": ipfs_image_cid.as_ref().map(|cid| format!("https://ipfs.io/ipfs/{}", cid)),
            }))
        },
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

// Rest of your code remains the same
async fn get_user_nfts(
    data: web::Data<AppState>,
    user_id: web::Path<String>,
) -> impl Responder {
    match data.db.get_nfts_by_owner(&user_id).await {
        Ok(nfts) => HttpResponse::Ok().json(nfts),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

async fn get_user(
    data: web::Data<AppState>,
    user_id: web::Path<String>,
) -> impl Responder {
    // Implement user retrieval logic here
    // For now, let's just return a simple response
    match data.db.user_exists(&user_id).await {
        Ok(true) => HttpResponse::Ok().body("User exists"),
        Ok(false) => HttpResponse::NotFound().body("User not found"),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

async fn transfer_nft(
    data: web::Data<AppState>,
    nft_id: web::Path<String>,
    transfer: web::Json<TransferRequest>,
) -> impl Responder {
    let nft_id_str = nft_id.into_inner();
    
    // First get the current owner of the NFT
    let current_owner = match data.db.get_nft_owner(&nft_id_str).await {
        Ok(Some(owner_id)) => owner_id,
        Ok(None) => return HttpResponse::NotFound()
            .body(format!("NFT with ID '{}' not found", nft_id_str)),
        Err(e) => return HttpResponse::InternalServerError()
            .body(format!("Failed to get NFT owner: {}", e.to_string())),
    };
    
    // Make sure the recipient user exists
    match data.db.user_exists(&transfer.to_user_id).await {
        Ok(true) => {}, // User exists, proceed
        Ok(false) => return HttpResponse::BadRequest()
            .body(format!("User with ID '{}' does not exist", transfer.to_user_id)),
        Err(e) => return HttpResponse::InternalServerError()
            .body(format!("Failed to verify user: {}", e.to_string())),
    }
    
    let transfer_id = Uuid::new_v4().to_string();
    
    // Get the NFT data to record in the transfer log
    let nft_data = match data.db.get_nft_by_id(&nft_id_str).await {
        Ok(nft) => serde_json::to_string(&nft).ok(),
        Err(_) => None,
    };
    
    // Handle blockchain transfer if available
    let mut tx_hash: Option<String> = None;
    if let Some(ref blockchain) = data.blockchain {
        if let Some(ref token_id) = blockchain.get_token_id(&nft_id_str).await.ok().flatten() {
            // Assuming you have wallet addresses for users
            if let (Some(from_address), Some(to_address)) = (
                blockchain.get_user_wallet_address(&current_owner).await.ok().flatten(),
                blockchain.get_user_wallet_address(&transfer.to_user_id).await.ok().flatten()
            ) {
                match blockchain.transfer_nft(&from_address, &to_address, token_id).await {
                    Ok(hash) => {
                        let hash_clone = hash.clone(); // Clone before moving
                        tx_hash = Some(hash);
                        println!("NFT transferred on blockchain. TX hash: {}", hash_clone);
                    },
                    Err(e) => {
                        eprintln!("Blockchain transfer failed but will continue with database update: {}", e);
                    }
                }
            }
        }
    }
    
    // Do the transfer with the actual owner and record transaction details
    match data.db.transfer_nft(
        &transfer_id,
        &nft_id_str,
        &current_owner,
        &transfer.to_user_id,
        nft_data.as_deref(),
        tx_hash.as_deref(),
    ).await {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({
            "id": transfer_id,
            "nft_id": nft_id_str,
            "from_user_id": current_owner,
            "to_user_id": transfer.to_user_id,
            "transferred_at": chrono::Local::now().naive_local(),
            "transaction_hash": tx_hash,
            "status": "completed"
        })),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

// Add a new endpoint to get NFT transfer history
async fn get_nft_transfer_history(
    data: web::Data<AppState>,
    nft_id: web::Path<String>,
) -> impl Responder {
    match data.db.get_nft_transfer_history(&nft_id).await {
        Ok(transfers) => HttpResponse::Ok().json(transfers),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    let storage_path = env::var("STORAGE_PATH").unwrap_or_else(|_| "./nft_storage".to_string());
    tokio::fs::create_dir_all(&storage_path).await?;
    
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
        
    let db = Database::new(&database_url).await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    
    // Run migrations
    db.run_migrations_for_instance().await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    
    // Initialize blockchain service if env variables are set
    let blockchain = if let (Ok(rpc_url), Ok(contract_address)) = (
        env::var("ETH_RPC_URL"),
        env::var("NFT_CONTRACT_ADDRESS"),
    ) {
        let private_key = env::var("WALLET_PRIVATE_KEY")
            .unwrap_or("0xb4d59920ba76441bbfcf9e6f517528cb75dcf7542aa454b966f0aa85724383be".to_string());
        
        match BlockchainService::new(&rpc_url, &contract_address, &private_key).await {
            Ok(service) => {
                println!("Blockchain service initialized with wallet: {:?}", service.wallet_address);
                Some(service)
            }
            Err(e) => {
                eprintln!("Failed to initialize blockchain service: {}", e);
                None
            }
        }
    } else {
        println!("Blockchain service not configured, running in local-only mode");
        None
    };
    
    let ipfs = match IpfsStorage::new() {
        ipfs => {
            println!("IPFS service initialized");
            Some(ipfs)
        }
    };
    
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState {
                db: db.clone(),
                storage_path: storage_path.clone(),
                blockchain: blockchain.clone(),
                ipfs: ipfs.clone(),
            }))
            // Routes remain the same
            .route("/users", web::post().to(create_user))
            .route("/users/{user_id}", web::get().to(get_user))
            .route("/nfts", web::post().to(create_nft))
            .route("/users/{user_id}/nfts", web::get().to(get_user_nfts))
            .route("/nfts/{nft_id}/transfer", web::post().to(transfer_nft))
    })
    .bind("127.0.0.1:30120")?
    .run()
    .await
}