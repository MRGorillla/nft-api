use ipfs_api_backend_hyper::{IpfsClient, IpfsApi};
use std::error::Error;
use std::io::Cursor;
use serde_json::json;

#[derive(Clone)]
pub struct IpfsStorage {
    client: IpfsClient,
}

impl IpfsStorage {
    pub fn new() -> Self {
        // Create a default client (localhost:5001)
        let client = IpfsClient::default();
        Self { client }
    }

    pub async fn upload_file(&self, file_data: &[u8]) -> Result<String, Box<dyn Error>> {
        let cursor = Cursor::new(file_data.to_vec());
        let res = self.client.add(cursor).await?;
        Ok(res.hash)
    }

    pub async fn upload_metadata(
        &self,
        name: &str,
        description: Option<&str>,
        image_cid: &str
    ) -> Result<String, Box<dyn Error>> {
        let metadata = json!({
            "name": name,
            "description": description.unwrap_or(""),
            "image": format!("ipfs://{}", image_cid)
        });
        
        let metadata_str = serde_json::to_string(&metadata)?;
        let cursor = Cursor::new(metadata_str);
        let res = self.client.add(cursor).await?;
        
        Ok(res.hash)
    }

    pub fn get_ipfs_uri(&self, cid: &str) -> String {
        format!("ipfs://{}", cid)
    }
    
    pub fn get_ipfs_gateway_url(&self, cid: &str) -> String {
        format!("https://ipfs.io/ipfs/{}", cid)
    }
}