==========================
Component Definitions
==========================

**Teaching Point**: Each component has a single, well-defined responsibility. This page shows how the actual implementation is organized into components.

Core Components
---------------

Runtime Core
~~~~~~~~~~~~

.. arch_component:: Runtime Core
   :id: ARCH_COMP_001
   :crate: wrt-runtime
   :implements: REQ_001, REQ_002, REQ_CORE_001
   :provides: ARCH_IF_001, ARCH_IF_002
   :requires: ARCH_IF_010, ARCH_IF_011
   :status: implemented

**Purpose**: Executes WebAssembly instructions and manages runtime state.

**Key Types** (from actual implementation):

.. code-block:: rust

   // From wrt-runtime/src/lib.rs
   pub trait EngineBehavior {
       fn execute_instruction(&mut self, instr: Instruction) -> Result<()>;
       fn get_module_instance(&self, idx: usize) -> Option<&ModuleInstance>;
   }
   
   pub struct ModuleInstance {
       module: Arc<Module>,
       memories: Arc<Mutex<Vec<Arc<Memory>>>>,
       tables: Arc<Mutex<Vec<Arc<Table>>>>,
       globals: Arc<Mutex<Vec<Arc<Global>>>>,
   }

**Environment Support**:

- **std**: Full threading with `Arc<Mutex<T>>`
- **no_std + alloc**: Uses `wrt_sync::Mutex` 
- **no_std + no_alloc**: Static instance pools with bounded capacity

Memory Manager
~~~~~~~~~~~~~~

.. arch_component:: Memory Manager
   :id: ARCH_COMP_002
   :crate: wrt-foundation
   :implements: REQ_MEM_001, REQ_MEM_SAFETY_001
   :provides: ARCH_IF_020, ARCH_IF_021
   :status: implemented

**Purpose**: Provides safe memory operations with verification.

**Actual Implementation**:

.. code-block:: rust

   // From wrt-foundation/src/safe_memory.rs
   pub trait MemoryProvider: Clone + PartialEq + Eq {
       fn len(&self) -> usize;
       fn read_bytes(&self, offset: usize, length: usize) -> Result<&[u8]>;
       fn write_bytes(&mut self, offset: usize, data: &[u8]) -> Result<()>;
   }
   
   // Different providers for different environments
   pub struct StdProvider { data: Vec<u8> }      // std environment
   pub struct NoStdProvider<const N: usize> {    // no_alloc environment
       data: [u8; N],
       used: usize,
   }

**Teaching Point**: The same interface (`MemoryProvider`) works across all environments, but with different implementations.

Component Model Runtime
~~~~~~~~~~~~~~~~~~~~~~~

.. arch_component:: Component Runtime
   :id: ARCH_COMP_003
   :crate: wrt-component
   :implements: REQ_COMP_001, REQ_COMP_002
   :provides: ARCH_IF_030
   :requires: ARCH_IF_001, ARCH_IF_020
   :status: implemented

**Purpose**: Implements WebAssembly Component Model specification.

**Key Interfaces** (actual code):

.. code-block:: rust

   // From wrt-component/src/lib.rs
   pub trait ComponentInstance {
       fn instantiate(component: &Component, imports: Imports) -> Result<Self>;
       fn export(&self, name: &str) -> Option<Export>;
       fn call(&mut self, name: &str, args: &[Value]) -> Result<Vec<Value>>;
   }

Decoder Component
~~~~~~~~~~~~~~~~~

.. arch_component:: Binary Decoder
   :id: ARCH_COMP_004
   :crate: wrt-decoder
   :implements: REQ_DECODE_001
   :provides: ARCH_IF_040
   :status: implemented

**Purpose**: Parses WebAssembly binary format into internal representation.

**no_alloc Support**:

.. code-block:: rust

   // From wrt-decoder/src/decoder_no_alloc.rs
   pub const MAX_MODULE_SIZE: usize = 65536; // 64KB
   
   pub struct NoAllocDecoder<const N: usize> {
       buffer: BoundedVec<u8, N>,
       sections: BoundedVec<Section, MAX_SECTIONS>,
   }

Platform Abstraction Layer
~~~~~~~~~~~~~~~~~~~~~~~~~~

.. arch_component:: Platform Layer
   :id: ARCH_COMP_005
   :crate: wrt-platform
   :implements: REQ_PLATFORM_001
   :provides: ARCH_IF_050, ARCH_IF_051
   :status: implemented

**Purpose**: Abstracts OS-specific operations for portability.

**Actual Traits**:

.. code-block:: rust

   // From wrt-platform/src/lib.rs
   pub trait PageAllocator {
       fn allocate(&mut self, pages: usize) -> Result<*mut u8>;
       fn deallocate(&mut self, ptr: *mut u8, pages: usize) -> Result<()>;
       fn grow(&mut self, ptr: *mut u8, old_pages: usize, new_pages: usize) -> Result<*mut u8>;
   }
   
   pub trait FutexLike {
       fn wait(&self, addr: &AtomicU32, expected: u32, timeout: Option<Duration>) -> Result<()>;
       fn wake(&self, addr: &AtomicU32, count: u32) -> Result<u32>;
   }

Supporting Components
---------------------

Error Management
~~~~~~~~~~~~~~~~

.. arch_component:: Error System
   :id: ARCH_COMP_010
   :crate: wrt-error
   :implements: REQ_ERROR_001
   :provides: ARCH_IF_100
   :status: implemented

**Actual Error Categories**:

.. code-block:: rust

   // From wrt-error/src/kinds.rs
   pub enum ErrorCategory {
       Validation,
       Runtime,
       Memory,
       Type,
       Resource,
       Component,
       Parse,
   }

Instruction Execution
~~~~~~~~~~~~~~~~~~~~~

.. arch_component:: Instruction Set
   :id: ARCH_COMP_011
   :crate: wrt-instructions
   :implements: REQ_EXEC_001
   :requires: ARCH_IF_001
   :status: implemented

**Instruction Categories** (actual implementation):

- Arithmetic operations (`arithmetic_ops.rs`)
- Control flow (`control_ops.rs`, `cfi_control_ops.rs`)
- Memory operations (`memory_ops.rs`)
- Variable access (`variable_ops.rs`)
- Type conversions (`conversion_ops.rs`)
- Table operations (`table_ops.rs`)

Bounded Collections
~~~~~~~~~~~~~~~~~~~

.. arch_component:: Bounded Collections
   :id: ARCH_COMP_012
   :crate: wrt-foundation
   :feature: no_alloc
   :implements: REQ_BOUNDED_001
   :status: implemented

**Teaching Point**: These replace standard collections in no_alloc environments:

.. code-block:: rust

   // Actual constants from wrt-foundation/src/bounded.rs
   pub const MAX_WASM_NAME_LENGTH: usize = 255;
   pub const MAX_BUFFER_SIZE: usize = 65536;
   
   pub struct BoundedVec<T, const N: usize, P: MemoryProvider> {
       provider: P,
       len: usize,
       verification_level: VerificationLevel,
   }

Component Dependencies
----------------------

.. needflow::
   :filter: type == "arch_component" and id in ["ARCH_COMP_001", "ARCH_COMP_002", "ARCH_COMP_003", "ARCH_COMP_004", "ARCH_COMP_005"]

Cross-References
----------------

- **Interface Details**: See :doc:`../03_interfaces/interface_catalog`
- **Dynamic Behavior**: See :doc:`../04_dynamic_behavior/interaction_flows`
- **Implementation Examples**: See :doc:`/examples/foundation/bounded_collections`