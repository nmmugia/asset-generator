use actix_web::{web, App, HttpResponse, HttpServer, Result};
use redis::Commands;
use tokio::fs::{File};
use std::fs::read_dir;
use tokio::io::AsyncReadExt;
use env_logger;
use uuid::Uuid;
use std::collections::HashMap;

extern crate redis;

async fn handle_assets_request(info: web::Path<(String, String)>) -> Result<HttpResponse> {
    let (key, filename) = info.into_inner();
    let file_path = format!("assets/{}", filename);

    let expected_api_key = std::env::var("EXPECTED_API_KEY")
        .expect("Expected API key not set in the environment");
        // Check if the key exists in Redis
    let mut redis_conn = redis::Client::open("redis://127.0.0.1/").expect("Failed to connect to redis");
    let key_exists: bool = redis_conn.exists(&key).expect("Failed to check key existence");
    if key_exists {
        if let Ok(mut file) = File::open(&file_path).await {
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)
                .await
                .expect("Failed to read file");

            Ok(HttpResponse::Ok()
                .content_type("image/jpeg") // Adjust content type as needed
                .body(buffer))
        } else {
            Ok(HttpResponse::NotFound().finish())
        }
    } else {
        // You can customize the response if the key is not found in Redis
        Ok(HttpResponse::NotFound().finish())
    }    

}

async fn sync_handler(
    info: web::Path<String>,
) -> Result<HttpResponse> {
    let key = info.into_inner();
    let mut response_data = Vec::new(); // Vector to store maps

    let expected_api_key = std::env::var("EXPECTED_API_KEY")
        .expect("Expected API key not set in the environment");
    if key == expected_api_key {
        // Traverse the "assets" folder and set keys in Redis with a TTL of 1 minute
        let assets_dir = "assets";
        let file_entries = match read_dir(assets_dir) {
            Ok(entries) => entries,
            Err(e) => {
                eprintln!("Failed to read assets folder: {}", e);
                return Ok(HttpResponse::InternalServerError().finish());
            }
        };

        let mut redis_conn = redis::Client::open("redis://127.0.0.1/").expect("Failed to connect to redis");

        for entry in file_entries {
            if let Ok(file_entry) = entry {
                let file_name = match file_entry.file_name().into_string() {
                    Ok(string) => {
                        string
                    },
                    Err(e) => {
                        return Ok(HttpResponse::InternalServerError().finish());
                    }
                };
                let uuid = Uuid::new_v4().to_string();
                // Set the Redis key-value pair with a TTL of 1 minute
                redis_conn
                    .set_ex::<&str, &str, bool>(&uuid, &file_name as &str, 60).expect("failed");
                // Create a HashMap and push it to the response_data vector
                let mut map = HashMap::new();
                map.insert("filename".to_string(), file_name.clone());
                map.insert("uuid".to_string(), uuid.clone());
                response_data.push(map);
            }
        }

        Ok(HttpResponse::Ok().json(response_data))
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    HttpServer::new(|| {
        App::new()
        .service(
            web::resource("/assets/{key}/{filename}")
                .route(web::get().to(handle_assets_request)),
        )
        .service(
            web::resource("/sync/{key}")
            .route(web::post().to(sync_handler)),
        )
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
