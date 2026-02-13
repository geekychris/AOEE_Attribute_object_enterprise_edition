#!/bin/bash

# AOEE Services Startup Script
# Starts: Rust server, Spring Boot proxy, React UI, and optionally Persistence service
#
# Usage:
#   ./start-services.sh          # Start core services only
#   ./start-services.sh --persist # Start with persistence service

AOEE_ROOT="/Users/chris/AOEE"
START_PERSISTENCE=false

# Parse arguments
for arg in "$@"; do
    case $arg in
        --persist|--persistence)
            START_PERSISTENCE=true
            ;;
    esac
done

echo "🛑 Stopping existing services..."

# Kill existing processes
pkill -f "aoee-server" 2>/dev/null
pkill -f "aoee-spring.*spring-boot:run" 2>/dev/null
pkill -f "aoee-persistence.*spring-boot:run" 2>/dev/null
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

# Optionally start persistence service (port 8081)
if [ "$START_PERSISTENCE" = true ]; then
    echo "  → Starting Persistence service..."
    cd "$AOEE_ROOT/aoee-persistence"
    mvn spring-boot:run -q > "$AOEE_ROOT/logs/persistence.log" 2>&1 &
    PERSIST_PID=$!
    sleep 5
    
    # Start Spring Boot proxy with persistence enabled
    echo "  → Starting Spring Boot proxy (with persistence)..."
    cd "$AOEE_ROOT/aoee-spring"
    SPRING_OPTS="-Daoee.persistence.enabled=true"
    mvn spring-boot:run -q -Dspring-boot.run.jvmArguments="$SPRING_OPTS" > "$AOEE_ROOT/logs/spring-boot.log" 2>&1 &
    SPRING_PID=$!
else
    # Start Spring Boot proxy without persistence
    echo "  → Starting Spring Boot proxy..."
    cd "$AOEE_ROOT/aoee-spring"
    mvn spring-boot:run -q > "$AOEE_ROOT/logs/spring-boot.log" 2>&1 &
    SPRING_PID=$!
fi

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
if [ "$START_PERSISTENCE" = true ]; then
echo "   Persistence:       http://localhost:8081          [PID: $PERSIST_PID]"
echo "   Spring Boot proxy: http://localhost:8080 (w/persist) [PID: $SPRING_PID]"
else
echo "   Spring Boot proxy: http://localhost:8080          [PID: $SPRING_PID]"
fi
echo "   React UI:          http://localhost:5173          [PID: $UI_PID]"
echo ""
echo "📁 Logs: $AOEE_ROOT/logs/"
echo ""
if [ "$START_PERSISTENCE" = true ]; then
echo "🔗 Persistence endpoints:"
echo "   REST API:    http://localhost:8081/api/v1/"
echo "   GraphQL:     http://localhost:8081/graphql"
echo "   GraphiQL:    http://localhost:8081/graphiql"
echo "   H2 Console:  http://localhost:8081/h2-console"
echo ""
echo "   Warm cache:  curl -X POST http://localhost:8080/api/cache/warm"
echo ""
fi
echo "To stop all services: pkill -f aoee-server; pkill -f spring-boot:run; pkill -f 'http.server 5173'"
