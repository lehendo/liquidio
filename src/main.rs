mod blockchain;
mod config;
mod liquidation_detector;
mod simulator;
mod executor;
mod mempool_streamer;
mod metrics;
mod backtesting;

use anyhow::Result;
use std::sync::Arc;
use tracing::{info, error};
use tracing_subscriber;

use crate::blockchain::BlockchainClient;
use crate::config::Config;
use crate::liquidation_detector::LiquidationDetector;
use crate::simulator::LiquidationSimulator;
use crate::executor::LiquidationExecutor;
use crate::backtesting::BacktestEngine;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    
    info!("Liquidio - Low-Latency DeFi Liquidation Bot");
    info!("================================================");
    
    // Load configuration
    let config = Config::from_env()?;
    info!("[OK] Configuration loaded");
    
    // Connect to blockchain
    let blockchain = Arc::new(
        BlockchainClient::new(
            &config.anvil_rpc_url,
            Some(&config.anvil_ws_url),
            config.lending_protocol_address,
            config.mock_token_address,
        )
        .await?
    );
    info!("[OK] Connected to blockchain");
    
    // Initialize components
    let detector = Arc::new(LiquidationDetector::new(blockchain.clone()));
    let simulator = Arc::new(LiquidationSimulator::new(
        blockchain.clone(),
        config.min_profit_threshold_usd,
    ));
    let executor = Arc::new(LiquidationExecutor::new(
        blockchain.clone(),
        None, // No wallet for simulation mode
        config.max_gas_price_gwei,
    )    );
    
    info!("[OK] Components initialized");
    
    // Create backtest engine
    let backtest_engine = BacktestEngine::new(
        blockchain.clone(),
        detector.clone(),
        simulator.clone(),
        executor.clone(),
        config.lending_protocol_address,
    );
    
    // Run backtesting suite
    info!("\nStarting Backtesting Suite");
    info!("==============================");
    
    // Test 1: Full transaction stream backtest
    info!("\nTest 1: Transaction Stream Backtest (50k transactions)");
    let metrics_1 = backtest_engine.run_backtest(50_000).await?;
    backtest_engine.generate_report(&metrics_1, "benchmark_results/transaction_stream_backtest").await?;
    
    // Test 2: Latency stress test
    info!("\nTest 2: Latency Stress Test (10k iterations)");
    let metrics_2 = backtest_engine.run_latency_stress_test(10_000).await?;
    backtest_engine.generate_report(&metrics_2, "benchmark_results/latency_stress_test").await?;
    
    // Final summary
    info!("\nAll tests complete!");
    info!("=====================");
    info!("Results saved to benchmark_results/");
    
    // Validate performance targets
    validate_performance_targets(&metrics_2)?;
    
    Ok(())
}

fn validate_performance_targets(metrics: &metrics::AggregateMetrics) -> Result<()> {
    info!("\nValidating Performance Targets");
    info!("==================================");
    
    let mut all_targets_met = true;
    
    // Target 1: End-to-end latency < 10ms (P99)
    if let Some(p99) = metrics.percentile("end_to_end_us", 99.0) {
        let p99_ms = p99 / 1000.0;
        let target_met = p99_ms < 10.0;
        info!("End-to-end latency (P99): {:.2}ms [Target: <10ms] {}", 
            p99_ms, if target_met { "[OK]" } else { "[FAIL]" });
        all_targets_met &= target_met;
    }
    
    // Target 2: Signal detection < 2ms (P99)
    if let Some(p99) = metrics.percentile("signal_detection_us", 99.0) {
        let p99_ms = p99 / 1000.0;
        let target_met = p99_ms < 2.0;
        info!("Signal detection (P99): {:.2}ms [Target: <2ms] {}", 
            p99_ms, if target_met { "[OK]" } else { "[FAIL]" });
        all_targets_met &= target_met;
    }
    
    // Target 3: Simulation < 5ms (P99)
    if let Some(p99) = metrics.percentile("simulation_us", 99.0) {
        let p99_ms = p99 / 1000.0;
        let target_met = p99_ms < 5.0;
        info!("Simulation (P99): {:.2}ms [Target: <5ms] {}", 
            p99_ms, if target_met { "[OK]" } else { "[FAIL]" });
        all_targets_met &= target_met;
    }
    
    // Target 4: Transaction construction < 1ms (P99)
    if let Some(p99) = metrics.percentile("construction_us", 99.0) {
        let p99_ms = p99 / 1000.0;
        let target_met = p99_ms < 1.0;
        info!("Transaction construction (P99): {:.2}ms [Target: <1ms] {}", 
            p99_ms, if target_met { "[OK]" } else { "[FAIL]" });
        all_targets_met &= target_met;
    }
    
    if all_targets_met {
        info!("\nALL PERFORMANCE TARGETS MET!");
    } else {
        info!("\nSome performance targets not met (see above)");
    }
    
    Ok(())
}


