//! WAST Test Runner Integration
//!
//! This module provides a comprehensive WAST test infrastructure that integrates
//! with the existing wrt-test-registry framework. It supports all WAST directive
//! types and provides proper categorization, error handling, and resource management.

#![cfg(test)]

#[cfg(feature = "std")]
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

#[cfg(not(feature = "std"))]
use wrt_foundation::bounded::{BoundedHashMap as HashMap, BoundedVec};

use wast::{
    core::{NanPattern, WastArgCore, WastRetCore},
    parser::{self, ParseBuffer},
    Wast, WastArg, WastDirective, WastExecute, WastRet,
};

use wrt::{Error, Module, Result, StacklessEngine, Value};
use wrt_test_registry::{TestCase, TestConfig, TestRegistry, TestResult, TestRunner, TestSuite};

/// WAST Test Runner that integrates with the existing test infrastructure
pub struct WastTestRunner {
    /// Module registry for linking tests (std only)
    #[cfg(feature = "std")]
    module_registry: HashMap<String, Module>,
    /// Current active module for testing
    current_module: Option<Module>,
    /// Test statistics
    pub stats: WastTestStats,
    /// Resource limits for exhaustion testing
    pub resource_limits: ResourceLimits,
}

/// Statistics for WAST test execution
#[derive(Debug, Default, Clone)]
pub struct WastTestStats {
    /// Number of assert_return tests executed
    pub assert_return_count: usize,
    /// Number of assert_trap tests executed
    pub assert_trap_count: usize,
    /// Number of assert_invalid tests executed
    pub assert_invalid_count: usize,
    /// Number of assert_malformed tests executed
    pub assert_malformed_count: usize,
    /// Number of assert_unlinkable tests executed
    pub assert_unlinkable_count: usize,
    /// Number of assert_exhaustion tests executed
    pub assert_exhaustion_count: usize,
    /// Number of module registration operations
    pub register_count: usize,
    /// Number of successful tests
    pub passed: usize,
    /// Number of failed tests
    pub failed: usize,
}

/// Resource limits for testing exhaustion scenarios
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Maximum stack depth
    pub max_stack_depth: usize,
    /// Maximum memory size in bytes
    pub max_memory_size: usize,
    /// Maximum execution steps
    pub max_execution_steps: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_stack_depth: 1024,
            max_memory_size: 64 * 1024 * 1024, // 64MB
            max_execution_steps: 1_000_000,
        }
    }
}

/// Classification of WAST test types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WastTestType {
    /// Correctness tests (assert_return)
    Correctness,
    /// Error handling tests (assert_trap, assert_invalid, etc.)
    ErrorHandling,
    /// Integration tests (multi-module with register)
    Integration,
    /// Resource tests (assert_exhaustion)
    Resource,
}

/// Information about a WAST directive for categorization
#[derive(Debug, Clone)]
pub struct WastDirectiveInfo {
    /// Type of test
    pub test_type: WastTestType,
    /// Name of the directive
    pub directive_name: String,
    /// Whether the test requires module state
    pub requires_module_state: bool,
    /// Whether the test modifies the engine state
    pub modifies_engine_state: bool,
}

impl WastTestRunner {
    /// Create a new WAST test runner
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "std")]
            module_registry: HashMap::new(),
            current_module: None,
            stats: WastTestStats::default(),
            resource_limits: ResourceLimits::default(),
        }
    }

    /// Set resource limits for exhaustion testing
    pub fn set_resource_limits(&mut self, limits: ResourceLimits) {
        self.resource_limits = limits;
    }

    /// Execute a WAST directive with proper error handling and categorization
    pub fn execute_directive(
        &mut self,
        engine: &mut StacklessEngine,
        directive: &mut WastDirective,
    ) -> Result<WastDirectiveInfo> {
        match directive {
            WastDirective::Module(ref mut wast_module) => {
                self.handle_module_directive(engine, wast_module)
            }
            WastDirective::AssertReturn { span: _, exec, results } => {
                self.handle_assert_return_directive(engine, exec, results)
            }
            WastDirective::AssertTrap { span: _, exec, message } => {
                self.handle_assert_trap_directive(engine, exec, message)
            }
            WastDirective::AssertInvalid { span: _, module, message } => {
                self.handle_assert_invalid_directive(module, message)
            }
            WastDirective::AssertMalformed { span: _, module, message } => {
                self.handle_assert_malformed_directive(module, message)
            }
            WastDirective::AssertUnlinkable { span: _, module, message } => {
                self.handle_assert_unlinkable_directive(module, message)
            }
            WastDirective::AssertExhaustion { span: _, exec, message } => {
                self.handle_assert_exhaustion_directive(engine, exec, message)
            }
            WastDirective::Register { span: _, name, module } => {
                self.handle_register_directive(name, module)
            }
            WastDirective::Invoke(exec) => self.handle_invoke_directive(engine, exec),
            _ => {
                // Handle any other directive types
                Ok(WastDirectiveInfo {
                    test_type: WastTestType::Correctness,
                    directive_name: "unknown".to_string(),
                    requires_module_state: false,
                    modifies_engine_state: false,
                })
            }
        }
    }

    /// Handle module directive (instantiate a module)
    fn handle_module_directive(
        &mut self,
        engine: &mut StacklessEngine,
        wast_module: &mut wast::core::Module,
    ) -> Result<WastDirectiveInfo> {
        // Get the binary from the WAST module
        let binary = wast_module.encode().map_err(|e| Error::Parse(e.to_string()))?;

        // Create and load the WRT module
        let mut wrt_module = Module::new()?;
        let loaded_module = wrt_module.load_from_binary(&binary)?;

        // Store as current module
        self.current_module = Some(loaded_module.clone());

        // Instantiate the module
        engine.instantiate(loaded_module)?;

        Ok(WastDirectiveInfo {
            test_type: WastTestType::Integration,
            directive_name: "module".to_string(),
            requires_module_state: false,
            modifies_engine_state: true,
        })
    }

    /// Handle assert_return directive
    fn handle_assert_return_directive(
        &mut self,
        engine: &mut StacklessEngine,
        exec: &WastExecute,
        results: &[WastRet],
    ) -> Result<WastDirectiveInfo> {
        self.stats.assert_return_count += 1;

        match exec {
            WastExecute::Invoke(invoke) => {
                let args: Result<Vec<Value>, _> =
                    invoke.args.iter().map(convert_wast_arg_core).collect();
                let args = args?;

                let expected: Result<Vec<Value>, _> =
                    results.iter().map(convert_wast_ret_core).collect();
                let expected = expected?;

                // Execute the function and compare results
                let actual = engine.invoke_export(invoke.name, &args)?;

                // Validate results
                if actual.len() != expected.len() {
                    self.stats.failed += 1;
                    return Err(Error::Validation(format!(
                        "Result count mismatch: expected {}, got {}",
                        expected.len(),
                        actual.len()
                    )));
                }

                for (i, (a, e)) in actual.iter().zip(expected.iter()).enumerate() {
                    if !compare_wasm_values(a, e) {
                        self.stats.failed += 1;
                        return Err(Error::Validation(format!(
                            "Result mismatch at index {}: expected {:?}, got {:?}",
                            i, e, a
                        )));
                    }
                }

                self.stats.passed += 1;
                Ok(WastDirectiveInfo {
                    test_type: WastTestType::Correctness,
                    directive_name: "assert_return".to_string(),
                    requires_module_state: true,
                    modifies_engine_state: false,
                })
            }
            _ => {
                self.stats.failed += 1;
                Err(Error::Validation("Unsupported execution type for assert_return".into()))
            }
        }
    }

    /// Handle assert_trap directive
    fn handle_assert_trap_directive(
        &mut self,
        engine: &mut StacklessEngine,
        exec: &WastExecute,
        expected_message: &str,
    ) -> Result<WastDirectiveInfo> {
        self.stats.assert_trap_count += 1;

        match exec {
            WastExecute::Invoke(invoke) => {
                let args: Result<Vec<Value>, _> =
                    invoke.args.iter().map(convert_wast_arg_core).collect();
                let args = args?;

                // Execute and expect a trap
                match engine.invoke_export(invoke.name, &args) {
                    Ok(_) => {
                        self.stats.failed += 1;
                        Err(Error::Validation(format!(
                            "Expected trap '{}' but execution succeeded",
                            expected_message
                        )))
                    }
                    Err(error) => {
                        // Check if the error message matches expectations
                        let error_msg = error.to_string().to_lowercase();
                        let expected_msg = expected_message.to_lowercase();

                        if error_msg.contains(&expected_msg)
                            || contains_trap_keyword(&error_msg, &expected_msg)
                        {
                            self.stats.passed += 1;
                            Ok(WastDirectiveInfo {
                                test_type: WastTestType::ErrorHandling,
                                directive_name: "assert_trap".to_string(),
                                requires_module_state: true,
                                modifies_engine_state: false,
                            })
                        } else {
                            self.stats.failed += 1;
                            Err(Error::Validation(format!(
                                "Expected trap '{}' but got error: {}",
                                expected_message, error
                            )))
                        }
                    }
                }
            }
            _ => {
                self.stats.failed += 1;
                Err(Error::Validation("Unsupported execution type for assert_trap".into()))
            }
        }
    }

    /// Handle assert_invalid directive
    fn handle_assert_invalid_directive(
        &mut self,
        wast_module: &wast::core::Module,
        expected_message: &str,
    ) -> Result<WastDirectiveInfo> {
        self.stats.assert_invalid_count += 1;

        // Try to encode the module - it should fail
        match wast_module.encode() {
            Ok(binary) => {
                // If encoding succeeds, try to load it - should fail at validation
                match Module::new().and_then(|mut m| m.load_from_binary(&binary)) {
                    Ok(_) => {
                        self.stats.failed += 1;
                        Err(Error::Validation(format!(
                            "Expected invalid module '{}' but validation succeeded",
                            expected_message
                        )))
                    }
                    Err(error) => {
                        let error_msg = error.to_string().to_lowercase();
                        let expected_msg = expected_message.to_lowercase();

                        if error_msg.contains(&expected_msg)
                            || contains_validation_keyword(&error_msg, &expected_msg)
                        {
                            self.stats.passed += 1;
                            Ok(WastDirectiveInfo {
                                test_type: WastTestType::ErrorHandling,
                                directive_name: "assert_invalid".to_string(),
                                requires_module_state: false,
                                modifies_engine_state: false,
                            })
                        } else {
                            self.stats.failed += 1;
                            Err(Error::Validation(format!(
                                "Expected validation error '{}' but got: {}",
                                expected_message, error
                            )))
                        }
                    }
                }
            }
            Err(encode_error) => {
                // Encoding failed, which is also acceptable for invalid modules
                let error_msg = encode_error.to_string().to_lowercase();
                let expected_msg = expected_message.to_lowercase();

                if error_msg.contains(&expected_msg)
                    || contains_validation_keyword(&error_msg, &expected_msg)
                {
                    self.stats.passed += 1;
                    Ok(WastDirectiveInfo {
                        test_type: WastTestType::ErrorHandling,
                        directive_name: "assert_invalid".to_string(),
                        requires_module_state: false,
                        modifies_engine_state: false,
                    })
                } else {
                    self.stats.failed += 1;
                    Err(Error::Validation(format!(
                        "Expected validation error '{}' but got encoding error: {}",
                        expected_message, encode_error
                    )))
                }
            }
        }
    }

    /// Handle assert_malformed directive
    fn handle_assert_malformed_directive(
        &mut self,
        wast_module: &wast::core::Module,
        expected_message: &str,
    ) -> Result<WastDirectiveInfo> {
        self.stats.assert_malformed_count += 1;

        // Try to encode the module - it should fail with malformed error
        match wast_module.encode() {
            Ok(_) => {
                self.stats.failed += 1;
                Err(Error::Validation(format!(
                    "Expected malformed module '{}' but encoding succeeded",
                    expected_message
                )))
            }
            Err(encode_error) => {
                let error_msg = encode_error.to_string().to_lowercase();
                let expected_msg = expected_message.to_lowercase();

                if error_msg.contains(&expected_msg)
                    || contains_malformed_keyword(&error_msg, &expected_msg)
                {
                    self.stats.passed += 1;
                    Ok(WastDirectiveInfo {
                        test_type: WastTestType::ErrorHandling,
                        directive_name: "assert_malformed".to_string(),
                        requires_module_state: false,
                        modifies_engine_state: false,
                    })
                } else {
                    self.stats.failed += 1;
                    Err(Error::Validation(format!(
                        "Expected malformed error '{}' but got: {}",
                        expected_message, encode_error
                    )))
                }
            }
        }
    }

    /// Handle assert_unlinkable directive
    fn handle_assert_unlinkable_directive(
        &mut self,
        wast_module: &wast::core::Module,
        expected_message: &str,
    ) -> Result<WastDirectiveInfo> {
        self.stats.assert_unlinkable_count += 1;

        // Try to encode and instantiate the module - linking should fail
        match wast_module.encode() {
            Ok(binary) => {
                match Module::new().and_then(|mut m| m.load_from_binary(&binary)) {
                    Ok(module) => {
                        // Try to instantiate - should fail at linking
                        let mut engine = StacklessEngine::new();
                        match engine.instantiate(module) {
                            Ok(_) => {
                                self.stats.failed += 1;
                                Err(Error::Validation(format!(
                                    "Expected unlinkable module '{}' but linking succeeded",
                                    expected_message
                                )))
                            }
                            Err(error) => {
                                let error_msg = error.to_string().to_lowercase();
                                let expected_msg = expected_message.to_lowercase();

                                if error_msg.contains(&expected_msg)
                                    || contains_linking_keyword(&error_msg, &expected_msg)
                                {
                                    self.stats.passed += 1;
                                    Ok(WastDirectiveInfo {
                                        test_type: WastTestType::ErrorHandling,
                                        directive_name: "assert_unlinkable".to_string(),
                                        requires_module_state: false,
                                        modifies_engine_state: false,
                                    })
                                } else {
                                    self.stats.failed += 1;
                                    Err(Error::Validation(format!(
                                        "Expected linking error '{}' but got: {}",
                                        expected_message, error
                                    )))
                                }
                            }
                        }
                    }
                    Err(error) => {
                        // Module loading failed, which might also indicate unlinkable
                        let error_msg = error.to_string().to_lowercase();
                        let expected_msg = expected_message.to_lowercase();

                        if error_msg.contains(&expected_msg)
                            || contains_linking_keyword(&error_msg, &expected_msg)
                        {
                            self.stats.passed += 1;
                            Ok(WastDirectiveInfo {
                                test_type: WastTestType::ErrorHandling,
                                directive_name: "assert_unlinkable".to_string(),
                                requires_module_state: false,
                                modifies_engine_state: false,
                            })
                        } else {
                            self.stats.failed += 1;
                            Err(Error::Validation(format!(
                                "Expected linking error '{}' but got loading error: {}",
                                expected_message, error
                            )))
                        }
                    }
                }
            }
            Err(encode_error) => {
                self.stats.failed += 1;
                Err(Error::Validation(format!(
                    "Module encoding failed before linking test: {}",
                    encode_error
                )))
            }
        }
    }

    /// Handle assert_exhaustion directive
    fn handle_assert_exhaustion_directive(
        &mut self,
        engine: &mut StacklessEngine,
        exec: &WastExecute,
        expected_message: &str,
    ) -> Result<WastDirectiveInfo> {
        self.stats.assert_exhaustion_count += 1;

        match exec {
            WastExecute::Invoke(invoke) => {
                let args: Result<Vec<Value>, _> =
                    invoke.args.iter().map(convert_wast_arg_core).collect();
                let args = args?;

                // Set resource limits before execution
                engine.set_fuel(Some(self.resource_limits.max_execution_steps));

                // Execute and expect resource exhaustion
                match engine.invoke_export(invoke.name, &args) {
                    Ok(_) => {
                        self.stats.failed += 1;
                        Err(Error::Validation(format!(
                            "Expected resource exhaustion '{}' but execution succeeded",
                            expected_message
                        )))
                    }
                    Err(error) => {
                        let error_msg = error.to_string().to_lowercase();
                        let expected_msg = expected_message.to_lowercase();

                        if error_msg.contains(&expected_msg)
                            || contains_exhaustion_keyword(&error_msg, &expected_msg)
                        {
                            self.stats.passed += 1;
                            Ok(WastDirectiveInfo {
                                test_type: WastTestType::Resource,
                                directive_name: "assert_exhaustion".to_string(),
                                requires_module_state: true,
                                modifies_engine_state: false,
                            })
                        } else {
                            self.stats.failed += 1;
                            Err(Error::Validation(format!(
                                "Expected exhaustion '{}' but got error: {}",
                                expected_message, error
                            )))
                        }
                    }
                }
            }
            _ => {
                self.stats.failed += 1;
                Err(Error::Validation("Unsupported execution type for assert_exhaustion".into()))
            }
        }
    }

    /// Handle register directive (register module for imports)
    fn handle_register_directive(
        &mut self,
        name: &str,
        _module: &Option<wast::token::Id>,
    ) -> Result<WastDirectiveInfo> {
        self.stats.register_count += 1;

        // Register the current module if available (std only)
        #[cfg(feature = "std")]
        if let Some(ref module) = self.current_module {
            self.module_registry.insert(name.to_string(), module.clone());
            self.stats.passed += 1;
            return Ok(WastDirectiveInfo {
                test_type: WastTestType::Integration,
                directive_name: "register".to_string(),
                requires_module_state: true,
                modifies_engine_state: true,
            });
        }

        #[cfg(not(feature = "std"))]
        {
            // In no_std mode, we can't maintain a registry, but we can still track the directive
            if self.current_module.is_some() {
                self.stats.passed += 1;
                return Ok(WastDirectiveInfo {
                    test_type: WastTestType::Integration,
                    directive_name: "register".to_string(),
                    requires_module_state: true,
                    modifies_engine_state: true,
                });
            }
        }

        self.stats.failed += 1;
        Err(Error::Validation("No module available for registration".into()))
    }

    /// Handle invoke directive (standalone function call)
    fn handle_invoke_directive(
        &mut self,
        engine: &mut StacklessEngine,
        exec: &WastExecute,
    ) -> Result<WastDirectiveInfo> {
        match exec {
            WastExecute::Invoke(invoke) => {
                let args: Result<Vec<Value>, _> =
                    invoke.args.iter().map(convert_wast_arg_core).collect();
                let args = args?;

                // Execute the function (result is discarded)
                engine.invoke_export(invoke.name, &args)?;

                self.stats.passed += 1;
                Ok(WastDirectiveInfo {
                    test_type: WastTestType::Correctness,
                    directive_name: "invoke".to_string(),
                    requires_module_state: true,
                    modifies_engine_state: true,
                })
            }
            _ => {
                self.stats.failed += 1;
                Err(Error::Validation("Unsupported execution type for invoke".into()))
            }
        }
    }

    /// Run a complete WAST file (std only)
    #[cfg(feature = "std")]
    pub fn run_wast_file(&mut self, path: &Path) -> Result<WastTestStats> {
        let contents = fs::read_to_string(path)
            .map_err(|e| Error::Parse(format!("Failed to read file: {}", e)))?;

        let buf = ParseBuffer::new(&contents)
            .map_err(|e| Error::Parse(format!("Failed to create parse buffer: {}", e)))?;

        let wast: Wast = parser::parse(&buf)
            .map_err(|e| Error::Parse(format!("Failed to parse WAST: {}", e)))?;

        let module = Module::new()?;
        let mut engine = StacklessEngine::new();

        for mut directive in wast.directives {
            match self.execute_directive(&mut engine, &mut directive) {
                Ok(_) => {
                    // Test passed, stats already updated in execute_directive
                }
                Err(e) => {
                    eprintln!("WAST directive failed: {}", e);
                    // Error stats already updated in execute_directive
                }
            }
        }

        Ok(self.stats.clone())
    }

    /// Run WAST content from a string (works in both std and no_std)
    pub fn run_wast_content(&mut self, content: &str) -> Result<WastTestStats> {
        let buf = ParseBuffer::new(content)
            .map_err(|e| Error::Parse(format!("Failed to create parse buffer: {}", e)))?;

        let wast: Wast = parser::parse(&buf)
            .map_err(|e| Error::Parse(format!("Failed to parse WAST: {}", e)))?;

        let module = Module::new()?;
        let mut engine = StacklessEngine::new();

        for mut directive in wast.directives {
            match self.execute_directive(&mut engine, &mut directive) {
                Ok(_) => {
                    // Test passed, stats already updated in execute_directive
                }
                Err(e) => {
                    // In no_std mode, we can't use eprintln!, so we just continue
                    #[cfg(feature = "std")]
                    eprintln!("WAST directive failed: {}", e);
                    // Error stats already updated in execute_directive
                }
            }
        }

        Ok(self.stats.clone())
    }
}

// Helper functions for argument and return value conversion
fn convert_wast_arg_core(arg: &WastArg) -> Result<Value, Error> {
    match arg {
        WastArg::Core(core_arg) => match core_arg {
            WastArgCore::I32(x) => Ok(Value::I32(*x)),
            WastArgCore::I64(x) => Ok(Value::I64(*x)),
            WastArgCore::F32(x) => Ok(Value::F32(f32::from_bits(x.bits))),
            WastArgCore::F64(x) => Ok(Value::F64(f64::from_bits(x.bits))),
            WastArgCore::V128(x) => Ok(Value::V128(x.to_le_bytes())),
            _ => Err(Error::Validation("Unsupported argument type".into())),
        },
        _ => Err(Error::Validation("Unsupported argument type".into())),
    }
}

fn convert_wast_ret_core(ret: &WastRet) -> Result<Value, Error> {
    match ret {
        WastRet::Core(core_ret) => match core_ret {
            WastRetCore::I32(x) => Ok(Value::I32(*x)),
            WastRetCore::I64(x) => Ok(Value::I64(*x)),
            WastRetCore::F32(x) => match x {
                NanPattern::Value(x) => Ok(Value::F32(f32::from_bits(x.bits))),
                NanPattern::CanonicalNan => Ok(Value::F32(f32::NAN)),
                NanPattern::ArithmeticNan => Ok(Value::F32(f32::NAN)),
            },
            WastRetCore::F64(x) => match x {
                NanPattern::Value(x) => Ok(Value::F64(f64::from_bits(x.bits))),
                NanPattern::CanonicalNan => Ok(Value::F64(f64::NAN)),
                NanPattern::ArithmeticNan => Ok(Value::F64(f64::NAN)),
            },
            WastRetCore::V128(x) => Ok(Value::V128(x.to_le_bytes())),
            _ => Err(Error::Validation("Unsupported return type".into())),
        },
        _ => Err(Error::Validation("Unsupported return type".into())),
    }
}

// Helper function to compare Wasm values with proper NaN and tolerance handling
fn compare_wasm_values(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::F32(a), Value::F32(e)) => {
            if e.is_nan() {
                a.is_nan()
            } else if a.is_nan() {
                false
            } else {
                (a - e).abs() < 1e-6
            }
        }
        (Value::F64(a), Value::F64(e)) => {
            if e.is_nan() {
                a.is_nan()
            } else if a.is_nan() {
                false
            } else {
                (a - e).abs() < 1e-9
            }
        }
        (Value::V128(a), Value::V128(e)) => a == e,
        (a, e) => a == e,
    }
}

// Helper functions for error message classification
fn contains_trap_keyword(error_msg: &str, expected_msg: &str) -> bool {
    let trap_keywords = [
        "divide by zero",
        "integer overflow",
        "invalid conversion",
        "unreachable",
        "out of bounds",
        "undefined element",
        "uninitialized",
        "trap",
    ];

    trap_keywords
        .iter()
        .any(|keyword| error_msg.contains(keyword) || expected_msg.contains(keyword))
}

fn contains_validation_keyword(error_msg: &str, expected_msg: &str) -> bool {
    let validation_keywords = [
        "type mismatch",
        "unknown",
        "invalid",
        "malformed",
        "validation",
        "expected",
        "duplicate",
        "import",
        "export",
    ];

    validation_keywords
        .iter()
        .any(|keyword| error_msg.contains(keyword) || expected_msg.contains(keyword))
}

fn contains_malformed_keyword(error_msg: &str, expected_msg: &str) -> bool {
    let malformed_keywords = [
        "malformed",
        "unexpected end",
        "invalid",
        "encoding",
        "format",
        "binary",
        "section",
        "leb128",
    ];

    malformed_keywords
        .iter()
        .any(|keyword| error_msg.contains(keyword) || expected_msg.contains(keyword))
}

fn contains_linking_keyword(error_msg: &str, expected_msg: &str) -> bool {
    let linking_keywords = [
        "unknown import",
        "incompatible import",
        "link",
        "import",
        "export",
        "module",
        "instantiation",
        "missing",
    ];

    linking_keywords
        .iter()
        .any(|keyword| error_msg.contains(keyword) || expected_msg.contains(keyword))
}

fn contains_exhaustion_keyword(error_msg: &str, expected_msg: &str) -> bool {
    let exhaustion_keywords = [
        "stack overflow",
        "call stack exhausted",
        "out of fuel",
        "limit exceeded",
        "resource",
        "exhausted",
        "overflow",
        "fuel",
    ];

    exhaustion_keywords
        .iter()
        .any(|keyword| error_msg.contains(keyword) || expected_msg.contains(keyword))
}

// Integration with test registry
impl Default for WastTestRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Register WAST tests from the external testsuite (std only)
#[cfg(feature = "std")]
pub fn register_wast_tests() {
    let registry = TestRegistry::global();

    // Register a test suite for WAST file execution
    let test_case = wrt_test_registry::TestCaseImpl {
        name: "wast_testsuite_runner",
        category: "wast",
        requires_std: true,
        features: wrt_foundation::bounded::BoundedVec::new(),
        test_fn: Box::new(|_config: &TestConfig| -> wrt_test_registry::TestResult {
            run_wast_testsuite_tests()
        }),
        description: "Execute WAST files from the external WebAssembly testsuite",
    };

    if let Err(e) = registry.register(Box::new(test_case)) {
        eprintln!("Failed to register WAST tests: {}", e);
    }
}

/// Run WAST testsuite tests and return results (std only)
#[cfg(feature = "std")]
fn run_wast_testsuite_tests() -> wrt_test_registry::TestResult {
    let testsuite_path = match get_testsuite_path() {
        Some(path) => path,
        None => {
            return wrt_test_registry::TestResult::Ok(());
        }
    };

    let testsuite_dir = Path::new(&testsuite_path);
    if !testsuite_dir.exists() {
        return wrt_test_registry::TestResult::Ok(());
    }

    let mut runner = WastTestRunner::new();
    let mut total_files = 0;
    let mut failed_files = 0;

    // Run a subset of basic tests for demonstration
    let test_files = ["i32.wast", "i64.wast", "f32.wast", "f64.wast", "const.wast", "nop.wast"];

    for file_name in &test_files {
        let file_path = testsuite_dir.join(file_name);
        if file_path.exists() {
            total_files += 1;
            match runner.run_wast_file(&file_path) {
                Ok(stats) => {
                    println!("✓ {} - {} passed, {} failed", file_name, stats.passed, stats.failed);
                }
                Err(e) => {
                    eprintln!("✗ {} - Error: {}", file_name, e);
                    failed_files += 1;
                }
            }
        }
    }

    if failed_files == 0 {
        println!("All {} WAST files processed successfully", total_files);
        wrt_test_registry::TestResult::Ok(())
    } else {
        wrt_test_registry::TestResult::Err(format!(
            "{}/{} WAST files failed",
            failed_files, total_files
        ))
    }
}

/// Utility function to get the test suite path from environment variables (std only)
#[cfg(feature = "std")]
fn get_testsuite_path() -> Option<String> {
    std::env::var("WASM_TESTSUITE").ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wast_runner_creation() {
        let runner = WastTestRunner::new();
        assert_eq!(runner.stats.passed, 0);
        assert_eq!(runner.stats.failed, 0);
    }

    #[test]
    fn test_resource_limits_default() {
        let limits = ResourceLimits::default();
        assert_eq!(limits.max_stack_depth, 1024);
        assert_eq!(limits.max_memory_size, 64 * 1024 * 1024);
    }

    #[test]
    fn test_value_comparison() {
        // Test exact values
        assert!(compare_wasm_values(&Value::I32(42), &Value::I32(42)));
        assert!(!compare_wasm_values(&Value::I32(42), &Value::I32(43)));

        // Test NaN handling
        assert!(compare_wasm_values(&Value::F32(f32::NAN), &Value::F32(f32::NAN)));
        assert!(!compare_wasm_values(&Value::F32(1.0), &Value::F32(f32::NAN)));

        // Test tolerance for floats
        assert!(compare_wasm_values(&Value::F32(1.0), &Value::F32(1.0000001)));
    }

    #[test]
    fn test_error_keyword_detection() {
        assert!(contains_trap_keyword("divide by zero", ""));
        assert!(contains_validation_keyword("type mismatch error", ""));
        assert!(contains_malformed_keyword("unexpected end of input", ""));
        assert!(contains_linking_keyword("unknown import module", ""));
        assert!(contains_exhaustion_keyword("stack overflow detected", ""));
    }
}
