use ethers::{
    prelude::*,
    types::{Address, U256},
    contract::Contract,
};
use std::sync::Arc;
use std::str::FromStr;
use std::error::Error;
// use std::path::Path;

#[derive(Clone)]
pub struct BlockchainService {
    contract: Arc<Contract<SignerMiddleware<Provider<Http>, Wallet<k256::ecdsa::SigningKey>>>>,
    pub wallet_address: Address,
}

impl BlockchainService {
    pub async fn new(rpc_url: &str,contract_address: &str,private_key: &str) -> Result<Self, Box<dyn Error>> {
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
        
        // Load ABI from file
        let abi_json = include_str!("./MyNFT.json");
        let abi: ethers::abi::Abi = serde_json::from_str(abi_json)?;
        
        // Create the contract instance
        let contract = Arc::new(Contract::new(contract_addr, abi, client.clone()));
        
        Ok(Self {
            contract,
            wallet_address,
        })
    }
    
    pub async fn mint_nft(&self,recipient: &str,token_uri: &str) -> Result<(U256, String), Box<dyn Error>> {
        let recipient_addr = Address::from_str(recipient)?;
        
        // Fix: Store the method call in a variable before sending
        let method_call = self.contract.method::<_, U256>("mintNFT", (recipient_addr, token_uri.to_string()))?;
        let tx = method_call.send().await?;
        
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
    
    // Fix the transfer_nft method to match what's being called in main.rs
    pub async fn transfer_nft(&self,from_address: &str,to_address: &str,token_id: &str) -> Result<String, Box<dyn Error>> {
        // Convert the addresses and token ID from strings
        let from_addr = Address::from_str(from_address)?;
        let to_addr = Address::from_str(to_address)?;
        let token_id_u256 = U256::from_dec_str(token_id)?;
        
        // Fix: Store the method call in a variable before sending
        let method_call = self.contract.method::<_, ()>("transferFrom", (from_addr, to_addr, token_id_u256))?;
        let tx = method_call.send().await?;
        
        // Get the transaction hash
        let tx_hash = tx.tx_hash();
        
        // Wait for the transaction to be mined
        tx.await?
            .ok_or("Transaction failed to be mined")?;
            
        Ok(format!("{:?}", tx_hash))
    }

    // Add missing methods that are called in main.rs
    pub async fn get_token_id(&self, nft_id: &str) -> Result<Option<String>, Box<dyn Error>> {
        // In a real implementation, this would query the database or blockchain
        // For now, return a dummy token ID
        println!("Looking up token ID for NFT: {}", nft_id);
        Ok(Some("1".to_string()))
    }

    pub async fn get_user_wallet_address(&self, user_id: &str) -> Result<Option<String>, Box<dyn Error>> {
        // In a real implementation, this would query a user-to-wallet mapping
        // For now, return the wallet address from this service
        println!("Looking up wallet address for user: {}", user_id);
        Ok(Some(format!("{:?}", self.wallet_address)))
    