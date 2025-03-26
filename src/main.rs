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
use models::{User, NewUser, NFT, NewNFT, Transfer, TransferRequest};

struct AppState {
    db: Database,
    storage_path: String,
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

    let nft_payload = match nft_data {
        Some(data) => data,
        None => return HttpResponse::BadRequest().body("Missing NFT metadata"),
    };

    let owner_id = &nft_payload.owner_id;
    match data.db.user_exists(owner_id).await {
        Ok(true) => {}, // User exists, proceed
        Ok(false) => return HttpResponse::BadRequest()
            .body(format!("User with ID '{}' does not exist", owner_id)),
        Err(e) => return HttpResponse::InternalServerError()
            .body(format!("Failed to verify user: {}", e.to_string())),
    }


    let image = match image_data {
        Some(data) => data,
        None => return HttpResponse::BadRequest().body("Missing image data"),
    };

    let nft_id = Uuid::new_v4().to_string();

    
    let image_path = format!("{}/{}.jpg", data.storage_path, nft_id);
    if let Err(e) = tokio::fs::write(&image_path, &image).await {
        return HttpResponse::InternalServerError().body(e.to_string());
    }
    
    match data.db.create_nft(
        &nft_id,
        &nft_payload.name,
        nft_payload.description.as_deref(),
        &image_path,
        &owner_id,
    ).await {
        Ok(_) => HttpResponse::Ok().json(NFT {
            id: nft_id,
            name: nft_payload.name,
            description: nft_payload.description,
            image_path,
            owner_id: owner_id.to_string(),
            created_at: chrono::Local::now().naive_local(),
        }),
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

async fn transfer_nft(
    data: web::Data<AppState>,
    nft_id: web::Path<String>,
    transfer: web::Json<TransferRequest>,
) -> impl Responder {
    let from_user_id = "temp_owner";
    let transfer_id = Uuid::new_v4().to_string();
    
    match data.db.transfer_nft(
        &transfer_id,
        &nft_id,
        &from_user_id,
        &transfer.to_user_id,
    ).await {
        Ok(_) => HttpResponse::Ok().json(Transfer {
            id: transfer_id,
            nft_id: nft_id.into_inner(),
            from_user_id: from_user_id.to_string(),
            to_user_id: transfer.to_user_id.clone(),
            transferred_at: chrono::Local::now().naive_local(),
        }),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    
    let storage_path = "./nft_storage";
    tokio::fs::create_dir_all(storage_path).await?;
    
    let database_url = env::var("DATABASE_URL")
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        
    let db = Database::new(&database_url).await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState {
                db: db.clone(),
                storage_path: storage_path.to_string(),
            }))
            .route("/users", web::post().to(create_user))
            .route("/nfts", web::post().to(create_nft))
            .route("/users/{user_id}/nfts", web::get().to(get_user_nfts))
            .route("/nfts/{nft_id}/transfer", web::post().to(transfer_nft))
    })
    .bind("127.0.0.1:30120")?
    .run()
    .await
}