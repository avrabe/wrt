//! Firewall strategy for intercepting component function calls
//!
//! This strategy enforces security rules for function calls between
//! components and hosts. It can allow or deny calls based on various criteria.

use crate::prelude::*;

/// A rule to enforce on function calls
#[derive(Debug, Clone)]
pub enum FirewallRule {
    /// Allow a specific function to be called (source, target, function)
    AllowFunction(String, String, String),
    /// Allow all functions from a source to a target
    AllowSource(String, String),
    /// Allow all functions to a target
    AllowTarget(String),
    /// Deny a specific function (source, target, function)
    DenyFunction(String, String, String),
    /// Deny all functions from a source to a target
    DenySource(String, String),
    /// Deny all functions to a target
    DenyTarget(String),
}

/// Configuration for the firewall strategy
#[derive(Debug, Clone, Default)]
pub struct FirewallConfig {
    /// Default policy (true = allow by default, false = deny by default)
    pub default_allow: bool,
    /// Rules to enforce
    pub rules: Vec<FirewallRule>,
    /// Whether to check function parameters
    pub check_parameters: bool,
}

/// A strategy that enforces security rules on function calls
pub struct FirewallStrategy {
    /// Configuration for this strategy
    config: FirewallConfig,
    /// Cache of allowed function calls for performance
    #[cfg(feature = "std")]
    allowed_functions: RwLock<HashSet<String>>,
    /// Cache of denied function calls for performance
    #[cfg(feature = "std")]
    denied_functions: RwLock<HashSet<String>>,
}

impl FirewallStrategy {
    /// Create a new firewall strategy with the given configuration
    pub fn new(config: FirewallConfig) -> Self {
        Self {
            config,
            #[cfg(feature = "std")]
            allowed_functions: RwLock::new(HashSet::new()),
            #[cfg(feature = "std")]
            denied_functions: RwLock::new(HashSet::new()),
        }
    }

    /// Helper function to generate a unique key for a function call
    fn function_key(source: &str, target: &str, function: &str) -> String {
        format!("{}->{}::{}", source, target, function)
    }

    /// Check if a function call is allowed
    #[cfg(feature = "std")]
    fn is_allowed(&self, source: &str, target: &str, function: &str) -> bool {
        let key = Self::function_key(source, target, function);

        // Check cache first
        if let Ok(allowed) = self.allowed_functions.read() {
            if allowed.contains(&key) {
                return true;
            }
        }

        if let Ok(denied) = self.denied_functions.read() {
            if denied.contains(&key) {
                return false;
            }
        }

        // Not in cache, apply rules
        let allowed = self.apply_rules(source, target, function);

        // Update cache
        if allowed {
            if let Ok(mut allowed_cache) = self.allowed_functions.write() {
                allowed_cache.insert(key);
            }
        } else if let Ok(mut denied_cache) = self.denied_functions.write() {
            denied_cache.insert(key);
        }

        allowed
    }

    /// Apply rules to determine if a function call is allowed
    #[cfg(feature = "std")]
    fn apply_rules(&self, source: &str, target: &str, function: &str) -> bool {
        // Start with default policy
        let mut allowed = self.config.default_allow;

        // Apply rules in order
        for rule in &self.config.rules {
            match rule {
                FirewallRule::AllowFunction(s, t, f) => {
                    if s == source && t == target && f == function {
                        allowed = true;
                    }
                }
                FirewallRule::AllowSource(s, t) => {
                    if s == source && t == target {
                        allowed = true;
                    }
                }
                FirewallRule::AllowTarget(t) => {
                    if t == target {
                        allowed = true;
                    }
                }
                FirewallRule::DenyFunction(s, t, f) => {
                    if s == source && t == target && f == function {
                        allowed = false;
                    }
                }
                FirewallRule::DenySource(s, t) => {
                    if s == source && t == target {
                        allowed = false;
                    }
                }
                FirewallRule::DenyTarget(t) => {
                    if t == target {
                        allowed = false;
                    }
                }
            }
        }

        allowed
    }
}

impl LinkInterceptorStrategy for FirewallStrategy {
    fn before_call(
        &self,
        source: &str,
        target: &str,
        function: &str,
        args: &[Value],
    ) -> Result<Vec<Value>> {
        #[cfg(feature = "std")]
        {
            // Check if the function call is allowed
            if !self.is_allowed(source, target, function) {
                return Err(Error::new(
                    ErrorCategory::Runtime,
                    codes::RUNTIME_ERROR,
                    format!(
                        "Security error: Function call from '{}' to '{}::{}' is not allowed by firewall policy",
                        source, target, function
                    )
                ));
            }
        }

        // In no_std mode, we use a simpler implementation that applies rules directly
        #[cfg(all(not(feature = "std"), feature = "alloc"))]
        {
            // Start with default policy
            let mut allowed = self.config.default_allow;

            // Apply rules in order
            for rule in &self.config.rules {
                match rule {
                    FirewallRule::AllowFunction(s, t, f) => {
                        if s == source && t == target && f == function {
                            allowed = true;
                        }
                    }
                    FirewallRule::AllowSource(s, t) => {
                        if s == source && t == target {
                            allowed = true;
                        }
                    }
                    FirewallRule::AllowTarget(t) => {
                        if t == target {
                            allowed = true;
                        }
                    }
                    FirewallRule::DenyFunction(s, t, f) => {
                        if s == source && t == target && f == function {
                            allowed = false;
                        }
                    }
                    FirewallRule::DenySource(s, t) => {
                        if s == source && t == target {
                            allowed = false;
                        }
                    }
                    FirewallRule::DenyTarget(t) => {
                        if t == target {
                            allowed = false;
                        }
                    }
                }
            }

            if !allowed {
                return Err(Error::new(
                    ErrorCategory::Runtime,
                    codes::RUNTIME_ERROR,
                    format!(
                        "Security error: Function call from '{}' to '{}::{}' is not allowed by firewall policy",
                        source, target, function
                    )
                ));
            }
        }

        // Check parameters if configured
        if self.config.check_parameters {
            // Parameter checking logic would go here
            // For example, you could check for malicious input patterns
        }

        // Return unmodified arguments
        Ok(args.to_vec())
    }

    fn after_call(
        &self,
        _source: &str,
        _target: &str,
        _function: &str,
        _args: &[Value],
        result: Result<Vec<Value>>,
    ) -> Result<Vec<Value>> {
        // Return unmodified result
        result
    }

    fn clone_strategy(&self) -> Arc<dyn LinkInterceptorStrategy> {
        Arc::new(Self {
            config: self.config.clone(),
            #[cfg(feature = "std")]
            allowed_functions: RwLock::new(HashSet::new()),
            #[cfg(feature = "std")]
            denied_functions: RwLock::new(HashSet::new()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_firewall_allow_by_default() {
        let config = FirewallConfig {
            default_allow: true,
            rules: vec![FirewallRule::DenyFunction(
                "source".to_string(),
                "target".to_string(),
                "denied_function".to_string(),
            )],
            check_parameters: false,
        };
        let strategy = FirewallStrategy::new(config);

        // Test allowed function
        let result = strategy.before_call("source", "target", "allowed_function", &[]);
        assert!(result.is_ok());

        // Test denied function
        let result = strategy.before_call("source", "target", "denied_function", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_firewall_deny_by_default() {
        let config = FirewallConfig {
            default_allow: false,
            rules: vec![FirewallRule::AllowFunction(
                "source".to_string(),
                "target".to_string(),
                "allowed_function".to_string(),
            )],
            check_parameters: false,
        };
        let strategy = FirewallStrategy::new(config);

        // Test allowed function
        let result = strategy.before_call("source", "target", "allowed_function", &[]);
        assert!(result.is_ok());

        // Test denied function
        let result = strategy.before_call("source", "target", "denied_function", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_firewall_allow_source() {
        let config = FirewallConfig {
            default_allow: false,
            rules: vec![FirewallRule::AllowSource("source".to_string(), "target".to_string())],
            check_parameters: false,
        };
        let strategy = FirewallStrategy::new(config);

        // Test allowed source
        let result = strategy.before_call("source", "target", "any_function", &[]);
        assert!(result.is_ok());

        // Test denied source
        let result = strategy.before_call("other_source", "target", "any_function", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_firewall_rule_precedence() {
        let config = FirewallConfig {
            default_allow: false,
            rules: vec![
                FirewallRule::AllowSource("source".to_string(), "target".to_string()),
                FirewallRule::DenyFunction(
                    "source".to_string(),
                    "target".to_string(),
                    "denied_function".to_string(),
                ),
            ],
            check_parameters: false,
        };
        let strategy = FirewallStrategy::new(config);

        // Test allowed function
        let result = strategy.before_call("source", "target", "allowed_function", &[]);
        assert!(result.is_ok());

        // Test denied function
        let result = strategy.before_call("source", "target", "denied_function", &[]);
        assert!(result.is_err());
    }
}
