use anyhow::Result;
use ethers::{
    prelude::*,
    types::{Address, U256, Eip1559TransactionRequest},
    signers::LocalWallet,
};
use std::sync::Arc;
use tracing::{info, warn, error};

use crate::blockchain::BlockchainClient;
use crate::liquidation_detector::LiquidationSignal;
use crate::simulator::SimulationResult;
use crate::metrics::LatencyMetrics;

/// Constructs and executes liquidation transactions
pub struct LiquidationExecutor {
    blockchain: Arc<BlockchainClient>,
    wallet: Option<LocalWallet>,
    max_gas_price_gwei: u64,
}

impl LiquidationExecutor {
    pub fn new(
        blockchain: Arc<BlockchainClient>,
        wallet: Option<LocalWallet>,
        max_gas_price_gwei: u64,
    ) -> Self {
        Self {
            blockchain,
            wallet,
            max_gas_price_gwei,
        }
    }
    
    /// Execute liquidation transaction with EIP-1559 gas optimization
    pub async fn execute_liquidation(
        &self,
        signal: &LiquidationSignal,
        simulation: &SimulationResult,
        mut metrics: LatencyMetrics,
    ) -> Result<H256> {
        let _wallet = match &self.wallet {
            Some(w) => w,
            None => {
                warn!("No wallet configured, skipping execution");
                return Err(anyhow::anyhow!("No wallet configured"));
            }
        };
        
        info!("Executing liquidation for user {}", signal.user);
        
        // Construct transaction
        let tx_request = self.build_liquidation_transaction(
            signal.user,
            simulation.debt_to_cover,
        ).await?;
        
        metrics.mark_constructed();
        
        // For POC: we log the transaction instead of actually sending it
        // In production with real funds, you would send via private relay (Flashbots)
        info!("Transaction constructed:");
        info!("   To: {:?}", tx_request.to);
        info!("   Value: {:?}", tx_request.value);
        info!("   Gas limit: {:?}", tx_request.gas);
        info!("   Max fee per gas: {:?}", tx_request.max_fee_per_gas);
        info!("   Max priority fee: {:?}", tx_request.max_priority_fee_per_gas);
        
        metrics.mark_sent();
        
        // Calculate latencies
        let latencies = metrics.get_all_latencies();
        info!("Latency breakdown:");
        if let Some(e2e) = latencies.get("end_to_end_us") {
            info!("   End-to-end: {:.2} μs ({:.2} ms)", e2e, e2e / 1000.0);
        }
        if let Some(sig) = latencies.get("signal_detection_us") {
            info!("   Signal detection: {:.2} μs", sig);
        }
        if let Some(sim) = latencies.get("simulation_us") {
            info!("   Simulation: {:.2} μs", sim);
        }
        
        // Return a mock transaction hash for POC
        let mock_hash = H256::random();
        info!("[OK] Liquidation executed (simulated): {:?}", mock_hash);
        
        Ok(mock_hash)
    }
    
    /// Build EIP-1559 transaction with optimized gas pricing
    async fn build_liquidation_transaction(
        &self,
        user: Address,
        debt_to_cover: U256,
    ) -> Result<Eip1559TransactionRequest> {
        // Get current base fee
        let gas_price = self.blockchain.get_gas_price().await?;
        
        // Calculate EIP-1559 fees
        let base_fee = gas_price;
        let max_priority_fee = U256::from(2_000_000_000u64); // 2 gwei tip
        let max_fee_per_gas = base_fee * 2 + max_priority_fee; // 2x base fee + tip
        
        // Cap at max gas price
        let max_allowed = U256::from(self.max_gas_price_gwei) * U256::from(1_000_000_000u64);
        let max_fee_per_gas = std::cmp::min(max_fee_per_gas, max_allowed);
        
        // Encode liquidate function call
        let protocol_address = self.blockchain.lending_protocol.address();
        let call_data = self.encode_liquidate_call(user, debt_to_cover);
        
        let tx = Eip1559TransactionRequest::new()
            .to(protocol_address)
            .data(call_data)
            .gas(U256::from(350_000)) // Gas limit
            .max_fee_per_gas(max_fee_per_gas)
            .max_priority_fee_per_gas(max_priority_fee)
            .chain_id(31337);
        
        Ok(tx)
    }
    
    /// Encode liquidate(address user, uint256 debtToCover) function call
    fn encode_liquidate_call(&self, user: Address, debt_to_cover: U256) -> Bytes {
        // liquidate(address,uint256) selector: 0x26cdbe1a
        let mut data = hex::decode("26cdbe1a").unwrap();
        
        // Encode address (padded to 32 bytes)
        let mut user_bytes = [0u8; 32];
        user_bytes[12..32].copy_from_slice(user.as_bytes());
        data.extend_from_slice(&user_bytes);
        
        // Encode uint256
        let mut amount_bytes = [0u8; 32];
        debt_to_cover.to_big_endian(&mut amount_bytes);
        data.extend_from_slice(&amount_bytes);
        
        Bytes::from(data)
    }
    
    /// Submit transaction via private relay (Flashbots simulation)
    /// In production, this would send to actual Flashbots relay
    pub async fn submit_via_private_relay(
        &self,
        _tx: Eip1559TransactionRequest,
    ) -> Result<H256> {
        info!("Submitting to private relay (simulated)");
        info!("   In production, this would use Flashbots RPC");
        
        // Simulate successful submission
        Ok(H256::random())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_liquidate_call_encoding() {
        let executor = LiquidationExecutor::new(
            Arc::new(BlockchainClient::new(
                "http://127.0.0.1:8545",
                None,
                Address::zero(),
                Address::zero(),
            ).await.unwrap()),
            None,
            100,
        );
        
        let user = Address::from_low_u64_be(1);
        let debt = U256::from(1000);
        let encoded = executor.encode_liquidate_call(user, debt);
        
        // Check selector
        assert_eq!(&encoded[..4], &hex::decode("26cdbe1a").unwrap());
    }
}

