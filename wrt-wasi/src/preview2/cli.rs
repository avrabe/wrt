//! WASI CLI interface implementation
//!
//! Implements the `wasi:cli` interface for command line arguments and environment
//! variables using WRT's platform abstractions.

use crate::prelude::*;
use wrt_component::values::Value;
use core::any::Any;

/// WASI get arguments operation
///
/// Implements `wasi:cli/environment.get-arguments` using platform abstractions
pub fn wasi_cli_get_arguments(
    _target: &mut dyn Any,
    _args: Vec<Value>,
) -> Result<Vec<Value>> {
    // Get command line arguments using platform abstraction
    #[cfg(feature = "std")]
    {
        use std::env;
        let args: Vec<String> = env::args().collect();
        
        // Convert to WASI list<string>
        let wasi_args: Vec<Value> = args.into_iter()
            .map(Value::String)
            .collect();
            
        Ok(vec![Value::List(wasi_args)])
    }
    
    #[cfg(not(feature = "std"))]
    {
        // In no_std environment, return empty args or predefined args
        // This would typically be configured at compile time or through
        // embedded system configuration
        let empty_args = Vec::new();
        Ok(vec![Value::List(empty_args)])
    }
}

/// WASI get environment operation
///
/// Implements `wasi:cli/environment.get-environment` for environment variables
pub fn wasi_cli_get_environment(
    _target: &mut dyn Any,
    _args: Vec<Value>,
) -> Result<Vec<Value>> {
    // Get environment variables using platform abstraction
    #[cfg(feature = "std")]
    {
        use std::env;
        
        let mut env_vars = Vec::new();
        
        // Iterate through environment variables
        for (key, value) in env::vars() {
            // Create tuple of (key, value)
            let env_tuple = Value::Tuple(vec![
                Value::String(key),
                Value::String(value),
            ]);
            env_vars.push(env_tuple);
        }
        
        Ok(vec![Value::List(env_vars)])
    }
    
    #[cfg(not(feature = "std"))]
    {
        // In no_std environment, return empty environment or predefined vars
        let empty_env = Vec::new();
        Ok(vec![Value::List(empty_env)])
    }
}

/// WASI get initial working directory operation
///
/// Implements `wasi:cli/environment.initial-cwd` for current working directory
pub fn wasi_get_initial_cwd(
    _target: &mut dyn Any,
    _args: Vec<Value>,
) -> Result<Vec<Value>> {
    #[cfg(feature = "std")]
    {
        use std::env;
        
        match env::current_dir() {
            Ok(cwd) => {
                let cwd_string = cwd.to_string_lossy().to_string();
                Ok(vec![Value::Option(Some(Box::new(Value::String(cwd_string))))])
            }
            Err(_) => {
                // Return None if current directory cannot be determined
                Ok(vec![Value::Option(None)])
            }
        }
    }
    
    #[cfg(not(feature = "std"))]
    {
        // In no_std environment, return None or a predefined directory
        Ok(vec![Value::Option(None)])
    }
}

/// WASI get terminal stdin information
///
/// Implements `wasi:cli/terminal-stdin.get-terminal-stdin` for terminal detection
pub fn wasi_get_terminal_stdin(
    _target: &mut dyn Any,
    _args: Vec<Value>,
) -> Result<Vec<Value>> {
    #[cfg(feature = "std")]
    {
        use std::io::{self, IsTerminal};
        
        // Check if stdin is a terminal
        let is_terminal = io::stdin().is_terminal();
        
        if is_terminal {
            // Return some terminal handle (simplified)
            Ok(vec![Value::Option(Some(Box::new(Value::U32(0))))])
        } else {
            Ok(vec![Value::Option(None)])
        }
    }
    
    #[cfg(not(feature = "std"))]
    {
        // In no_std environment, typically no terminal
        Ok(vec![Value::Option(None)])
    }
}

/// WASI get terminal stdout information
///
/// Implements `wasi:cli/terminal-stdout.get-terminal-stdout` for terminal detection
pub fn wasi_get_terminal_stdout(
    _target: &mut dyn Any,
    _args: Vec<Value>,
) -> Result<Vec<Value>> {
    #[cfg(feature = "std")]
    {
        use std::io::{self, IsTerminal};
        
        // Check if stdout is a terminal
        let is_terminal = io::stdout().is_terminal();
        
        if is_terminal {
            // Return some terminal handle (simplified)
            Ok(vec![Value::Option(Some(Box::new(Value::U32(1))))])
        } else {
            Ok(vec![Value::Option(None)])
        }
    }
    
    #[cfg(not(feature = "std"))]
    {
        // In no_std environment, typically no terminal
        Ok(vec![Value::Option(None)])
    }
}

/// WASI get terminal stderr information
///
/// Implements `wasi:cli/terminal-stderr.get-terminal-stderr` for terminal detection
pub fn wasi_get_terminal_stderr(
    _target: &mut dyn Any,
    _args: Vec<Value>,
) -> Result<Vec<Value>> {
    #[cfg(feature = "std")]
    {
        use std::io::{self, IsTerminal};
        
        // Check if stderr is a terminal
        let is_terminal = io::stderr().is_terminal();
        
        if is_terminal {
            // Return some terminal handle (simplified)
            Ok(vec![Value::Option(Some(Box::new(Value::U32(2))))])
        } else {
            Ok(vec![Value::Option(None)])
        }
    }
    
    #[cfg(not(feature = "std"))]
    {
        // In no_std environment, typically no terminal
        Ok(vec![Value::Option(None)])
    }
}

/// Get filtered environment variables based on capabilities
///
/// Helper function to filter environment variables based on WASI capabilities
pub fn get_filtered_environment(capabilities: &WasiEnvironmentCapabilities) -> Result<Vec<(String, String)>> {
    let mut filtered_vars = Vec::new();
    
    if !capabilities.environ_access {
        return Ok(filtered_vars);
    }
    
    #[cfg(feature = "std")]
    {
        use std::env;
        
        for (key, value) in env::vars() {
            if capabilities.is_env_var_allowed(&key) {
                filtered_vars.push((key, value));
            }
        }
    }
    
    Ok(filtered_vars)
}

/// Get filtered command line arguments based on capabilities
///
/// Helper function to filter arguments based on WASI capabilities
pub fn get_filtered_arguments(capabilities: &WasiEnvironmentCapabilities) -> Result<Vec<String>> {
    if !capabilities.args_access {
        return Ok(Vec::new());
    }
    
    #[cfg(feature = "std")]
    {
        use std::env;
        Ok(env::args().collect())
    }
    
    #[cfg(not(feature = "std"))]
    {
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_wasi_get_arguments() -> Result<()> {
        let result = wasi_cli_get_arguments(&mut (), vec![])?;
        assert_eq!(result.len(), 1);
        
        // Should return a list
        if let Value::List(_args) = &result[0] {
            // Arguments should be a list of strings
            // In test environment, may be empty or contain test runner args
        } else {
            panic!("Expected list of arguments");
        }
        
        Ok(())
    }
    
    #[test]
    fn test_wasi_get_environment() -> Result<()> {
        let result = wasi_cli_get_environment(&mut (), vec![])?;
        assert_eq!(result.len(), 1);
        
        // Should return a list of tuples
        if let Value::List(env_vars) = &result[0] {
            // Each environment variable should be a tuple of (key, value)
            for env_var in env_vars {
                if let Value::Tuple(tuple) = env_var {
                    assert_eq!(tuple.len(), 2);
                    // Both should be strings
                    assert!(matches!(tuple[0], Value::String(_)));
                    assert!(matches!(tuple[1], Value::String(_)));
                }
            }
        } else {
            panic!("Expected list of environment variables");
        }
        
        Ok(())
    }
    
    #[test]
    fn test_wasi_get_initial_cwd() -> Result<()> {
        let result = wasi_get_initial_cwd(&mut (), vec![])?;
        assert_eq!(result.len(), 1);
        
        // Should return an option
        if let Value::Option(cwd_opt) = &result[0] {
            // May be Some(string) or None depending on environment
            if let Some(cwd_value) = cwd_opt {
                assert!(matches!(**cwd_value, Value::String(_)));
            }
        } else {
            panic!("Expected option for current working directory");
        }
        
        Ok(())
    }
    
    #[test]
    fn test_filtered_environment() -> Result<()> {
        let mut capabilities = WasiEnvironmentCapabilities::minimal()?;
        capabilities.environ_access = true;
        capabilities.add_allowed_var("PATH")?;
        
        let filtered = get_filtered_environment(&capabilities)?;
        
        // Should only contain PATH if it exists in environment
        for (key, _value) in &filtered {
            assert_eq!(key, "PATH");
        }
        
        Ok(())
    }
    
    #[test]
    fn test_filtered_arguments() -> Result<()> {
        let mut capabilities = WasiEnvironmentCapabilities::minimal()?;
        capabilities.args_access = true;
        
        let filtered = get_filtered_arguments(&capabilities)?;
        
        // Should return arguments when access is allowed
        // In test environment, may contain test runner arguments
        
        // Test with access disabled
        capabilities.args_access = false;
        let no_args = get_filtered_arguments(&capabilities)?;
        assert!(no_args.is_empty());
        
        Ok(())
    }
}