use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use actix_multipart::Multipart;
use actix_cors::Cors; 
use futures::{StreamExt, TryStreamExt};
use serde_json::from_str;
use dotenv::dotenv;
use std::env;
use reqwest::Client as HttpClient;
use std::time::Duration;
use std::collections::HashMap;
use rand::{thread_rng, Rng};
use std::sync::Mutex;
use uuid::Uuid;
use chrono;

mod database;
use database::Database;

mod models;
use models::{User, NewUser, NewNFT, TransferRequest};
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
    otps: std::sync::Mutex<HashMap<String, String>>,
    http_client: HttpClient
}

// Implement your handler functions
async fn create_user(data: web::Data<AppState>,user: web::Json<NewUser>,) -> impl Responder {
    // Validate Aadhaar number (must be 12 digits)
    if let Some(ref aadhaar) = user.aadhaar_number {
        if aadhaar.len() != 12 || !aadhaar.chars().all(|c| c.is_digit(10)) {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "status": "error",
                "message": "Aadhaar number must be exactly 12 digits"
            }));
        }
    } else {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "status": "error",
            "message": "Aadhaar number is required"
        }));
    }
    
    // Validate phone number (must be a valid format)
    if let Some(ref phone) = user.phone_number {
        // Basic validation for Indian phone numbers (10 digits, optionally starting with +91)
        let valid_phone = phone.starts_with("+91") && phone.len() == 13 && phone[3..].chars().all(|c| c.is_digit(10))
            || phone.len() == 10 && phone.chars().all(|c| c.is_digit(10));
        
        if !valid_phone {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "status": "error",
                "message": "Invalid phone number format"
            }));
        }
    } else {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "status": "error",
            "message": "Phone number is required"
        }));
    }

    // Generate a new UUID for the user
    let user_id = Uuid::new_v4().to_string();
    
    // Generate a unique owner ID - using a prefix and a shortened UUID
    let owner_id = format!("OWN-{}", &Uuid::new_v4().to_string()[..8].to_uppercase());
    
    // Create the user in the database
    match data.db.create_user(&user_id, &user.name,user.aadhaar_number.as_deref(),user.phone_number.as_deref(),user.email.as_deref(),&owner_id).await {
        Ok(_) => {
            // Return the created user with its ID and generated owner ID
            HttpResponse::Created().json(serde_json::json!({
                "status": "success",
                "message": "User created successfully",
                "user": {
                    "id": user_id,
                    "name": user.name,
                    "aadhaar_number": user.aadhaar_number,
                    "phone_number": user.phone_number,
                    "email": user.email,
                    "owner_id": owner_id
                }
            }))
        },
        Err(e) => {
            // Check for duplicate Aadhaar error
            if e.to_string().contains("UNIQUE constraint failed") && e.to_string().contains("aadhaar_number") {
                HttpResponse::Conflict().json(serde_json::json!({
                    "status": "error",
                    "message": "A user with this Aadhaar number already exists"
                }))
            } else {
                HttpResponse::InternalServerError().json(serde_json::json!({
                    "status": "error",
                    "message": e.to_string()
                }))
            }
        },
    }
}

async fn send_sms(client: &HttpClient,to_number: &str, message: &str,account_sid: &str,auth_token: &str,from_number: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    println!("Sending SMS to {} with message: {}", to_number, message);
    
    // Format the phone number correctly (Twilio requires E.164 format)
    let formatted_number = if to_number.starts_with("+") {
        to_number.to_string()
    } else if to_number.len() == 10 {
        format!("+91{}", to_number) // Add India country code if 10 digits
    } else {
        return Err("Invalid phone number format".into());
    };
    
    // Build the Twilio API request
    let twilio_url = format!(
        "https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json",
        account_sid
    );
    
    let params = [
        ("To", formatted_number.as_str()),
        ("From", from_number),
        ("Body", message)
    ];
    let response = client.post(&twilio_url).basic_auth(account_sid, Some(auth_token)).form(&params).send().await?;
    if response.status().is_success() {
        Ok(true)
    } else {
        let error_text = response.text().await?;
        Err(format!("Twilio API error: {}", error_text).into())
    }
}

async fn send_otp(data: web::Data<AppState>,request: web::Json<serde_json::Value>,) -> impl Responder {
    println!("Received OTP request: {:?}", request);
    
    let aadhaar_number = match request.get("aadhaarNumber").and_then(|v| v.as_str()) {
        Some(id) => {
            println!("Found aadhaarNumber: {}", id);
            id
        },
        None => {
            println!("Missing aadhaarNumber in request");
            return HttpResponse::BadRequest().json(serde_json::json!({
                "status": "error", 
                "message": "Missing aadhaarNumber"
            }));
        },
    };
    
    // Check if user with this Aadhaar exists in the database
    match data.db.get_user_by_aadhaar(aadhaar_number).await {
        Ok(Some(user)) => {
            // Check if phone number is available
            if user.phone_number.is_none() {
                return HttpResponse::BadRequest().json(serde_json::json!({
                    "status": "error",
                    "message": "No phone number registered for this Aadhaar"
                }));
            }
            
            let phone = user.phone_number.unwrap();
            
            // Generate a 6-digit OTP
            let mut rng = thread_rng();
            let otp: String = (0..6)
                .map(|_| rng.gen_range(0..10).to_string())
                .collect();
            
            // Store the OTP with the user ID
            let mut otps = data.otps.lock().unwrap();
            otps.insert(aadhaar_number.to_string(), otp.clone());
            
            // Get Twilio credentials
            match (env::var("TWILIO_ACCOUNT_SID"), env::var("TWILIO_AUTH_TOKEN"), env::var("TWILIO_PHONE_NUMBER")) {
                (Ok(account_sid), Ok(auth_token), Ok(from_number)) => {
                    let message = format!("Your Propella verification OTP is: {}. Valid for 10 minutes.", otp);
                    // Send OTP via Twilio
                    match send_sms(&data.http_client, &phone, &message,&account_sid,&auth_token,&from_number).await {
                        Ok(_) => println!("SMS sent successfully to {}", phone),
                        Err(e) => {
                            // For development, also print the OTP to console if SMS fails
                            println!("Failed to send SMS: {}", e);
                            println!("OTP for Aadhaar {}: {} - Would be sent to {}", aadhaar_number, otp, phone);
                        }
                    };
                },
                _ => {
                    println!("Twilio credentials not found in environment variables");
                    println!("OTP for Aadhaar {}: {} - Would be sent to {}", aadhaar_number, otp, phone);
                }
            }
            
            // Create masked phone number to return to frontend (show last 4 digits)
            let masked_phone = if phone.len() > 4 {
                let visible_part = &phone[phone.len() - 4..];
                format!("XXXXXXXX{}", visible_part)
            } else {
                "XXXXXXXXXXXX".to_string()
            };
            
            HttpResponse::Ok().json(serde_json::json!({
                "status": "success",
                "message": "OTP sent successfully",
                "maskedPhone": masked_phone,
                "userId": user.id
            }))
        },
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
            "status": "error",
            "message": "No user found with this Aadhaar number"
        })),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

async fn verify_otp(data: web::Data<AppState>,request: web::Json<serde_json::Value>) -> impl Responder {
    let aadhaar_number = match request.get("aadhaarNumber").and_then(|v| v.as_str()) {
        Some(id) => id,
        None => return HttpResponse::BadRequest().body("Missing aadhaarNumber"),
    };
    
    let otp = match request.get("otp").and_then(|v| v.as_str()) {
        Some(otp) => otp,
        None => return HttpResponse::BadRequest().body("Missing otp"),
    };
    
    let mut otps = data.otps.lock().unwrap();
    if let Some(stored_otp) = otps.get(aadhaar_number) {
        if stored_otp == otp {
            // OTP matches - generate auth token
            let auth_token = Uuid::new_v4().to_string();
            
            // Remove the used OTP
            otps.remove(aadhaar_number);
            
            // Get user details
            match data.db.get_user_by_aadhaar(aadhaar_number).await {
                Ok(Some(user)) => {
                    HttpResponse::Ok().json(serde_json::json!({
                        "status": "success",
                        "token": auth_token,
                        "userId": user.id,
                        "userName": user.name
                    }))
                },
                Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
                    "status": "error",
                    "message": "User not found"
                })),
                Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
            }
        } else {
            HttpResponse::Unauthorized().json(serde_json::json!({
                "status": "error",
                "message": "Invalid OTP"
            }))
        }
    } else {
        HttpResponse::Unauthorized().json(serde_json::json!({
            "status": "error",
            "message": "No OTP request found for this Aadhaar number"
        }))
    }
}

async fn create_nft(data: web::Data<AppState>,mut payload: Multipart,) -> impl Responder 
{
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
                if let Ok(metadata_cid) = ipfs.upload_metadata(&nft_payload.name,nft_payload.description.as_deref(),&cid).await {
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
    match data.db.create_nft(&nft_id,&nft_payload.name,nft_payload.description.as_deref(),&image_path,&nft_payload.owner_id,token_id.as_deref(),ipfs_image_cid.as_deref(),ipfs_metadata_cid.as_deref(),blockchain_tx_hash.as_deref()).await {
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
async fn get_user_nfts(data: web::Data<AppState>, user_id: web::Path<String>) -> impl Responder {
    match data.db.get_nfts_by_owner(&user_id).await {
        Ok(nfts) => HttpResponse::Ok().json(nfts),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

async fn get_user(data: web::Data<AppState>,user_id: web::Path<String>) -> impl Responder {
    // Implement user retrieval logic here
    // For now, let's just return a simple response
    match data.db.user_exists(&user_id).await {
        Ok(true) => HttpResponse::Ok().body("User exists"),
        Ok(false) => HttpResponse::NotFound().body("User not found"),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

async fn transfer_nft(data: web::Data<AppState>,nft_id: web::Path<String>,transfer: web::Json<TransferRequest>) -> impl Responder {
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
    match data.db.transfer_nft(&transfer_id,&nft_id_str,&current_owner,&transfer.to_user_id,nft_data.as_deref(),tx_hash.as_deref()).await {
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
async fn get_nft_transfer_history(data: web::Data<AppState>,nft_id: web::Path<String>) -> impl Responder {
    match data.db.get_nft_transfer_history(&nft_id).await {
        Ok(transfers) => HttpResponse::Ok().json(transfers),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

async fn get_user_transfer_history(user_id: web::Path<String>,data: web::Data<AppState>) -> impl Responder {
    // Get all transfers where the user is either the sender or receiver
    match data.db.get_user_transfer_history(&user_id).await {
        Ok(transfers) => HttpResponse::Ok().json(transfers),
        Err(e) => {
            eprintln!("Error retrieving user transfer history: {}", e);
            HttpResponse::InternalServerError().body("Failed to retrieve transfer history")
        }
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
    
    // Initialize blockchain service 
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
    let http_client = HttpClient::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .unwrap_or_else(|_| HttpClient::new());

    HttpServer::new(move || {
        let cors = Cors::default().allow_any_origin() .allow_any_method().allow_any_header().max_age(3600);
        App::new().wrap(cors).app_data(web::Data::new(AppState {
                db: db.clone(),
                storage_path: storage_path.clone(),
                blockchain: blockchain.clone(),
                ipfs: ipfs.clone(),
                otps: std::sync::Mutex::new(HashMap::new()),
                http_client: http_client.clone(),
            }))
            // Routes remain the same
            .route("/users", web::post().to(create_user))
            .route("/users/{user_id}", web::get().to(get_user))
            .route("/nfts", web::post().to(create_nft))
            .route("/users/{user_id}/nfts", web::get().to(get_user_nfts))
            .route("/nfts/{nft_id}/transfer", web::post().to(transfer_nft))
            .route("/nfts/{nft_id}/transfers", web::get().to(get_nft_transfer_history))
            .route("/users/{user_id}/transfers", web::get().to(get_user_transfer_history))
            .route("/send-otp", web::post().to(send_otp))
            .route("/verify-otp", web::post().to(verify_otp))
    })
    .bind("127.0.0.1:30120")?
    .run()
    .await
}