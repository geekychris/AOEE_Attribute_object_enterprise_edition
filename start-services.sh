#!/bin/bash

# AOEE Services Startup Script
# Starts: Rust server, Spring Boot proxy, React UI

AOEE_ROOT="/Users/chris/AOEE"

echo "🛑 Stopping existing services..."

# Kill existing processes
pkill -f "aoee-server" 2>/dev/null
pkill -f "spring-boot:run" 2>/dev/null
pkill -f "http.server 5173" 2>/dev/null

sleep 1

echo "🚀 Starting AOEE services..."

# Start Rust AOEE server (port 50051)
echo "  → Starting Rust AOEE server..."
cd "$AOEE_ROOT/aoee"
./target/release/aoee-server > "$AOEE_ROOT/logs/aoee-server.log" 2>&1 &
RUST_PID=$!

# Wait for Rust server to be ready
sleep 3

# Start Spring Boot proxy (port 8080)
echo "  → Starting Spring Boot proxy..."
cd "$AOEE_ROOT/aoee-spring"
mvn spring-boot:run -q > "$AOEE_ROOT/logs/spring-boot.log" 2>&1 &
SPRING_PID=$!

# Wait for Spring to start
sleep 5

# Start React UI server (port 5173)
echo "  → Starting React UI server..."
cd "$AOEE_ROOT/aoee-ui"
python3 -m http.server 5173 --directory dist > "$AOEE_ROOT/logs/react-ui.log" 2>&1 &
UI_PID=$!

echo ""
echo "✅ All services started!"
echo ""
echo "   Rust AOEE server:  http://localhost:50051 (gRPC)  [PID: $RUST_PID]"
echo "   Spring Boot proxy: http://localhost:8080          [PID: $SPRING_PID]"
echo "   React UI:          http://localhost:5173          [PID: $UI_PID]"
echo ""
echo "📁 Logs: $AOEE_ROOT/logs/"
echo ""
echo "To stop all services: pkill -f aoee-server; pkill -f spring-boot:run; pkill -f 'http.server 5173'"
