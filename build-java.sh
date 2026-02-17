#!/bin/bash
#
# Build script for AOEE Java components
# Builds gRPC client, Spring app, and persistence service
#

set -e  # Exit on error

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Parse arguments
SKIP_TESTS="-DskipTests"
CLEAN="clean"
VERBOSE=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --with-tests)
            SKIP_TESTS=""
            shift
            ;;
        --no-clean)
            CLEAN=""
            shift
            ;;
        -v|--verbose)
            VERBOSE="-X"
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [options]"
            echo ""
            echo "Options:"
            echo "  --with-tests   Run tests during build"
            echo "  --no-clean     Skip clean step (faster rebuild)"
            echo "  -v, --verbose  Enable Maven verbose output"
            echo "  -h, --help     Show this help"
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Check prerequisites
log_info "Checking prerequisites..."

if ! command -v java &> /dev/null; then
    log_error "Java not found. Please install Java 21+."
    exit 1
fi

if ! command -v mvn &> /dev/null; then
    log_error "Maven not found. Please install Maven 3.8+."
    exit 1
fi

JAVA_VERSION=$(java -version 2>&1 | head -n 1 | cut -d'"' -f2 | cut -d'.' -f1)
if [[ "$JAVA_VERSION" -lt 21 ]]; then
    log_warn "Java version $JAVA_VERSION detected. Java 21+ recommended."
fi

log_info "Java: $(java -version 2>&1 | head -n 1)"
log_info "Maven: $(mvn -version 2>&1 | head -n 1)"

# Build aoee-java-client (generates gRPC classes)
log_info "Building aoee-java-client (gRPC client with proto generation)..."
cd "$SCRIPT_DIR/aoee-java-client"

if ! mvn $CLEAN install $SKIP_TESTS $VERBOSE; then
    log_error "Failed to build aoee-java-client"
    exit 1
fi

log_info "✓ aoee-java-client built and installed to local Maven repo"

# Verify generated sources exist
if [[ -d "target/generated-sources/protobuf/java" ]]; then
    PROTO_COUNT=$(find target/generated-sources/protobuf -name "*.java" | wc -l | tr -d ' ')
    log_info "✓ Generated $PROTO_COUNT protobuf/gRPC Java files"
else
    log_warn "Generated sources directory not found"
fi

# Build aoee-spring
log_info "Building aoee-spring (Spring Boot application)..."
cd "$SCRIPT_DIR/aoee-spring"

if ! mvn $CLEAN compile $SKIP_TESTS $VERBOSE; then
    log_error "Failed to build aoee-spring"
    exit 1
fi

log_info "✓ aoee-spring built"

# Build aoee-persistence
log_info "Building aoee-persistence (Persistence service)..."
cd "$SCRIPT_DIR/aoee-persistence"

if ! mvn $CLEAN compile $SKIP_TESTS $VERBOSE; then
    log_error "Failed to build aoee-persistence"
    exit 1
fi

log_info "✓ aoee-persistence built"

# Summary
echo ""
echo "============================================"
echo -e "${GREEN}All Java components built successfully!${NC}"
echo "============================================"
echo ""
echo "Generated gRPC classes are in:"
echo "  aoee-java-client/target/generated-sources/protobuf/"
echo ""
echo "To use in IntelliJ:"
echo "  1. Open Maven tool window"
echo "  2. Click 'Reload All Maven Projects'"
echo ""
echo "To run the services:"
echo "  ./start-services.sh"
echo ""
