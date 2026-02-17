# Building gRPC Java Client

This guide explains how to build the gRPC generated classes for the AOEE Java client and Spring application.

## Prerequisites

- Java 21+ (OpenJDK or Corretto)
- Maven 3.8+

Verify your setup:
```bash
java -version   # Should show 21+
mvn -version    # Should show 3.8+
```

## Quick Build

From the project root, run:
```bash
./build-java.sh
```

This script builds everything in the correct order.

## Manual Build Steps

### Step 1: Build and Install the Java Client

The `aoee-java-client` module contains the proto file and generates the gRPC stubs.

```bash
cd aoee-java-client
mvn clean install
```

This will:
- Download the protoc compiler and gRPC Java plugin
- Generate Java classes from `src/main/proto/aoee.proto`
- Compile the generated code plus the client wrapper classes
- Install the JAR to your local Maven repository (`~/.m2/repository`)

**Generated files location:**
- Protobuf messages: `target/generated-sources/protobuf/java/`
- gRPC service stubs: `target/generated-sources/protobuf/grpc-java/`

### Step 2: Build the Spring Application

The `aoee-spring` module depends on `aoee-java-client`:

```bash
cd aoee-spring
mvn clean compile
```

### Step 3: Build the Persistence Service (optional)

If you need the persistence layer:

```bash
cd aoee-persistence
mvn clean compile
```

## IntelliJ IDEA Setup

After running the Maven build, IntelliJ needs to recognize the generated sources:

### Option A: Reload Maven Project (Recommended)
1. Open the **Maven** tool window (View → Tool Windows → Maven)
2. Click the **Reload All Maven Projects** button (🔄)
3. Wait for indexing to complete

### Option B: Mark Directories Manually
1. Navigate to `aoee-java-client/target/generated-sources/protobuf/java`
2. Right-click → **Mark Directory as** → **Generated Sources Root**
3. Navigate to `aoee-java-client/target/generated-sources/protobuf/grpc-java`
4. Right-click → **Mark Directory as** → **Generated Sources Root**

### Option C: Import as Maven Project
If the above doesn't work:
1. File → Invalidate Caches / Restart
2. After restart: File → Open → Select the `pom.xml` in `aoee-java-client`
3. Choose "Open as Project"

## Generated Classes

After building, you'll have these generated classes:

| Package | Class | Description |
|---------|-------|-------------|
| `com.aoee.proto` | `Aoee` | All protobuf message types |
| `com.aoee.proto` | `AoeeGrpc` | gRPC service stubs |
| `com.aoee.proto` | `AoeeGrpc.AoeeBlockingStub` | Synchronous client stub |
| `com.aoee.proto` | `AoeeGrpc.AoeeStub` | Async client stub |

## Troubleshooting

### "Cannot resolve symbol" in IntelliJ
- Run `mvn clean install` in `aoee-java-client`
- Reload Maven projects in IntelliJ
- If still failing, try File → Invalidate Caches / Restart

### Proto compilation fails
- Ensure you have internet access (Maven downloads protoc)
- Check that `src/main/proto/aoee.proto` exists
- Verify Maven version: `mvn -version`

### Spring app can't find aoee-java-client
- Ensure you ran `mvn install` (not just `compile`) on aoee-java-client
- Check `~/.m2/repository/com/aoee/aoee-java-client/0.1.0/` exists

### Architecture mismatch (M1/M2 Mac)
If you see errors about architecture:
```bash
# Force Maven to use the correct OS classifier
mvn clean install -Dos.detected.classifier=osx-aarch_64
```

## Project Structure

```
AOEE/
├── aoee-java-client/          # gRPC client library
│   ├── pom.xml                # Maven config with protobuf plugin
│   └── src/
│       ├── main/
│       │   ├── java/          # Client wrapper classes
│       │   └── proto/         # Proto definitions
│       │       └── aoee.proto
│       └── target/
│           └── generated-sources/
│               └── protobuf/
│                   ├── java/       # Generated message classes
│                   └── grpc-java/  # Generated service stubs
│
├── aoee-spring/               # Spring Boot application
│   ├── pom.xml                # Depends on aoee-java-client
│   └── src/
│
└── aoee-persistence/          # Persistence service
    ├── pom.xml
    └── src/
```

## Related Documentation

- [Running Services](RUNNING.md) - How to start all services
- [Architecture](ARCHITECTURE.md) - System overview
- [Persistence](PERSISTENCE.md) - Storage layer details
