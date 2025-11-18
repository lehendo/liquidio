use anyhow::{Context, Result};
use ethers::types::{Address, H256};
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub anvil_rpc_url: String,
    pub anvil_ws_url: String,
    pub chain_id: u64,
    pub lending_protocol_address: Address,
    pub mock_token_address: Address,
    pub liquidator_private_key: Option<H256>,
    pub min_profit_threshold_usd: f64,
    pub max_gas_price_gwei: u64,
    pub mempool_batch_size: usize,
    pub health_check_interval_ms: u64,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenv::dotenv().ok(); // Load .env file if it exists

        Ok(Config {
            anvil_rpc_url: env::var("ANVIL_RPC_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8545".to_string()),
            
            anvil_ws_url: env::var("ANVIL_WS_URL")
                .unwrap_or_else(|_| "ws://127.0.0.1:8545".to_string()),
            
            chain_id: env::var("CHAIN_ID")
                .unwrap_or_else(|_| "31337".to_string())
                .parse()
                .context("Invalid CHAIN_ID")?,
            
            lending_protocol_address: env::var("LENDING_PROTOCOL_ADDRESS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| Address::zero()),
            
            mock_token_address: env::var("MOCK_TOKEN_ADDRESS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| Address::zero()),
            
            liquidator_private_key: env::var("LIQUIDATOR_PRIVATE_KEY")
                .ok()
                .and_then(|s| s.parse().ok()),
            
            min_profit_threshold_usd: env::var("MIN_PROFIT_THRESHOLD_USD")
                .unwrap_or_else(|_| "10.0".to_string())
                .parse()
                .context("Invalid MIN_PROFIT_THRESHOLD_USD")?,
            
            max_gas_price_gwei: env::var("MAX_GAS_PRICE_GWEI")
                .unwrap_or_else(|_| "100".to_string())
                .parse()
                .context("Invalid MAX_GAS_PRICE_GWEI")?,
            
            mempool_batch_size: env::var("MEMPOOL_BATCH_SIZE")
                .unwrap_or_else(|_| "100".to_string())
                .parse()
                .context("Invalid MEMPOOL_BATCH_SIZE")?,
            
            health_check_interval_ms: env::var("HEALTH_CHECK_INTERVAL_MS")
                .unwrap_or_else(|_| "100".to_string())
                .parse()
                .context("Invalid HEALTH_CHECK_INTERVAL_MS")?,
        })
    }

    pub fn validate(&self) -> Result<()> {
        if self.lending_protocol_address == Address::zero() {
            anyhow::bail!("LENDING_PROTOCOL_ADDRESS not set");
        }
        if self.mock_token_address == Address::zero() {
            anyhow::bail!("MOCK_TOKEN_ADDRESS not set");
        }
        Ok(())
    }
}


