use ethers::{
    prelude::*,
    types::{Address, U256},
};
use std::sync::Arc;
use std::str::FromStr;
use std::error::Error;
use std::path::Path;

// Include the ABI from the compiled contract
abigen!(
    MyNFT,
    "./src/MyNFT.json",
    event_derives(serde::Deserialize, serde::Serialize)
);

#[derive(Clone)]
pub struct BlockchainService {
    contract: ethers::contract::Contract<SignerMiddleware<Provider<Http>, Wallet<k256::ecdsa::SigningKey>>>,
    pub wallet_address: Address,
}

impl BlockchainService {
    pub async fn new(
        rpc_url: &str,
        contract_address: &str,
        private_key: &str,
    ) -> Result<Self, Box<dyn Error>> {
        // Connect to the network
        let provider = Provider::<Http>::try_from(rpc_url)?;
        
        // Set up the wallet
        let wallet: LocalWallet = private_key.parse::<LocalWallet>()?;
        let chain_id = provider.get_chainid().await?.as_u64();
        let wallet = wallet.with_chain_id(chain_id);
        let wallet_address = wallet.address();
        
        // Connect the wallet to the provider
        let client = Arc::new(SignerMiddleware::new(provider, wallet));
        
        // Connect to the contract
        let contract_addr = Address::from_str(contract_address)?;
        
        // Check if the ABI file exists
        let abi_path = Path::new("src/MyNFT.json");
        if !abi_path.exists() {
            return Err("ABI file not found at src/MyNFT.json. Please copy it from your blockchain artifacts.".into());
        }
        
        // Read the ABI from file
        let abi_str = std::fs::read_to_string(abi_path)?;
        let abi: ethers::abi::Abi = serde_json::from_str(&abi_str)?;
        
        // Create the contract
        let contract = ethers::contract::Contract::new(contract_addr, abi, client);
        
        Ok(Self {
            contract,
            wallet_address,
        })
    }
    
    pub async fn mint_nft(
        &self,
        recipient: &str,
        token_uri: &str,
    ) -> Result<(U256, String), Box<dyn Error>> {
        let recipient_addr = Address::from_str(recipient)?;
        
        // Call the mintNFT function using the dynamic approach
        let params = (recipient_addr, token_uri.to_string());
        let call = self.contract.method::<_, U256>("mintNFT", params)?;
        
        // Send the transaction
        let tx = call.send().await?;
        
        // Get the transaction hash
        let tx_hash = tx.tx_hash();
        
        // Wait for the transaction to be mined
        let receipt = tx.await?
            .ok_or("Transaction failed to be mined")?;
            
        // Parse the logs to get the token ID
        if let Some(logs) = receipt.logs.get(0) {
            if logs.topics.len() >= 4 {
                // The token ID is usually in the last topic
                let token_id = U256::from(logs.topics[3].as_fixed_bytes());
                return Ok((token_id, format!("{:?}", tx_hash)));
            }
        }
        
        Err("Failed to extract token ID from transaction logs".into())
    }
}