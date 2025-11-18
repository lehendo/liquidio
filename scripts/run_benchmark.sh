#!/bin/bash
set -e

echo "Liquidio Benchmark Suite"
echo "========================"

# Check if .env exists
if [ ! -f .env ]; then
    echo "[ERROR] .env file not found"
    echo "   Please run ./scripts/deploy_contracts.sh first"
    exit 1
fi

# Source environment variables
source .env

# Check if Anvil is running
if ! curl -s http://127.0.0.1:8545 > /dev/null; then
    echo "[ERROR] Anvil is not running"
    echo "   Please run ./scripts/deploy_contracts.sh first"
    exit 1
fi

echo "[OK] Environment configured"
echo "   Protocol: $LENDING_PROTOCOL_ADDRESS"
echo "   Token: $MOCK_TOKEN_ADDRESS"
echo ""

# Build in release mode for maximum performance
echo "Building bot in release mode..."
cargo build --release

if [ $? -ne 0 ]; then
    echo "[ERROR] Build failed"
    exit 1
fi

echo "[OK] Build complete"
echo ""

# Run the bot
echo "Running benchmark suite..."
echo ""
cargo run --release

echo ""
echo "[OK] Benchmark complete!"
echo ""
echo "Results saved to:"
echo "   - benchmark_results/transaction_stream_backtest.csv"
echo "   - benchmark_results/transaction_stream_backtest.json"
echo "   - benchmark_results/latency_stress_test.csv"
echo "   - benchmark_results/latency_stress_test.json"
echo ""
