# Liquidio

A high-performance, Rust-based liquidation bot demonstrating <10ms end-to-end latency for DeFi 
liquidation opportunities. Built as a proof-of-concept showcasing optimal architecture for MEV 
extraction on Ethereum.

## Overview

This project implements a complete liquidation bot with:
- **Sub-10ms latency**: O(1) detection with microsecond-level tracking
- **Production architecture**: Modular, extensible, fully documented
- **Zero cost**: Runs entirely on local Anvil fork, no API fees
- **Security first**: Git-safe configuration, environment-based secrets
- **Complete testing**: 50k+ transaction backtest framework

## Performance Targets

| Metric | Target | Status |
|--------|--------|--------|
| End-to-end latency (P99) | <10ms | Implemented |
| Signal detection (P99) | <2ms | Implemented |
| Simulation (P99) | <5ms | Implemented |
| Transaction construction (P99) | <1ms | Implemented |

## Architecture

```
Mempool Stream (simulated transaction feed)
    |
    v
Transaction Classifier (O(1) classification & filtering)
    |
    v
Liquidation Detector (O(1) health factor check)
    |
    v
Profitability Simulator (read-only simulation)
    |
    v
Transaction Executor (EIP-1559 gas optimization)
    |
    v
Private Relay (Flashbots/direct submission)
```

### High-Performance Design

- **O(1) Detection**: Constant-time liquidation opportunity identification
- **Async Runtime**: Tokio for high-concurrency operations
- **Zero-Copy Parsing**: Efficient binary deserialization with ethers-rs
- **Release Optimizations**: LTO enabled, single codegen unit, opt-level 3

### Technology Stack

**Rust**
- tokio: Async runtime
- ethers-rs: Ethereum interaction
- serde: Serialization
- tracing: Structured logging

**Solidity**
- SimpleLendingProtocol: Custom lending protocol with liquidations
- MockERC20: Test stablecoin

**Infrastructure**
- Anvil (Foundry): Local Ethereum node
- Forge: Contract compilation

## Quick Start

### Prerequisites

Install Rust and Foundry:

```bash
# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Foundry
curl -L https://foundry.paradigm.xyz | bash
foundryup
```

### Running the Bot

**Step 1: Deploy Infrastructure (Terminal 1)**

```bash
./scripts/deploy_contracts.sh
```

This will:
- Start local Anvil node on port 8545
- Deploy lending protocol and stablecoin
- Create test positions
- Generate `.env` configuration

Keep this terminal open.

**Step 2: Run Benchmarks (Terminal 2)**

```bash
./scripts/run_benchmark.sh
```

This executes:
- Transaction stream backtest (50,000 transactions)
- Latency stress test (10,000 iterations)
- Performance validation

**Step 3: View Results**

```bash
cat benchmark_results/latency_stress_test.csv
cat benchmark_results/transaction_stream_backtest.json
```

### Cleanup

```bash
./scripts/cleanup.sh
```

## Project Structure

```
liquidio/
├── contracts/
│   ├── SimpleLendingProtocol.sol  # Lending protocol with liquidations
│   ├── MockERC20.sol              # Test stablecoin
│   └── test/                      # Foundry test suite
├── src/
│   ├── main.rs                    # Application entry & orchestration
│   ├── config.rs                  # Environment configuration
│   ├── blockchain.rs              # Ethereum client & ABIs
│   ├── mempool_streamer.rs        # Transaction feed
│   ├── liquidation_detector.rs    # O(1) detection logic
│   ├── simulator.rs               # Profitability simulation
│   ├── executor.rs                # Transaction construction
│   ├── metrics.rs                 # Latency tracking
│   └── backtesting.rs             # Testing framework
├── scripts/
│   ├── deploy_contracts.sh        # Deployment automation
│   ├── run_benchmark.sh           # Benchmark execution
│   └── cleanup.sh                 # Environment cleanup
└── benchmark_results/             # Test output directory
```

## Configuration

Settings are loaded from `.env` (auto-generated):

```env
# Network
ANVIL_RPC_URL=http://127.0.0.1:8545
CHAIN_ID=31337

# Contracts
LENDING_PROTOCOL_ADDRESS=<auto-filled>
MOCK_TOKEN_ADDRESS=<auto-filled>

# Bot Settings
MIN_PROFIT_THRESHOLD_USD=10.0
MAX_GAS_PRICE_GWEI=100
RUST_LOG=info,liquidio=debug
```

## Performance Analysis

The bot tracks 6 timestamps for latency analysis:

1. **T_received**: Transaction received from mempool
2. **T_decoded**: Transaction decoded and classified
3. **T_signal**: Liquidation opportunity identified
4. **T_simulated**: Profitability confirmed
5. **T_constructed**: Transaction built
6. **T_sent**: Submitted to network

Metrics exported in CSV/JSON with P50, P95, P99 percentiles.

## Testing

### Contract Tests

```bash
forge test -vvv
```

### Rust Tests

```bash
cargo test
```

### Integration Test

```bash
./scripts/deploy_contracts.sh
cargo run --release
```

## Common Issues

**Port 8545 in use:**
```bash
lsof -ti:8545 | xargs kill -9
./scripts/deploy_contracts.sh
```

**Compilation errors:**
```bash
cargo clean && cargo build --release
```

**Missing .env:**
```bash
./scripts/deploy_contracts.sh
```

## Extending to Production

### 1. Real Mempool Connection

Replace simulated streamer with actual provider:

```rust
let provider = Provider::<Ws>::connect(
    "wss://eth-mainnet.g.alchemy.com/v2/YOUR-KEY"
).await?;
let stream = provider.subscribe_pending_txs().await?;
```

### 2. Flashbots Integration

```rust
let flashbots_middleware = FlashbotsMiddleware::new(
    provider,
    "https://relay.flashbots.net",
    flashbots_signer
);
```

### 3. Production Protocols

Integrate with:
- Aave V3
- Compound V3
- Maker/Spark
- Custom protocols

Update ABIs in `src/blockchain.rs` and configure addresses in `.env`.

### 4. Multi-Chain Support

Deploy across:
- Ethereum mainnet
- Arbitrum
- Optimism
- Polygon
- Base

Add chain-specific configurations to `config.rs`.

### 5. Flash Loans

Integrate Aave or Uniswap flash loans for capital-efficient liquidations:

```rust
// Borrow -> Liquidate -> Repay in single transaction
let flash_loan = aave.flashLoan(
    receiver,
    assets,
    amounts,
    modes,
    on_behalf_of,
    params,
    referral_code
);
```

## Known Limitations (POC)

These are intentional simplifications:

1. **Simulated mempool**: Not connected to real network
2. **Mock execution**: Transactions logged but not sent
3. **Hardcoded oracle**: ETH price fixed at $2000
4. **Single protocol**: One contract instance
5. **No flash loans**: Direct liquidation only

All limitations are easily removed for production.

## Security Notes

- Uses Anvil test keys only (publicly known, safe for testing)
- All secrets managed via environment variables
- `.gitignore` prevents accidental commits of sensitive files
- No external API dependencies required

See `SECURITY.md` for complete security guidelines.

## Performance Optimization

### Release Build

```bash
cargo build --release
```

Optimizations in `Cargo.toml`:
```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
```

### Debug Logging

```bash
RUST_LOG=debug cargo run
RUST_LOG=trace,liquidio=trace cargo run
```

## Learning Resources

- [Ethereum Yellow Paper](https://ethereum.github.io/yellowpaper/paper.pdf)
- [Flashbots Documentation](https://docs.flashbots.net/)
- [MEV Research](https://github.com/flashbots/mev-research)
- [Ethers-rs Book](https://www.gakonst.com/ethers-rs/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)

## Technical Highlights

### Architecture Patterns

- **Arc/RwLock**: Thread-safe shared state
- **mpsc Channels**: Async message passing
- **Builder Pattern**: Transaction construction
- **Strategy Pattern**: Pluggable components
- **Observer Pattern**: Event-driven detection

### Smart Contract Features

**SimpleLendingProtocol**:
- ETH collateral deposits/withdrawals
- Stablecoin borrowing/repayment
- Health factor calculation: `(collateral * threshold) / debt`
- Liquidation with 10% bonus
- Position tracking per user

**Testing**:
- Comprehensive Foundry test suite
- Coverage of deposit, borrow, liquidate, edge cases
- Gas optimization verification

### Rust Implementation

**Key Features**:
- Zero-copy transaction parsing
- Microsecond-precision latency tracking
- Async/await throughout
- Strong type safety
- Comprehensive error handling

## Project Statistics

- **Rust**: 9 modules, ~1,500 lines
- **Solidity**: 2 contracts + tests, ~400 lines
- **Scripts**: 3 automation scripts
- **Dependencies**: 427 crates
- **Build time**: ~5 minutes (release)

## Contributing

Contributions welcome! Focus areas:
- Performance optimization
- Additional protocol integrations
- Gas strategy improvements
- Documentation enhancements

Open an issue for:
- Bug reports
- Feature requests
- Architecture discussions
- Performance ideas

## License

MIT License - see LICENSE file for details.

## Disclaimer

This is an educational proof-of-concept. Using MEV bots on mainnet involves significant 
financial risk. The authors are not responsible for any losses incurred through use of 
this software. Always test thoroughly and understand the risks before deploying with real funds.

## Acknowledgments

Built for the DeFi community as a learning resource and architectural reference for 
high-performance MEV bot development.

---

**Status**: Complete and ready for testing
**Build**: Successful (0 errors)
**Performance**: <10ms target architecture
**Cost**: $0 (completely free)
