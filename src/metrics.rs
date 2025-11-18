use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use tracing::info;

/// High-precision latency tracking for liquidation pipeline
#[derive(Debug, Clone)]
pub struct LatencyMetrics {
    #[allow(dead_code)]
    pub t_received: Instant,
    #[allow(dead_code)]
    pub t_decoded: Option<Instant>,
    #[allow(dead_code)]
    pub t_signal: Option<Instant>,
    #[allow(dead_code)]
    pub t_simulated: Option<Instant>,
    #[allow(dead_code)]
    pub t_constructed: Option<Instant>,
    #[allow(dead_code)]
    pub t_sent: Option<Instant>,
}

impl LatencyMetrics {
    pub fn new() -> Self {
        Self {
            t_received: Instant::now(),
            t_decoded: None,
            t_signal: None,
            t_simulated: None,
            t_constructed: None,
            t_sent: None,
        }
    }
    
    pub fn mark_decoded(&mut self) {
        self.t_decoded = Some(Instant::now());
    }
    
    pub fn mark_signal(&mut self) {
        self.t_signal = Some(Instant::now());
    }
    
    pub fn mark_simulated(&mut self) {
        self.t_simulated = Some(Instant::now());
    }
    
    pub fn mark_constructed(&mut self) {
        self.t_constructed = Some(Instant::now());
    }
    
    pub fn mark_sent(&mut self) {
        self.t_sent = Some(Instant::now());
    }
    
    /// Calculate latency from received to decoded
    pub fn latency_decode(&self) -> Option<Duration> {
        self.t_decoded.map(|t| t.duration_since(self.t_received))
    }
    
    /// Calculate latency from decoded to signal detected
    pub fn latency_signal_detection(&self) -> Option<Duration> {
        if let (Some(decoded), Some(signal)) = (self.t_decoded, self.t_signal) {
            Some(signal.duration_since(decoded))
        } else {
            None
        }
    }
    
    /// Calculate latency from signal to simulation complete
    pub fn latency_simulation(&self) -> Option<Duration> {
        if let (Some(signal), Some(simulated)) = (self.t_signal, self.t_simulated) {
            Some(simulated.duration_since(signal))
        } else {
            None
        }
    }
    
    /// Calculate latency from simulation to transaction construction
    pub fn latency_construction(&self) -> Option<Duration> {
        if let (Some(simulated), Some(constructed)) = (self.t_simulated, self.t_constructed) {
            Some(constructed.duration_since(simulated))
        } else {
            None
        }
    }
    
    /// Calculate end-to-end latency from received to sent
    pub fn latency_end_to_end(&self) -> Option<Duration> {
        self.t_sent.map(|t| t.duration_since(self.t_received))
    }
    
    /// Get all latencies as a map
    pub fn get_all_latencies(&self) -> HashMap<String, f64> {
        let mut map = HashMap::new();
        
        if let Some(d) = self.latency_decode() {
            map.insert("decode_us".to_string(), d.as_micros() as f64);
        }
        if let Some(d) = self.latency_signal_detection() {
            map.insert("signal_detection_us".to_string(), d.as_micros() as f64);
        }
        if let Some(d) = self.latency_simulation() {
            map.insert("simulation_us".to_string(), d.as_micros() as f64);
        }
        if let Some(d) = self.latency_construction() {
            map.insert("construction_us".to_string(), d.as_micros() as f64);
        }
        if let Some(d) = self.latency_end_to_end() {
            map.insert("end_to_end_us".to_string(), d.as_micros() as f64);
        }
        
        map
    }
}

impl Default for LatencyMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Aggregate metrics across multiple liquidation attempts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateMetrics {
    pub total_attempts: usize,
    pub successful_liquidations: usize,
    pub failed_liquidations: usize,
    pub latencies: Vec<HashMap<String, f64>>,
}

impl AggregateMetrics {
    pub fn new() -> Self {
        Self {
            total_attempts: 0,
            successful_liquidations: 0,
            failed_liquidations: 0,
            latencies: Vec::new(),
        }
    }
    
    pub fn record_attempt(&mut self, metrics: &LatencyMetrics, success: bool) {
        self.total_attempts += 1;
        if success {
            self.successful_liquidations += 1;
        } else {
            self.failed_liquidations += 1;
        }
        self.latencies.push(metrics.get_all_latencies());
    }
    
    /// Calculate percentile for a given metric
    pub fn percentile(&self, metric_name: &str, percentile: f64) -> Option<f64> {
        let mut values: Vec<f64> = self.latencies
            .iter()
            .filter_map(|m| m.get(metric_name).copied())
            .collect();
        
        if values.is_empty() {
            return None;
        }
        
        values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let index = ((percentile / 100.0) * values.len() as f64).floor() as usize;
        Some(values[index.min(values.len() - 1)])
    }
    
    /// Calculate mean for a given metric
    pub fn mean(&self, metric_name: &str) -> Option<f64> {
        let values: Vec<f64> = self.latencies
            .iter()
            .filter_map(|m| m.get(metric_name).copied())
            .collect();
        
        if values.is_empty() {
            return None;
        }
        
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }
    
    pub fn print_summary(&self) {
        info!("=== Liquidation Bot Performance Summary ===");
        info!("Total Attempts: {}", self.total_attempts);
        info!("Successful: {}", self.successful_liquidations);
        info!("Failed: {}", self.failed_liquidations);
        info!("Success Rate: {:.2}%", 
            (self.successful_liquidations as f64 / self.total_attempts as f64) * 100.0);
        
        info!("\n=== Latency Metrics (microseconds) ===");
        
        let metrics = vec![
            "decode_us",
            "signal_detection_us",
            "simulation_us",
            "construction_us",
            "end_to_end_us",
        ];
        
        for metric in metrics {
            if let (Some(p50), Some(p95), Some(p99), Some(mean)) = (
                self.percentile(metric, 50.0),
                self.percentile(metric, 95.0),
                self.percentile(metric, 99.0),
                self.mean(metric),
            ) {
                info!("{}: P50={:.2} P95={:.2} P99={:.2} Mean={:.2}", 
                    metric, p50, p95, p99, mean);
            }
        }
    }
    
    /// Export metrics to CSV
    pub fn export_to_csv(&self, filename: &str) -> anyhow::Result<()> {
        use std::fs::File;
        use csv::Writer;
        
        let file = File::create(filename)?;
        let mut writer = Writer::from_writer(file);
        
        // Write headers
        writer.write_record(&[
            "attempt",
            "decode_us",
            "signal_detection_us",
            "simulation_us",
            "construction_us",
            "end_to_end_us",
        ])?;
        
        // Write data
        for (i, latency) in self.latencies.iter().enumerate() {
            writer.write_record(&[
                i.to_string(),
                latency.get("decode_us").map(|v| v.to_string()).unwrap_or_default(),
                latency.get("signal_detection_us").map(|v| v.to_string()).unwrap_or_default(),
                latency.get("simulation_us").map(|v| v.to_string()).unwrap_or_default(),
                latency.get("construction_us").map(|v| v.to_string()).unwrap_or_default(),
                latency.get("end_to_end_us").map(|v| v.to_string()).unwrap_or_default(),
            ])?;
        }
        
        writer.flush()?;
        Ok(())
    }
}

impl Default for AggregateMetrics {
    fn default() -> Self {
        Self::new()
    }
}

