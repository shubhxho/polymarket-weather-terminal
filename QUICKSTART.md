# Quick Start Guide

## Build & Test (3 commands)

```bash
# 1. Build
cargo build --release

# 2. Test
cargo test

# 3. Run (offline mode)
OFFLINE_MODE=1 ./target/release/polymarket-weather-terminal
```

## Project Stats

- **Language**: Rust 2021 edition
- **Lines of Code**: ~1,500
- **Dependencies**: 418 crates
- **Build Time**: ~90 seconds
- **Binary Size**: 28 MB
- **Tests**: 5/5 passing ✅
- **Clippy**: Clean ✅

## Key Files

- `src/main.rs` - Entry point, spawns background refresh loop
- `src/api/polymarket.rs` - Gamma/CLOB API client (200+ lines)
- `src/api/weather.rs` - Open-Meteo ensemble forecasts (150+ lines)
- `src/analytics/mod.rs` - Signal computation, Kelly sizing (200+ lines)
- `src/gpu/renderer.rs` - GPU-accelerated UI (450+ lines)
- `fixtures/demo_markets.json` - Demo data for offline mode

## Architecture in 3 Layers

1. **API Layer** (`api/`): HTTP clients for Polymarket & weather data
2. **Analytics Layer** (`analytics/`): Ensemble → probability → edge → Kelly
3. **GPU Layer** (`gpu/`): eframe/wgpu rendering at 60+ FPS

## Demo Markets (Offline Mode)

When run with `OFFLINE_MODE=1`, shows 5 synthetic markets:

| City    | Question                              | Edge   |
|---------|---------------------------------------|--------|
| NYC     | High temp above 5°C on Nov 25?        | +15%   |
| London  | High temp between 8-12°C on Nov 25?   | +2%    |
| Chicago | High temp below 2°C on Nov 25?        | -24%   |
| Seoul   | High temp above 10°C on Nov 25?       | Variable |
| Tokyo   | High temp between 12-16°C on Nov 25?  | Variable |

(Edges shown are examples; actual values depend on synthetic forecast generation)

## Troubleshooting

### "Library libxkbcommon-x11.so could not be loaded"
**Fix**: Install X11 libraries
```bash
sudo apt install libxkbcommon-x11-dev
```

### "Package openssl was not found"
**Fix**: Install OpenSSL development headers
```bash
sudo apt install libssl-dev pkg-config
```

### "No weather markets found"
**Expected**: Polymarket currently has no active weather markets. App uses demo fixtures automatically.

## Performance Tips

- Release builds are **~10x faster** than debug builds
- UI targets **60+ FPS** (requires GPU with Vulkan/Metal/DX12)
- Data refresh happens every **30 seconds** in background
- Offline mode has **zero network latency**

## What It Does (ELI5)

1. Finds weather betting markets on Polymarket
2. Gets 31 different weather forecasts for each market
3. Counts how many forecasts say "YES" to the market question
4. Compares that to what people are betting
5. Shows you the difference (the "edge")
6. Suggests bet sizes using Kelly criterion
7. Displays everything in a fast GPU-rendered UI

## What It Doesn't Do

- ❌ No actual trading
- ❌ No wallet integration
- ❌ No real-time WebSocket streams (yet)
- ❌ No GPU compute shaders (yet)
- ❌ Not investment advice

## License & Disclaimer

**Educational purposes only. No warranty. Trading involves risk.**
