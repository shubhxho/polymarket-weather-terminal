#!/bin/bash
set -e

mkdir -p docs/screenshots

echo "Starting Xvfb on display :99..."
Xvfb :99 -screen 0 1600x900x24 &
XVFB_PID=$!
export DISPLAY=:99

sleep 2

echo "Running app in offline mode..."
timeout 15 OFFLINE_MODE=1 RUST_LOG=info ./target/release/polymarket-weather-terminal &
APP_PID=$!

sleep 8

echo "Capturing screenshot..."
import -window root docs/screenshots/offline_overview.png

sleep 2

kill $APP_PID 2>/dev/null || true

echo "Running app in online mode..."
timeout 15 RUST_LOG=info ./target/release/polymarket-weather-terminal &
APP_PID=$!

sleep 8

echo "Capturing online screenshot..."
import -window root docs/screenshots/online_overview.png

sleep 2

kill $APP_PID 2>/dev/null || true
kill $XVFB_PID 2>/dev/null || true

echo "Screenshots captured successfully!"
ls -lh docs/screenshots/

echo "Converting to smaller sizes..."
cd docs/screenshots
for img in *.png; do
    convert "$img" -resize 800x450 "${img%.png}_small.png"
done

ls -lh
