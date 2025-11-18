use anyhow::Result;
use ethers::types::{Address, Transaction};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::blockchain::BlockchainClient;
use crate::liquidation_detector::LiquidationDetector;
use crate::simulator::LiquidationSimulator;
use crate::executor::LiquidationExecutor;
use crate::mempool_streamer::{MempoolStreamer, TransactionClassifier};
use crate::metrics::{LatencyMetrics, AggregateMetrics};

/// Backtesting framework for validating liquidation strategy
pub struct BacktestEngine {
    blockchain: Arc<BlockchainClient>,
    detector: Arc<LiquidationDetector>,
    simulator: Arc<LiquidationSimulator>,
    executor: Arc<LiquidationExecutor>,
    protocol_address: Address,
}

impl BacktestEngine {
    pub fn new(
        blockchain: Arc<BlockchainClient>,
        detector: Arc<LiquidationDetector>,
        simulator: Arc<LiquidationSimulator>,
        executor: Arc<LiquidationExecutor>,
        protocol_address: Address,
    ) -> Self {
        Self {
            blockchain,
            detector,
            simulator,
            executor,
            protocol_address,
        }
    }
    
    /// Run backtest with synthetic transaction stream
    pub async fn run_backtest(&self, num_transactions: usize) -> Result<AggregateMetrics> {
        info!("Starting backtest with {} transactions", num_transactions);
        
        let mut aggregate_metrics = AggregateMetrics::new();
        
        // Create mempool streamer
        let (streamer, mut rx) = MempoolStreamer::new(self.protocol_address);
        
        // Start streaming transactions in background
        let streamer_handle = tokio::spawn(async move {
            streamer.start_simulation(num_transactions).await
        });
        
        // Process transactions
        let mut processed = 0;
        let mut liquidations_found = 0;
        
        while let Some(tx) = rx.recv().await {
            processed += 1;
            
            if processed % 10000 == 0 {
                info!("Processed {} / {} transactions", processed, num_transactions);
            }
            
            // Detect liquidation opportunity
            match self.detector.process_transaction(&tx, self.protocol_address).await {
                Ok(Some(mut signal)) => {
                    liquidations_found += 1;
                    
                    // Mark simulation start
                    signal.metrics.mark_signal();
                    
                    // Simulate liquidation
                    match self.simulator.simulate_liquidation(&signal).await {
                        Ok(sim_result) => {
                            signal.metrics.mark_simulated();
                            
                            if sim_result.profitable {
                                // Execute (simulated)
                                signal.metrics.mark_constructed();
                                signal.metrics.mark_sent();
                                
                                aggregate_metrics.record_attempt(&signal.metrics, true);
                            } else {
                                aggregate_metrics.record_attempt(&signal.metrics, false);
                            }
                        }
                        Err(e) => {
                            warn!("Simulation failed: {}", e);
                            aggregate_metrics.record_attempt(&signal.metrics, false);
                        }
                    }
                }
                Ok(None) => {
                    // No liquidation opportunity
                }
                Err(e) => {
                    warn!("Detection error: {}", e);
                }
            }
        }
        
        // Wait for streamer to complete
        let _ = streamer_handle.await;
        
        info!("[OK] Backtest complete");
        info!("   Transactions processed: {}", processed);
        info!("   Liquidation opportunities found: {}", liquidations_found);
        info!("   Detection rate: {:.2}%", (liquidations_found as f64 / processed as f64) * 100.0);
        
        Ok(aggregate_metrics)
    }
    
    /// Run focused stress test for latency measurement
    pub async fn run_latency_stress_test(&self, iterations: usize) -> Result<AggregateMetrics> {
        info!("Running latency stress test ({} iterations)", iterations);
        
        let mut aggregate_metrics = AggregateMetrics::new();
        
        // Create test user with liquidatable position
        let test_user = Address::random();
        
        for i in 0..iterations {
            let mut metrics = LatencyMetrics::new();
            
            // Simulate detection
            metrics.mark_decoded();
            
            // Create synthetic liquidation signal
            let signal = crate::liquidation_detector::LiquidationSignal {
                user: test_user,
                collateral: ethers::types::U256::from(5 * 10u64.pow(18)), // 5 ETH
                debt: ethers::types::U256::from(8000 * 10u64.pow(18)), // $8000
                health_factor: ethers::types::U256::from(80), // 80%
                metrics: metrics.clone(),
            };
            
            metrics.mark_signal();
            
            // Simulate liquidation
            match self.simulator.simulate_liquidation(&signal).await {
                Ok(sim_result) => {
                    metrics.mark_simulated();
                    
                    if sim_result.profitable {
                        metrics.mark_constructed();
                        metrics.mark_sent();
                        aggregate_metrics.record_attempt(&metrics, true);
                    } else {
                        aggregate_metrics.record_attempt(&metrics, false);
                    }
                }
                Err(e) => {
                    warn!("Simulation failed: {}", e);
                    aggregate_metrics.record_attempt(&metrics, false);
                }
            }
            
            if (i + 1) % 1000 == 0 {
                info!("Completed {} / {} iterations", i + 1, iterations);
            }
        }
        
        info!("[OK] Stress test complete");
        
        Ok(aggregate_metrics)
    }
    
    /// Generate performance report
    pub async fn generate_report(
        &self,
        metrics: &AggregateMetrics,
        filename: &str,
    ) -> Result<()> {
        info!("Generating performance report: {}", filename);
        
        // Print summary to console
        metrics.print_summary();
        
        // Export to CSV
        let csv_filename = format!("{}.csv", filename);
        metrics.export_to_csv(&csv_filename)?;
        
        // Export to JSON
        let json_filename = format!("{}.json", filename);
        let json_data = serde_json::to_string_pretty(metrics)?;
        std::fs::write(&json_filename, json_data)?;
        
        info!("[OK] Report generated successfully");
        info!("   CSV: {}", csv_filename);
        info!("   JSON: {}", json_filename);
        
        // Validate <10ms target
        if let Some(p99) = metrics.percentile("end_to_end_us", 99.0) {
            let p99_ms = p99 / 1000.0;
            if p99_ms < 10.0 {
                info!("SUCCESS: P99 latency ({:.2}ms) is below 10ms target!", p99_ms);
            } else {
                warn!("WARNING: P99 latency ({:.2}ms) exceeds 10ms target", p99_ms);
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires full setup
    async fn test_backtest_engine() {
        // This would require a full blockchain setup
        // Left as integration test
    }
}


