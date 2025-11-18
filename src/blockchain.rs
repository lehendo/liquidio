use anyhow::Result;
use ethers::{
    providers::{Provider, Ws, Http, Middleware},
    types::{Block, Transaction, TransactionReceipt, Address, U256, H256},
    contract::abigen,
};
use std::sync::Arc;
use tracing::{debug, info};

// Generate contract bindings
abigen!(
    LendingProtocol,
    r#"[
        function deposit() external payable
        function withdraw(uint256 amount) external
        function borrow(uint256 amount) external
        function repay(uint256 amount) external
        function liquidate(address user, uint256 debtToCover) external
        function getHealthFactor(address user) external view returns (uint256)
        function isLiquidatable(address user) external view returns (bool)
        function getPosition(address user) external view returns (uint256 collateral, uint256 debt, uint256 healthFactor)
        event Deposit(address indexed user, uint256 amount)
        event Withdraw(address indexed user, uint256 amount)
        event Borrow(address indexed user, uint256 amount)
        event Repay(address indexed user, uint256 amount)
        event Liquidate(address indexed liquidator, address indexed user, uint256 debtRepaid, uint256 collateralSeized)
    ]"#
);

abigen!(
    ERC20,
    r#"[
        function approve(address spender, uint256 amount) external returns (bool)
        function transfer(address to, uint256 amount) external returns (bool)
        function balanceOf(address account) external view returns (uint256)
        function allowance(address owner, address spender) external view returns (uint256)
    ]"#
);

pub type HttpProvider = Provider<Http>;
pub type WsProvider = Provider<Ws>;

pub struct BlockchainClient {
    pub http_provider: Arc<HttpProvider>,
    pub ws_provider: Option<Arc<WsProvider>>,
    pub lending_protocol: LendingProtocol<HttpProvider>,
    pub token: ERC20<HttpProvider>,
}

impl BlockchainClient {
    pub async fn new(
        rpc_url: &str,
        ws_url: Option<&str>,
        protocol_address: Address,
        token_address: Address,
    ) -> Result<Self> {
        info!("Connecting to blockchain at {}", rpc_url);
        
        let http_provider = Provider::<Http>::try_from(rpc_url)?;
        let http_provider = Arc::new(http_provider);
        
        let ws_provider = if let Some(ws_url) = ws_url {
            debug!("Connecting WebSocket at {}", ws_url);
            let provider = Provider::<Ws>::connect(ws_url).await?;
            Some(Arc::new(provider))
        } else {
            None
        };
        
        let lending_protocol = LendingProtocol::new(protocol_address, http_provider.clone());
        let token = ERC20::new(token_address, http_provider.clone());
        
        info!("Blockchain client initialized");
        
        Ok(Self {
            http_provider,
            ws_provider,
            lending_protocol,
            token,
        })
    }
    
    pub async fn get_block_number(&self) -> Result<u64> {
        let block_num = self.http_provider.get_block_number().await?;
        Ok(block_num.as_u64())
    }
    
    pub async fn get_block(&self, block_number: u64) -> Result<Option<Block<H256>>> {
        Ok(self.http_provider.get_block(block_number).await?)
    }
    
    pub async fn get_transaction(&self, tx_hash: H256) -> Result<Option<Transaction>> {
        Ok(self.http_provider.get_transaction(tx_hash).await?)
    }
    
    pub async fn get_transaction_receipt(&self, tx_hash: H256) -> Result<Option<TransactionReceipt>> {
        Ok(self.http_provider.get_transaction_receipt(tx_hash).await?)
    }
    
    pub async fn get_health_factor(&self, user: Address) -> Result<U256> {
        Ok(self.lending_protocol.get_health_factor(user).call().await?)
    }
    
    pub async fn is_liquidatable(&self, user: Address) -> Result<bool> {
        Ok(self.lending_protocol.is_liquidatable(user).call().await?)
    }
    
    pub async fn get_position(&self, user: Address) -> Result<(U256, U256, U256)> {
        Ok(self.lending_protocol.get_position(user).call().await?)
    }
    
    pub async fn get_gas_price(&self) -> Result<U256> {
        Ok(self.http_provider.get_gas_price().await?)
    }
    
    pub async fn estimate_gas_liquidation(
        &self,
        user: Address,
        debt_to_cover: U256,
    ) -> Result<U256> {
        let call = self.lending_protocol.liquidate(user, debt_to_cover);
        Ok(call.estimate_gas().await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires running Anvil instance
    async fn test_blockchain_connection() {
        let client = BlockchainClient::new(
            "http://127.0.0.1:8545",
            None,
            Address::zero(),
            Address::zero(),
        )
        .await;
        
        assert!(client.is_ok());
    }
}

