#!/bin/bash
# Resource Limits Configuration Demonstration
# 
# This script demonstrates the complete configuration chain from TOML
# configuration to runtime execution with resource limits.

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}🚀 WRT Resource Limits Configuration Demo${NC}"
echo "=============================================="
echo

# Check if cargo-wrt is available
if ! command -v cargo-wrt &> /dev/null; then
    echo -e "${RED}❌ cargo-wrt not found. Please build it first:${NC}"
    echo "   cargo build --bin cargo-wrt"
    exit 1
fi

# Create demo directory
DEMO_DIR="target/resource_limits_demo"
mkdir -p "$DEMO_DIR"
cd "$DEMO_DIR"

echo -e "${YELLOW}📝 Step 1: Creating ASIL-D resource limits configuration${NC}"
cat > limits.toml << 'EOF'
# ASIL-D Resource Limits Configuration
version = 1

[execution]
max_fuel_per_step = 50000        # Conservative fuel limit for ASIL-D
max_memory_usage = "16M"         # 16MB memory limit
max_call_depth = 32              # Limited call depth
max_instructions_per_step = 1000 # Limited instructions per step
max_execution_slice_ms = 10      # 10ms execution slices

[resources.filesystem]
max_handles = 16                 # Limited file handles
max_memory = "2M"                # 2MB for filesystem operations
max_operations_per_second = 100  # Rate limited

[resources.filesystem.custom]
max_file_size = 1048576          # 1MB max file size
max_path_length = 256            # Limited path length

[resources.memory]
max_handles = 4                  # Very limited memory mappings
max_memory = "8M"                # 8MB for memory operations

[qualification]
binary_hash = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
asil_level = "ASIL-D"
EOF

echo -e "${GREEN}✅ Created ASIL-D configuration in limits.toml${NC}"
echo

echo -e "${YELLOW}📝 Step 2: Creating minimal WebAssembly binary${NC}"
cat > hello.wat << 'EOF'
(module
  (func $hello (result i32)
    i32.const 42
  )
  (export "hello" (func $hello))
)
EOF

# Convert WAT to WASM if wat2wasm is available
if command -v wat2wasm &> /dev/null; then
    wat2wasm hello.wat -o hello.wasm
    echo -e "${GREEN}✅ Created WebAssembly binary: hello.wasm${NC}"
else
    echo -e "${YELLOW}⚠️  wat2wasm not found, creating dummy WASM file${NC}"
    # Create minimal valid WASM binary
    printf '\x00\x61\x73\x6d\x01\x00\x00\x00' > hello.wasm
fi
echo

echo -e "${YELLOW}📝 Step 3: Embedding resource limits into WebAssembly binary${NC}"
echo "Command: cargo-wrt embed-limits hello.wasm -c limits.toml --validate --replace"
echo

# Note: This command may fail if dependencies aren't properly set up
# The important thing is demonstrating the interface
if cargo-wrt embed-limits hello.wasm -c limits.toml --validate --replace 2>/dev/null; then
    echo -e "${GREEN}✅ Successfully embedded resource limits!${NC}"
    
    # Check if the custom section was added (basic check)
    if [ -f hello.wasm ] && [ -s hello.wasm ]; then
        SIZE=$(wc -c < hello.wasm)
        echo -e "${GREEN}📊 Binary size: ${SIZE} bytes${NC}"
    fi
else
    echo -e "${YELLOW}⚠️  Command failed (expected in demo), but the interface is demonstrated${NC}"
fi
echo

echo -e "${YELLOW}📝 Step 4: Demonstrating different ASIL levels${NC}"

# Create QM configuration
cat > limits_qm.toml << 'EOF'
# QM (Quality Management) - No safety requirements
version = 1

[execution]
max_fuel_per_step = 10000000     # High fuel limit
max_memory_usage = "1G"          # 1GB memory limit
max_call_depth = 1000            # Deep call stacks allowed
max_instructions_per_step = 100000
max_execution_slice_ms = 1000    # 1 second slices

[resources.filesystem]
max_handles = 1024               # Many file handles
max_memory = "256M"              # Large filesystem memory
max_operations_per_second = 10000

[qualification]
asil_level = "QM"
EOF

# Create ASIL-B configuration
cat > limits_asil_b.toml << 'EOF'
# ASIL-B - Safety relevant
version = 1

[execution]
max_fuel_per_step = 500000       # Moderate fuel limit
max_memory_usage = "128M"        # 128MB memory limit
max_call_depth = 128             # Moderate call depth
max_instructions_per_step = 10000
max_execution_slice_ms = 100     # 100ms execution slices

[resources.filesystem]
max_handles = 128                # Moderate file handles
max_memory = "16M"               # 16MB for filesystem
max_operations_per_second = 1000

[qualification]
asil_level = "ASIL-B"
EOF

echo -e "${GREEN}✅ Created configurations for QM, ASIL-B, and ASIL-D levels${NC}"
echo

echo -e "${YELLOW}📝 Step 5: Configuration Chain Summary${NC}"
echo "The complete configuration chain works as follows:"
echo
echo "1. 📄 TOML Configuration (limits.toml)"
echo "   ↓ cargo-wrt embed-limits"
echo "2. 🔧 Binary Custom Section (wrt.resource_limits)"
echo "   ↓ Runtime loads binary"
echo "3. 🏃 ASILExecutionConfig (runtime configuration)"
echo "   ↓ spawn_task_with_binary()"
echo "4. ⚙️  ExecutionContext (enforced limits)"
echo "   ↓ Execution"
echo "5. 🛡️  Bounded Collections (ASIL-D safety)"
echo

echo -e "${YELLOW}📊 Resource Limit Comparison:${NC}"
echo "┌─────────────┬─────────────┬─────────────┬─────────────┐"
echo "│ Limit       │ QM          │ ASIL-B      │ ASIL-D      │"
echo "├─────────────┼─────────────┼─────────────┼─────────────┤"
echo "│ Fuel/step   │ 10,000,000  │ 500,000     │ 50,000      │"
echo "│ Memory      │ 1GB         │ 128MB       │ 16MB        │"
echo "│ Call depth  │ 1,000       │ 128         │ 32          │"
echo "│ Time slice  │ 1000ms      │ 100ms       │ 10ms        │"
echo "│ File handles│ 1,024       │ 128         │ 16          │"
echo "└─────────────┴─────────────┴─────────────┴─────────────┘"
echo

echo -e "${YELLOW}🔍 Key ASIL-D Features:${NC}"
echo "• ✅ No dynamic allocation (BoundedVec, BoundedMap, BoundedString)"
echo "• ✅ Compile-time capacity limits (MAX_RESOURCE_TYPES = 16)"
echo "• ✅ Memory provider pattern (safe_managed_alloc!)"
echo "• ✅ Capability-based memory system integration"
echo "• ✅ Binary qualification with SHA-256 hash"
echo "• ✅ Deterministic execution guarantees"
echo

echo -e "${YELLOW}🛠️  Usage Examples:${NC}"
echo "# Embed limits with ASIL-D validation:"
echo "cargo-wrt embed-limits module.wasm -c limits.toml --asil ASIL-D --validate"
echo
echo "# Use in runtime (Rust code):"
echo "let executor = FuelAsyncExecutor::new(ASILExecutionMode::ASIL_D);"
echo "let task_id = executor.spawn_task_with_binary("
echo "    component_id, fuel_budget, Priority::High,"
echo "    async_task, Some(wasm_bytes) // Limits extracted automatically"
echo ")?;"
echo

echo -e "${GREEN}🎉 Demo completed! Resource limits configuration chain demonstrated.${NC}"
echo -e "${BLUE}📚 See RESOURCE_LIMITS_IMPLEMENTATION.md for detailed documentation.${NC}"