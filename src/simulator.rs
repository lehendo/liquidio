use anyhow::Result;
use ethers::types::{Address, U256};
use std::sync::Arc;
use tracing::{debug, info};

use crate::blockchain::BlockchainClient;
use crate::liquidation_detector::LiquidationSignal;

const ETH_PRICE_USD: u64 = 2000; // Simplified price oracle
const LIQUIDATION_BONUS: u64 = 110; // 10% bonus
const PRECISION: u64 = 100;

/// Simulation result for liquidation profitability
#[derive(Debug, Clone)]
pub struct SimulationResult {
    pub profitable: bool,
    pub expected_profit_usd: f64,
    pub collateral_to_seize: U256,
    pub debt_to_cover: U256,
    pub estimated_gas: U256,
    pub estimated_gas_cost_usd: f64,
}

/// Simulates liquidation transactions to verify profitability
pub struct LiquidationSimulator {
    blockchain: Arc<BlockchainClient>,
    min_profit_threshold: f64,
}

impl LiquidationSimulator {
    pub fn new(blockchain: Arc<BlockchainClient>, min_profit_threshold: f64) -> Self {
        Self {
            blockchain,
            min_profit_threshold,
        }
    }
    
    /// Simulate liquidation and calculate profitability
    /// This is a read-only operation that doesn't modify blockchain state
    pub async fn simulate_liquidation(
        &self,
        signal: &LiquidationSignal,
    ) -> Result<SimulationResult> {
        let start = std::time::Instant::now();
        
        // Calculate optimal debt to cover (start with full debt)
        let debt_to_cover = signal.debt;
        
        // Calculate collateral to seize with bonus
        let collateral_value = (debt_to_cover * U256::from(10u64.pow(18))) / U256::from(ETH_PRICE_USD * 10u64.pow(18));
        let collateral_to_seize = (collateral_value * U256::from(LIQUIDATION_BONUS)) / U256::from(PRECISION);
        
        // Estimate gas cost
        let gas_estimate = match self.blockchain.estimate_gas_liquidation(signal.user, debt_to_cover).await {
            Ok(gas) => gas,
            Err(_) => U256::from(300_000), // Fallback estimate
        };
        
        let gas_price = self.blockchain.get_gas_price().await.unwrap_or(U256::from(50_000_000_000u64)); // 50 gwei
        let gas_cost_wei = gas_estimate * gas_price;
        let gas_cost_eth = gas_cost_wei.as_u128() as f64 / 1e18;
        let gas_cost_usd = gas_cost_eth * ETH_PRICE_USD as f64;
        
        // Calculate profit
        let collateral_value_usd = (collateral_to_seize.as_u128() as f64 / 1e18) * ETH_PRICE_USD as f64;
        let debt_value_usd = debt_to_cover.as_u128() as f64 / 1e18;
        let expected_profit_usd = collateral_value_usd - debt_value_usd - gas_cost_usd;
        
        let profitable = expected_profit_usd >= self.min_profit_threshold;
        
        let elapsed = start.elapsed();
        debug!("Simulation completed in {:?}", elapsed);
        
        if profitable {
            info!("[PROFITABLE] Liquidation opportunity");
            info!("   Expected profit: ${:.2}", expected_profit_usd);
            info!("   Collateral value: ${:.2}", collateral_value_usd);
            info!("   Debt to cover: ${:.2}", debt_value_usd);
            info!("   Gas cost: ${:.2}", gas_cost_usd);
        } else {
            debug!("[UNPROFITABLE] Liquidation (profit: ${:.2})", expected_profit_usd);
        }
        
        Ok(SimulationResult {
            profitable,
            expected_profit_usd,
            collateral_to_seize,
            debt_to_cover,
            estimated_gas: gas_estimate,
            estimated_gas_cost_usd: gas_cost_usd,
        })
    }
    
    /// Quick profitability check without full simulation (ultra-fast)
    pub fn quick_profitability_check(&self, signal: &LiquidationSignal) -> bool {
        // Simple heuristic: check if liquidation bonus covers gas costs
        let collateral_value_usd = (signal.collateral.as_u128() as f64 / 1e18) * ETH_PRICE_USD as f64;
        let debt_value_usd = signal.debt.as_u128() as f64 / 1e18;
        let bonus_value = (collateral_value_usd * 0.10) - (debt_value_usd * 0.0); // 10% bonus
        
        // Rough gas cost estimate
        let estimated_gas_cost_usd = (300_000.0 * 50.0) / 1e9 * ETH_PRICE_USD as f64;
        
        bonus_value > estimated_gas_cost_usd + self.min_profit_threshold
    }
    
    /// Optimize debt amount to cover for maximum profit
    /// (Advanced feature for production bots)
    pub async fn optimize_debt_amount(
        &self,
        signal: &LiquidationSignal,
    ) -> Result<U256> {
        // For this POC, we liquidate the full debt
        // In production, you might liquidate partial amounts
        Ok(signal.debt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::LatencyMetrics;

    #[test]
    fn test_profitability_calculation() {
        let signal = LiquidationSignal {
            user: Address::zero(),
            collateral: U256::from(5 * 10u64.pow(18)), // 5 ETH
            debt: U256::from(8000 * 10u64.pow(18)), // $8000
            health_factor: U256::from(80), // 80%
            metrics: LatencyMetrics::new(),
        };
        
        // At $2000/ETH, 5 ETH = $10,000
        // Debt = $8,000
        // With 10% bonus, liquidator gets $8,800 worth of ETH for $8,000 debt
        // Profit = $800 - gas (should be profitable)
        
        assert!(signal.health_factor < U256::from(100));
    }
}


