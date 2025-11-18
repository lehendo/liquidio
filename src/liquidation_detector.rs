use anyhow::Result;
use ethers::types::{Address, U256, Transaction};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::blockchain::BlockchainClient;
use crate::mempool_streamer::{TransactionClassifier, TransactionType};
use crate::metrics::LatencyMetrics;

const LIQUIDATION_THRESHOLD: u64 = 100; // 100% = HF < 1.0

/// Position tracker for users in the lending protocol
#[derive(Debug, Clone, Default)]
pub struct UserPosition {
    pub collateral: U256,
    pub debt: U256,
    pub health_factor: U256,
    pub last_updated: u64,
}

/// Liquidation opportunity signal
#[derive(Debug, Clone)]
pub struct LiquidationSignal {
    pub user: Address,
    pub collateral: U256,
    pub debt: U256,
    pub health_factor: U256,
    pub metrics: LatencyMetrics,
}

/// Detects liquidation opportunities by monitoring user positions
pub struct LiquidationDetector {
    blockchain: Arc<BlockchainClient>,
    positions: Arc<RwLock<HashMap<Address, UserPosition>>>,
}

impl LiquidationDetector {
    pub fn new(blockchain: Arc<BlockchainClient>) -> Self {
        Self {
            blockchain,
            positions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Process incoming transaction and check for liquidation opportunities
    /// This is the core O(1) detection logic
    pub async fn process_transaction(
        &self,
        tx: &Transaction,
        protocol_address: Address,
    ) -> Result<Option<LiquidationSignal>> {
        let mut metrics = LatencyMetrics::new();
        
        // Quick filter: only process protocol transactions
        if !TransactionClassifier::is_protocol_transaction(tx, protocol_address) {
            return Ok(None);
        }
        
        // Classify transaction type
        let tx_type = match TransactionClassifier::classify_transaction(tx) {
            Some(t) => t,
            None => return Ok(None),
        };
        
        metrics.mark_decoded();
        
        // Only check positions for transactions that change collateral/debt
        match tx_type {
            TransactionType::Deposit | 
            TransactionType::Withdraw | 
            TransactionType::Borrow | 
            TransactionType::Repay => {
                let user = TransactionClassifier::extract_user_address(tx);
                
                // Update position from blockchain (in production, use events for efficiency)
                if let Err(e) = self.update_position(user).await {
                    warn!("Failed to update position for {}: {}", user, e);
                    return Ok(None);
                }
                
                // O(1) check: is this position liquidatable?
                let signal = self.check_liquidation(user, &mut metrics).await?;
                
                if signal.is_some() {
                    metrics.mark_signal();
                }
                
                Ok(signal)
            }
            TransactionType::Liquidate => {
                // Someone else is liquidating, update our tracking
                let user = TransactionClassifier::extract_user_address(tx);
                let _ = self.update_position(user).await;
                Ok(None)
            }
        }
    }
    
    /// Update position data from blockchain (O(1) operation)
    async fn update_position(&self, user: Address) -> Result<()> {
        let (collateral, debt, health_factor) = self.blockchain.get_position(user).await?;
        
        let position = UserPosition {
            collateral,
            debt,
            health_factor,
            last_updated: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        
        let mut positions = self.positions.write().await;
        positions.insert(user, position);
        
        debug!("Updated position for {}: collateral={}, debt={}, HF={}", 
            user, collateral, debt, health_factor);
        
        Ok(())
    }
    
    /// O(1) check if position is liquidatable
    async fn check_liquidation(
        &self,
        user: Address,
        metrics: &mut LatencyMetrics,
    ) -> Result<Option<LiquidationSignal>> {
        let positions = self.positions.read().await;
        let position = match positions.get(&user) {
            Some(p) => p.clone(),
            None => return Ok(None),
        };
        drop(positions);
        
        // Check if health factor is below threshold
        if position.health_factor < U256::from(LIQUIDATION_THRESHOLD) && position.debt > U256::zero() {
            info!("[LIQUIDATION OPPORTUNITY] Detected for {}", user);
            info!("   Collateral: {} ETH", position.collateral);
            info!("   Debt: {} USD", position.debt);
            info!("   Health Factor: {}", position.health_factor);
            
            metrics.mark_signal();
            
            return Ok(Some(LiquidationSignal {
                user,
                collateral: position.collateral,
                debt: position.debt,
                health_factor: position.health_factor,
                metrics: metrics.clone(),
            }));
        }
        
        Ok(None)
    }
    
    /// Bulk check all positions for liquidation opportunities (for backtesting)
    pub async fn scan_all_positions(&self) -> Result<Vec<LiquidationSignal>> {
        let mut signals = Vec::new();
        let positions = self.positions.read().await;
        
        for (user, position) in positions.iter() {
            if position.health_factor < U256::from(LIQUIDATION_THRESHOLD) && position.debt > U256::zero() {
                let mut metrics = LatencyMetrics::new();
                metrics.mark_signal();
                
                signals.push(LiquidationSignal {
                    user: *user,
                    collateral: position.collateral,
                    debt: position.debt,
                    health_factor: position.health_factor,
                    metrics,
                });
            }
        }
        
        Ok(signals)
    }
    
    /// Get number of tracked positions
    pub async fn get_position_count(&self) -> usize {
        self.positions.read().await.len()
    }
    
    /// Clear all tracked positions (for testing)
    pub async fn clear_positions(&self) {
        self.positions.write().await.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_tracking() {
        let position = UserPosition {
            collateral: U256::from(10u64.pow(18)), // 1 ETH
            debt: U256::from(1000 * 10u64.pow(18)), // 1000 USD
            health_factor: U256::from(150), // 150%
            last_updated: 0,
        };
        
        assert!(position.health_factor >= U256::from(LIQUIDATION_THRESHOLD));
    }
}


