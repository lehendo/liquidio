#!/bin/bash

echo "Cleanup Script"
echo "=============="

# Kill Anvil processes
echo "Stopping Anvil processes..."
pkill -f anvil || true

# Clean build artifacts
echo "Cleaning Rust build artifacts..."
cargo clean

# Clean Foundry artifacts
echo "Cleaning Foundry artifacts..."
rm -rf cache/
rm -rf out/
rm -rf broadcast/

# Remove generated .env (keep .env.example)
if [ -f .env ]; then
    echo "Removing generated .env file..."
    rm .env
fi

# Clean benchmark results (keep directory and .gitkeep)
echo "Cleaning benchmark results..."
find benchmark_results/ -type f ! -name '.gitkeep' -delete

# Clean logs
if [ -f /tmp/anvil.log ]; then
    echo "Removing Anvil logs..."
    rm /tmp/anvil.log
fi

echo "[OK] Cleanup complete!"
echo ""
echo "To restart:"
echo "  1. ./scripts/deploy_contracts.sh"
echo "  2. ./scripts/run_benchmark.sh"
