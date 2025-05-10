//! WebAssembly Component Model format.
//!
//! This module provides types and utilities for working with the WebAssembly
//! Component Model binary format.

// Import from crate::lib re-exports to ensure proper features
use crate::{format, Box, String, Vec};

use crate::module::Module;
use crate::types::ValueType;
use crate::validation::Validatable;
use wrt_error::{Error, Result};
use wrt_types::resource::{ResourceDrop, ResourceNew, ResourceRep, ResourceRepresentation};
// Re-export ValType from wrt-types
pub use wrt_types::component_value::ValType;

/// WebAssembly Component Model component definition
#[derive(Debug, Clone)]
pub struct Component {
    /// Component name (if available from name section)
    pub name: Option<String>,
    /// Core modules included in this component
    pub modules: Vec<Module>,
    /// Core instances defined in this component
    pub core_instances: Vec<CoreInstance>,
    /// Core types defined in this component
    pub core_types: Vec<CoreType>,
    /// Nested components
    pub components: Vec<Component>,
    /// Component instances
    pub instances: Vec<Instance>,
    /// Component aliases
    pub aliases: Vec<Alias>,
    /// Component types
    pub types: Vec<ComponentType>,
    /// Canonical function conversions
    pub canonicals: Vec<Canon>,
    /// Component start function
    pub start: Option<Start>,
    /// Component imports
    pub imports: Vec<Import>,
    /// Component exports
    pub exports: Vec<Export>,
    /// Component values
    pub values: Vec<Value>,
    /// Original binary (if available)
    pub binary: Option<Vec<u8>>,
}

impl Default for Component {
    fn default() -> Self {
        Self::new()
    }
}

impl Component {
    /// Create a new empty component
    pub fn new() -> Self {
        Self {
            name: None,
            modules: Vec::new(),
            core_instances: Vec::new(),
            core_types: Vec::new(),
            components: Vec::new(),
            instances: Vec::new(),
            aliases: Vec::new(),
            types: Vec::new(),
            canonicals: Vec::new(),
            start: None,
            imports: Vec::new(),
            exports: Vec::new(),
            values: Vec::new(),
            binary: None,
        }
    }
}

impl Validatable for Component {
    fn validate(&self) -> Result<()> {
        // Validate component name if present
        if let Some(name) = &self.name {
            if name.is_empty() {
                return Err(Error::validation_error("Component name cannot be empty"));
            }
        }

        // Validate all child elements
        self.modules.validate()?;
        self.core_instances.validate()?;
        self.core_types.validate()?;
        self.components.validate()?;
        self.instances.validate()?;
        self.aliases.validate()?;
        self.types.validate()?;
        self.canonicals.validate()?;
        self.start.validate()?;
        self.imports.validate()?;
        self.exports.validate()?;
        self.values.validate()?;

        Ok(())
    }
}

/// Core WebAssembly instance definition in a component
#[derive(Debug, Clone)]
pub struct CoreInstance {
    /// Instance expression
    pub instance_expr: CoreInstanceExpr,
}

impl Validatable for CoreInstance {
    fn validate(&self) -> Result<()> {
        match &self.instance_expr {
            CoreInstanceExpr::Instantiate { module_idx, args } => {
                // Basic validation: module_idx should be reasonable
                if *module_idx > 10000 {
                    // Arbitrary reasonable limit
                    return Err(Error::validation_error(format!(
                        "Module index {} seems unreasonably large",
                        module_idx
                    )));
                }

                // Validate args
                for arg in args {
                    if arg.name.is_empty() {
                        return Err(Error::validation_error(
                            "Instantiate arg name cannot be empty",
                        ));
                    }
                }

                Ok(())
            }
            CoreInstanceExpr::InlineExports(exports) => {
                // Validate exports
                for export in exports {
                    if export.name.is_empty() {
                        return Err(Error::validation_error("Inline export name cannot be empty"));
                    }
                    // Reasonable index limit
                    if export.idx > 100000 {
                        return Err(Error::validation_error(format!(
                            "Export index {} seems unreasonably large",
                            export.idx
                        )));
                    }
                }

                Ok(())
            }
        }
    }
}

/// Core WebAssembly instance expression
#[derive(Debug, Clone)]
pub enum CoreInstanceExpr {
    /// Instantiate a core module
    Instantiate {
        /// Module index
        module_idx: u32,
        /// Instantiation arguments
        args: Vec<CoreInstantiateArg>,
    },
    /// Collection of inlined exports
    InlineExports(Vec<CoreInlineExport>),
}

/// Core WebAssembly instantiation argument
#[derive(Debug, Clone)]
pub struct CoreInstantiateArg {
    /// Name of the argument
    pub name: String,
    /// Instance index that provides the value
    pub instance_idx: u32,
}

/// Core WebAssembly inlined export
#[derive(Debug, Clone)]
pub struct CoreInlineExport {
    /// Export name
    pub name: String,
    /// Export reference
    pub sort: CoreSort,
    /// Index within the sort
    pub idx: u32,
}

/// Core WebAssembly sort kinds
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreSort {
    /// Function reference
    Function,
    /// Table reference
    Table,
    /// Memory reference
    Memory,
    /// Global reference
    Global,
    /// Type reference
    Type,
    /// Module reference
    Module,
    /// Instance reference
    Instance,
}

/// Core WebAssembly type definition in a component
#[derive(Debug, Clone)]
pub struct CoreType {
    /// Type definition
    pub definition: CoreTypeDefinition,
}

impl Validatable for CoreType {
    fn validate(&self) -> Result<()> {
        match &self.definition {
            CoreTypeDefinition::Function { params, results } => {
                // Basic validation: reasonable limits on params and results
                if params.len() > 1000 {
                    return Err(Error::validation_error(format!(
                        "Function has too many parameters ({})",
                        params.len()
                    )));
                }

                if results.len() > 1000 {
                    return Err(Error::validation_error(format!(
                        "Function has too many results ({})",
                        results.len()
                    )));
                }

                Ok(())
            }
            CoreTypeDefinition::Module { imports, exports } => {
                // Validate imports
                for (namespace, name, _) in imports {
                    if namespace.is_empty() {
                        return Err(Error::validation_error("Import namespace cannot be empty"));
                    }
                    if name.is_empty() {
                        return Err(Error::validation_error("Import name cannot be empty"));
                    }
                }

                // Validate exports
                for (name, _) in exports {
                    if name.is_empty() {
                        return Err(Error::validation_error("Export name cannot be empty"));
                    }
                }

                Ok(())
            }
        }
    }
}

/// Core WebAssembly type definition
#[derive(Debug, Clone)]
pub enum CoreTypeDefinition {
    /// Function type
    Function {
        /// Parameter types
        params: Vec<ValueType>,
        /// Result types
        results: Vec<ValueType>,
    },
    /// Module type
    Module {
        /// Module imports
        imports: Vec<(String, String, CoreExternType)>,
        /// Module exports
        exports: Vec<(String, CoreExternType)>,
    },
}

/// Core WebAssembly external type
#[derive(Debug, Clone)]
pub enum CoreExternType {
    /// Function type
    Function {
        /// Parameter types
        params: Vec<ValueType>,
        /// Result types
        results: Vec<ValueType>,
    },
    /// Table type
    Table {
        /// Element type
        element_type: ValueType,
        /// Minimum size
        min: u32,
        /// Maximum size (optional)
        max: Option<u32>,
    },
    /// Memory type
    Memory {
        /// Minimum size in pages
        min: u32,
        /// Maximum size in pages (optional)
        max: Option<u32>,
        /// Whether the memory is shared
        shared: bool,
    },
    /// Global type
    Global {
        /// Value type
        value_type: ValueType,
        /// Whether the global is mutable
        mutable: bool,
    },
}

/// Component instance definition
#[derive(Debug, Clone)]
pub struct Instance {
    /// Instance expression
    pub instance_expr: InstanceExpr,
}

/// Component instance expression
#[derive(Debug, Clone)]
pub enum InstanceExpr {
    /// Instantiate a component
    Instantiate {
        /// Component index
        component_idx: u32,
        /// Instantiation arguments
        args: Vec<InstantiateArg>,
    },
    /// Collection of inlined exports
    InlineExports(Vec<InlineExport>),
}

/// Component instantiation argument
#[derive(Debug, Clone)]
pub struct InstantiateArg {
    /// Name of the argument
    pub name: String,
    /// Sort of the referenced item
    pub sort: Sort,
    /// Index within the sort
    pub idx: u32,
}

/// Component inlined export
#[derive(Debug, Clone)]
pub struct InlineExport {
    /// Export name
    pub name: String,
    /// Export reference
    pub sort: Sort,
    /// Index within the sort
    pub idx: u32,
}

/// Component sort kinds
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sort {
    /// Core reference
    Core(CoreSort),
    /// Function reference
    Function,
    /// Value reference
    Value,
    /// Type reference
    Type,
    /// Component reference
    Component,
    /// Instance reference
    Instance,
}

/// Component alias definition
#[derive(Debug, Clone)]
pub struct Alias {
    /// Alias target
    pub target: AliasTarget,
}

/// Component alias target
#[derive(Debug, Clone)]
pub enum AliasTarget {
    /// Core WebAssembly export from an instance
    CoreInstanceExport {
        /// Instance index
        instance_idx: u32,
        /// Export name
        name: String,
        /// Kind of the target
        kind: CoreSort,
    },
    /// Export from a component instance
    InstanceExport {
        /// Instance index
        instance_idx: u32,
        /// Export name
        name: String,
        /// Kind of the target
        kind: Sort,
    },
    /// Outer definition from an enclosing component (forwarding from parent)
    Outer {
        /// Count of components to traverse outward
        count: u32,
        /// Kind of the target
        kind: Sort,
        /// Index within the kind
        idx: u32,
    },
}

/// Component type definition
#[derive(Debug, Clone)]
pub struct ComponentType {
    /// Type definition
    pub definition: ComponentTypeDefinition,
}

/// Component type definition
#[derive(Debug, Clone)]
pub enum ComponentTypeDefinition {
    /// Component type
    Component {
        /// Component imports
        imports: Vec<(String, String, ExternType)>,
        /// Component exports
        exports: Vec<(String, ExternType)>,
    },
    /// Instance type
    Instance {
        /// Instance exports
        exports: Vec<(String, ExternType)>,
    },
    /// Function type
    Function {
        /// Parameter types
        params: Vec<(String, FormatValType)>,
        /// Result types
        results: Vec<FormatValType>,
    },
    /// Value type
    Value(FormatValType),
    /// Resource type
    Resource {
        /// Resource representation type
        representation: ResourceRepresentation,
        /// Whether the resource is nullable
        nullable: bool,
    },
}

/// Component external type
#[derive(Debug, Clone)]
pub enum ExternType {
    /// Function type
    Function {
        /// Parameter types
        params: Vec<(String, FormatValType)>,
        /// Result types
        results: Vec<FormatValType>,
    },
    /// Value type
    Value(FormatValType),
    /// Type reference
    Type(u32),
    /// Instance type
    Instance {
        /// Instance exports
        exports: Vec<(String, ExternType)>,
    },
    /// Component type
    Component {
        /// Component imports
        imports: Vec<(String, String, ExternType)>,
        /// Component exports
        exports: Vec<(String, ExternType)>,
    },
}

/// Component Model value types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatValType {
    /// Boolean type
    Bool,
    /// 8-bit signed integer
    S8,
    /// 8-bit unsigned integer
    U8,
    /// 16-bit signed integer
    S16,
    /// 16-bit unsigned integer
    U16,
    /// 32-bit signed integer
    S32,
    /// 32-bit unsigned integer
    U32,
    /// 64-bit signed integer
    S64,
    /// 64-bit unsigned integer
    U64,
    /// 32-bit float
    F32,
    /// 64-bit float
    F64,
    /// Unicode character
    Char,
    /// String type
    String,
    /// Reference type
    Ref(u32),
    /// Record type with named fields
    Record(Vec<(String, FormatValType)>),
    /// Variant type
    Variant(Vec<(String, Option<FormatValType>)>),
    /// List type
    List(Box<FormatValType>),
    /// Fixed-length list type with element type and length
    FixedList(Box<FormatValType>, u32),
    /// Tuple type
    Tuple(Vec<FormatValType>),
    /// Flags type
    Flags(Vec<String>),
    /// Enum type
    Enum(Vec<String>),
    /// Option type
    Option(Box<FormatValType>),
    /// Result type (can contain ok or error)
    Result(Box<FormatValType>),
    /// Own a resource
    Own(u32),
    /// Borrow a resource
    Borrow(u32),
    /// Void/empty type
    Void,
    /// Error context type
    ErrorContext,
}

/// Canonical function conversion
#[derive(Debug, Clone)]
pub struct Canon {
    /// Canonical operation
    pub operation: CanonOperation,
}

/// Canonical operation
#[derive(Debug, Clone)]
pub enum CanonOperation {
    /// Lift a core function to the component ABI
    Lift {
        /// Core function index
        func_idx: u32,
        /// Type index for the lifted function
        type_idx: u32,
        /// Options for lifting
        options: LiftOptions,
    },
    /// Lower a component function to the core ABI
    Lower {
        /// Component function index
        func_idx: u32,
        /// Options for lowering
        options: LowerOptions,
    },
    /// Resource operations
    Resource(FormatResourceOperation),
    /// Reallocation operation
    Realloc {
        /// Function index for memory allocation
        alloc_func_idx: u32,
        /// Memory index to use
        memory_idx: u32,
    },
    /// Post-return operation (cleanup)
    PostReturn {
        /// Function index for post-return cleanup
        func_idx: u32,
    },
    /// Memory copy operation (optional optimization)
    MemoryCopy {
        /// Source memory index
        src_memory_idx: u32,
        /// Destination memory index
        dst_memory_idx: u32,
        /// Function index for the copy operation
        func_idx: u32,
    },
    /// Async operation (stackful lift)
    Async {
        /// Function index for the async operation
        func_idx: u32,
        /// Type index for the async function
        type_idx: u32,
        /// Options for async operations
        options: AsyncOptions,
    },
}

/// Options for lifting operations
#[derive(Debug, Clone)]
pub struct LiftOptions {
    /// Memory index to use for string/list conversions
    pub memory_idx: Option<u32>,
    /// String encoding to use
    pub string_encoding: Option<StringEncoding>,
    /// Realloc function index (optional)
    pub realloc_func_idx: Option<u32>,
    /// Post-return function index (optional)
    pub post_return_func_idx: Option<u32>,
    /// Whether this is an async lift
    pub is_async: bool,
}

/// Options for lowering operations
#[derive(Debug, Clone)]
pub struct LowerOptions {
    /// Memory index to use for string/list conversions
    pub memory_idx: Option<u32>,
    /// String encoding to use
    pub string_encoding: Option<StringEncoding>,
    /// Realloc function index (optional)
    pub realloc_func_idx: Option<u32>,
    /// Whether this is an async lower
    pub is_async: bool,
    /// Error handling mode
    pub error_mode: Option<ErrorMode>,
}

/// Options for async operations
#[derive(Debug, Clone)]
pub struct AsyncOptions {
    /// Memory index to use
    pub memory_idx: u32,
    /// Realloc function index
    pub realloc_func_idx: Option<u32>,
    /// String encoding to use
    pub string_encoding: Option<StringEncoding>,
}

/// String encoding options
#[derive(Debug, Clone)]
pub enum StringEncoding {
    /// UTF-8 encoding
    UTF8,
    /// UTF-16 encoding
    UTF16,
    /// Latin-1 encoding
    Latin1,
    /// ASCII encoding
    ASCII,
}

/// Error handling modes
#[derive(Debug, Clone)]
pub enum ErrorMode {
    /// Convert errors to exceptions
    ThrowOnError,
    /// Convert exceptions to errors
    CatchExceptions,
    /// Pass through errors/exceptions
    Passthrough,
}

/// Component start function
#[derive(Debug, Clone)]
pub struct Start {
    /// Function index
    pub func_idx: u32,
    /// Value arguments
    pub args: Vec<u32>,
    /// Number of results
    pub results: u32,
}

/// Import definition in a component
#[derive(Debug, Clone)]
pub struct Import {
    /// Import name in namespace.name format
    pub name: ImportName,
    /// Type of the import
    pub ty: ExternType,
}

/// Import name with support for nested namespaces
#[derive(Debug, Clone)]
pub struct ImportName {
    /// Namespace of the import
    pub namespace: String,
    /// Name of the import
    pub name: String,
    /// Nested namespaces (if any)
    pub nested: Vec<String>,
    /// Package reference (if any)
    pub package: Option<PackageReference>,
}

/// Package reference for imports
#[derive(Debug, Clone)]
pub struct PackageReference {
    /// Package name
    pub name: String,
    /// Package version
    pub version: Option<String>,
    /// Package hash (for content verification)
    pub hash: Option<String>,
}

/// Export definition in a component
#[derive(Debug, Clone)]
pub struct Export {
    /// Export name in "name" format
    pub name: ExportName,
    /// Sort of the exported item
    pub sort: Sort,
    /// Index within the sort
    pub idx: u32,
    /// Declared type (optional)
    pub ty: Option<ExternType>,
}

/// Export name with support for nested namespaces
#[derive(Debug, Clone)]
pub struct ExportName {
    /// Basic name
    pub name: String,
    /// Whether this export is a resource
    pub is_resource: bool,
    /// Semver compatibility string
    pub semver: Option<String>,
    /// Integrity hash for content verification
    pub integrity: Option<String>,
    /// Nested namespaces (if any)
    pub nested: Vec<String>,
}

impl ImportName {
    /// Create a new import name with just namespace and name
    pub fn new(namespace: String, name: String) -> Self {
        Self { namespace, name, nested: Vec::new(), package: None }
    }

    /// Create a new import name with nested namespaces
    pub fn with_nested(namespace: String, name: String, nested: Vec<String>) -> Self {
        Self { namespace, name, nested, package: None }
    }

    /// Add package reference to an import name
    pub fn with_package(mut self, package: PackageReference) -> Self {
        self.package = Some(package);
        self
    }

    /// Get the full import path as a string
    pub fn full_path(&self) -> String {
        let mut path = format!("{}.{}", self.namespace, self.name);
        for nested in &self.nested {
            path.push_str(&format!(".{}", nested));
        }
        path
    }
}

impl ExportName {
    /// Create a new export name
    pub fn new(name: String) -> Self {
        Self { name, is_resource: false, semver: None, integrity: None, nested: Vec::new() }
    }

    /// Create a new export name with nested namespaces
    pub fn with_nested(name: String, nested: Vec<String>) -> Self {
        Self { name, is_resource: false, semver: None, integrity: None, nested }
    }

    /// Mark as a resource export
    pub fn as_resource(mut self) -> Self {
        self.is_resource = true;
        self
    }

    /// Add semver compatibility information
    pub fn with_semver(mut self, semver: String) -> Self {
        self.semver = Some(semver);
        self
    }

    /// Add integrity hash for content verification
    pub fn with_integrity(mut self, integrity: String) -> Self {
        self.integrity = Some(integrity);
        self
    }

    /// Get the full export path as a string
    pub fn full_path(&self) -> String {
        let mut path = self.name.clone();
        for nested in &self.nested {
            path.push_str(&format!(".{}", nested));
        }
        path
    }
}

/// Component value definition
#[derive(Debug, Clone)]
pub struct Value {
    /// Type of the value
    pub ty: FormatValType,
    /// Encoded value data
    pub data: Vec<u8>,
    /// Value expression (if available)
    pub expression: Option<ValueExpression>,
    /// Value name (if available from custom sections)
    pub name: Option<String>,
}

/// Value expression types
#[derive(Debug, Clone)]
pub enum ValueExpression {
    /// Reference to an item in component
    ItemRef {
        /// Sort of the referenced item
        sort: Sort,
        /// Index within the sort
        idx: u32,
    },
    /// Global initialization expression
    GlobalInit {
        /// Global index
        global_idx: u32,
    },
    /// Function call expression
    FunctionCall {
        /// Function index
        func_idx: u32,
        /// Arguments (indices to other values)
        args: Vec<u32>,
    },
    /// Direct constant value
    Const(ConstValue),
}

/// Constant value types
#[derive(Debug, Clone)]
pub enum ConstValue {
    /// Boolean constant
    Bool(bool),
    /// 8-bit signed integer
    S8(i8),
    /// 8-bit unsigned integer
    U8(u8),
    /// 16-bit signed integer
    S16(i16),
    /// 16-bit unsigned integer
    U16(u16),
    /// 32-bit signed integer
    S32(i32),
    /// 32-bit unsigned integer
    U32(u32),
    /// 64-bit signed integer
    S64(i64),
    /// 64-bit unsigned integer
    U64(u64),
    /// 32-bit float
    F32(f32),
    /// 64-bit float
    F64(f64),
    /// Character constant
    Char(char),
    /// String constant
    String(String),
    /// Empty null constant
    Null,
}

impl Validatable for Instance {
    fn validate(&self) -> Result<()> {
        match &self.instance_expr {
            InstanceExpr::Instantiate { component_idx, args } => {
                // Basic validation: component_idx should be reasonable
                if *component_idx > 10000 {
                    // Arbitrary reasonable limit
                    return Err(Error::validation_error(format!(
                        "Component index {} seems unreasonably large",
                        component_idx
                    )));
                }

                // Validate args
                for arg in args {
                    if arg.name.is_empty() {
                        return Err(Error::validation_error(
                            "Instantiate arg name cannot be empty",
                        ));
                    }
                }

                Ok(())
            }
            InstanceExpr::InlineExports(exports) => {
                // Validate exports
                for export in exports {
                    if export.name.is_empty() {
                        return Err(Error::validation_error("Inline export name cannot be empty"));
                    }
                }

                Ok(())
            }
        }
    }
}

impl Validatable for Alias {
    fn validate(&self) -> Result<()> {
        match &self.target {
            AliasTarget::CoreInstanceExport { instance_idx, name, .. } => {
                if *instance_idx > 10000 {
                    return Err(Error::validation_error(format!(
                        "Instance index {} seems unreasonably large",
                        instance_idx
                    )));
                }

                if name.is_empty() {
                    return Err(Error::validation_error("Export name cannot be empty"));
                }

                Ok(())
            }
            AliasTarget::InstanceExport { instance_idx, name, .. } => {
                if *instance_idx > 10000 {
                    return Err(Error::validation_error(format!(
                        "Instance index {} seems unreasonably large",
                        instance_idx
                    )));
                }

                if name.is_empty() {
                    return Err(Error::validation_error("Export name cannot be empty"));
                }

                Ok(())
            }
            AliasTarget::Outer { count, idx, .. } => {
                if *count > 10 {
                    return Err(Error::validation_error(format!(
                        "Outer count {} seems unreasonably large",
                        count
                    )));
                }

                if *idx > 10000 {
                    return Err(Error::validation_error(format!(
                        "Index {} seems unreasonably large",
                        idx
                    )));
                }

                Ok(())
            }
        }
    }
}

impl Validatable for ComponentType {
    fn validate(&self) -> Result<()> {
        match &self.definition {
            ComponentTypeDefinition::Component { imports, exports } => {
                // Validate imports
                for (namespace, name, _) in imports {
                    if namespace.is_empty() {
                        return Err(Error::validation_error("Import namespace cannot be empty"));
                    }
                    if name.is_empty() {
                        return Err(Error::validation_error("Import name cannot be empty"));
                    }
                }

                // Validate exports
                for (name, _) in exports {
                    if name.is_empty() {
                        return Err(Error::validation_error("Export name cannot be empty"));
                    }
                }

                Ok(())
            }
            ComponentTypeDefinition::Instance { exports } => {
                // Validate exports
                for (name, _) in exports {
                    if name.is_empty() {
                        return Err(Error::validation_error("Export name cannot be empty"));
                    }
                }

                Ok(())
            }
            ComponentTypeDefinition::Function { params, results } => {
                // Basic validation: reasonable limits on params and results
                if params.len() > 1000 {
                    return Err(Error::validation_error(format!(
                        "Function has too many parameters ({})",
                        params.len()
                    )));
                }

                // Check param names
                for (name, _) in params {
                    if name.is_empty() {
                        return Err(Error::validation_error("Parameter name cannot be empty"));
                    }
                }

                if results.len() > 1000 {
                    return Err(Error::validation_error(format!(
                        "Function has too many results ({})",
                        results.len()
                    )));
                }

                Ok(())
            }
            ComponentTypeDefinition::Value(_) => {
                // Simple value types don't need further validation
                Ok(())
            }
            ComponentTypeDefinition::Resource { .. } => {
                // Resource types are validated elsewhere
                Ok(())
            }
        }
    }
}

impl Validatable for Canon {
    fn validate(&self) -> Result<()> {
        match &self.operation {
            CanonOperation::Lift { func_idx, type_idx, .. } => {
                if *func_idx > 10000 {
                    return Err(Error::validation_error(format!(
                        "Function index {} seems unreasonably large",
                        func_idx
                    )));
                }

                if *type_idx > 10000 {
                    return Err(Error::validation_error(format!(
                        "Type index {} seems unreasonably large",
                        type_idx
                    )));
                }

                Ok(())
            }
            CanonOperation::Lower { func_idx, .. } => {
                if *func_idx > 10000 {
                    return Err(Error::validation_error(format!(
                        "Function index {} seems unreasonably large",
                        func_idx
                    )));
                }

                Ok(())
            }
            // Other operations have simpler validation requirements
            _ => Ok(()),
        }
    }
}

impl Validatable for Start {
    fn validate(&self) -> Result<()> {
        if self.func_idx > 10000 {
            return Err(Error::validation_error(format!(
                "Function index {} seems unreasonably large",
                self.func_idx
            )));
        }

        if self.args.len() > 1000 {
            return Err(Error::validation_error(format!(
                "Start function has too many arguments ({})",
                self.args.len()
            )));
        }

        if self.results > 1000 {
            return Err(Error::validation_error(format!(
                "Start function has too many results ({})",
                self.results
            )));
        }

        Ok(())
    }
}

impl Validatable for Import {
    fn validate(&self) -> Result<()> {
        // Validate import name
        if self.name.namespace.is_empty() {
            return Err(Error::validation_error("Import namespace cannot be empty"));
        }

        if self.name.name.is_empty() {
            return Err(Error::validation_error("Import name cannot be empty"));
        }

        // Nested namespaces should not be empty strings
        for nested in &self.name.nested {
            if nested.is_empty() {
                return Err(Error::validation_error("Nested namespace cannot be empty"));
            }
        }

        // Validate package reference if present
        if let Some(pkg) = &self.name.package {
            if pkg.name.is_empty() {
                return Err(Error::validation_error("Package name cannot be empty"));
            }
        }

        Ok(())
    }
}

impl Validatable for Export {
    fn validate(&self) -> Result<()> {
        // Validate export name
        if self.name.name.is_empty() {
            return Err(Error::validation_error("Export name cannot be empty"));
        }

        // Nested namespaces should not be empty strings
        for nested in &self.name.nested {
            if nested.is_empty() {
                return Err(Error::validation_error("Nested namespace cannot be empty"));
            }
        }

        // Index should be reasonable
        if self.idx > 10000 {
            return Err(Error::validation_error(format!(
                "Export index {} seems unreasonably large",
                self.idx
            )));
        }

        Ok(())
    }
}

impl Validatable for Value {
    fn validate(&self) -> Result<()> {
        // Validate data size (should be reasonable)
        if self.data.len() > 1000000 {
            return Err(Error::validation_error(format!(
                "Value data size {} seems unreasonably large",
                self.data.len()
            )));
        }

        // Check value expression if present
        if let Some(expr) = &self.expression {
            match expr {
                ValueExpression::ItemRef { idx, .. } => {
                    if *idx > 10000 {
                        return Err(Error::validation_error(format!(
                            "Item reference index {} seems unreasonably large",
                            idx
                        )));
                    }
                }
                ValueExpression::GlobalInit { global_idx } => {
                    if *global_idx > 10000 {
                        return Err(Error::validation_error(format!(
                            "Global index {} seems unreasonably large",
                            global_idx
                        )));
                    }
                }
                ValueExpression::FunctionCall { func_idx, args } => {
                    if *func_idx > 10000 {
                        return Err(Error::validation_error(format!(
                            "Function index {} seems unreasonably large",
                            func_idx
                        )));
                    }

                    if args.len() > 1000 {
                        return Err(Error::validation_error(format!(
                            "Function call has too many arguments ({})",
                            args.len()
                        )));
                    }
                }
                ValueExpression::Const(_) => {
                    // Constants are validated elsewhere
                }
            }
        }

        Ok(())
    }
}

/// Resource operation in a canonical function
#[derive(Debug, Clone)]
pub enum FormatResourceOperation {
    /// New resource operation
    New(ResourceNew),
    /// Drop a resource
    Drop(ResourceDrop),
    /// Resource representation operation
    Rep(ResourceRep),
}
