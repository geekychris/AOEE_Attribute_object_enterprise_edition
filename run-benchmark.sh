#!/bin/bash

# AOEE Benchmark Runner
# Usage: ./run-benchmark.sh [small|medium|large|custom]

set -e

AOEE_ROOT="$(cd "$(dirname "$0")" && pwd)"
API_URL="http://localhost:8080/api/benchmark"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_header() {
    echo -e "\n${BLUE}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}  AOEE Benchmark Runner${NC}"
    echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}\n"
}

check_services() {
    echo -e "${YELLOW}Checking services...${NC}"
    
    # Check Rust server
    if ! curl -s http://localhost:8080/api/health > /dev/null 2>&1; then
        echo -e "${RED}Error: Spring Boot server not responding on port 8080${NC}"
        echo "Start services with: ./start-services.sh"
        exit 1
    fi
    
    echo -e "${GREEN}✓ Services are running${NC}\n"
}

run_benchmark() {
    local size=$1
    echo -e "${YELLOW}Running ${size} benchmark...${NC}"
    echo "This may take a few minutes for larger datasets."
    echo ""
    
    # Run benchmark and capture result
    local result=$(curl -s -X POST "${API_URL}/run/${size}")
    
    if [ $? -ne 0 ]; then
        echo -e "${RED}Error: Benchmark request failed${NC}"
        exit 1
    fi
    
    # Extract and display summary
    echo "$result" | python3 -c "
import sys, json
try:
    d = json.load(sys.stdin)
    print(d['summary'])
    
    # Save full results
    with open('benchmark-results-${size}.json', 'w') as f:
        json.dump(d, f, indent=2)
    print('\nFull results saved to: benchmark-results-${size}.json')
except Exception as e:
    print(f'Error parsing results: {e}', file=sys.stderr)
    sys.exit(1)
"
}

show_presets() {
    echo -e "${YELLOW}Available presets:${NC}\n"
    curl -s "${API_URL}/presets" | python3 -c "
import sys, json
d = json.load(sys.stdin)
for name, config in sorted(d.items()):
    users = config['numUsers']
    posts = config['numPosts']
    avg_follows = config['avgFollowsPerUser']
    max_follows = config['maxFollowsPerUser']
    avg_likes = config['avgLikesPerPost']
    max_likes = config['maxLikesPerPost']
    print(f'  {name}:')
    print(f'    Users: {users:,}, Posts: {posts:,}')
    print(f'    Follows: avg {avg_follows}, max {max_follows}')
    print(f'    Likes: avg {avg_likes}, max {max_likes}')
    print()
"
}

show_usage() {
    echo "Usage: $0 [command]"
    echo ""
    echo "Commands:"
    echo "  small     Run small benchmark (1K users, ~125K edges)"
    echo "  medium    Run medium benchmark (10K users, ~1.5M edges)"
    echo "  large     Run large benchmark (100K users, ~15M edges)"
    echo "  presets   Show preset configurations"
    echo "  generate  Generate data only (no benchmarking)"
    echo "  help      Show this help"
    echo ""
    echo "Examples:"
    echo "  $0 small              # Run small benchmark"
    echo "  $0 generate medium    # Generate medium dataset only"
}

# Main
print_header

case "${1:-help}" in
    small|medium|large)
        check_services
        run_benchmark "$1"
        ;;
    generate)
        check_services
        size="${2:-small}"
        echo -e "${YELLOW}Generating ${size} dataset...${NC}"
        curl -s -X POST "${API_URL}/generate/${size}" | python3 -c "
import sys, json
d = json.load(sys.stdin)
print(f\"Generated {d['totalEdges']:,} edges in {d['durationMs']:,}ms\")
print(f\"  Users: {d['usersCreated']:,}\")
print(f\"  Posts: {d['postsCreated']:,}\")
print(f\"  Groups: {d['groupsCreated']:,}\")
print(f\"  Follow edges: {d['followEdges']:,}\")
print(f\"  Friend edges: {d['friendEdges']:,}\")
print(f\"  Like edges: {d['likeEdges']:,}\")
print(f\"  Member edges: {d['memberEdges']:,}\")
"
        ;;
    presets)
        check_services
        show_presets
        ;;
    help|--help|-h)
        show_usage
        ;;
    *)
        echo -e "${RED}Unknown command: $1${NC}"
        show_usage
        exit 1
        ;;
esac

echo -e "\n${GREEN}Done!${NC}"
