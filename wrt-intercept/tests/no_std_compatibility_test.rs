//! Test no_std compatibility for wrt-intercept
//!
//! This file validates that the wrt-intercept crate works correctly in no_std environments.

// For testing in a no_std environment
#![cfg_attr(not(feature = "std"), no_std)]

// External crate imports
#[cfg(all(not(feature = "std"), feature = "alloc"))]
extern crate alloc;

// Global imports for the test file
use alloc::boxed::Box;
use alloc::collections::BTreeMap as HashMap; // For no_std contexts where HashMap might be BTreeMap
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec;
use core::sync::atomic::{AtomicUsize, Ordering};

// Import directly from the wrt_intercept crate
use wrt_intercept::{
    builtins::{BuiltinSerialization, InterceptContext},
    prelude::{
        self as wrt_prelude, codes, BeforeBuiltinResult, BuiltinType, ComponentValue, Error,
        ErrorCategory, InterceptionResult, LinkInterceptor, LinkInterceptorStrategy, LogSink,
        Modification, Result, ValType, Value,
    },
    strategies::{
        self as intercept_strategies, DefaultValueFormatter, FirewallConfig, FirewallRule,
        FirewallStrategy, LoggingConfig,
    }, // aliased strategies
};

#[cfg(feature = "std")]
use wrt_intercept::strategies::StatisticsStrategy;

#[cfg(test)]
mod tests {
    // Use explicit wrt_intercept:: paths
    use wrt_intercept::{
        builtins::{BuiltinSerialization, InterceptContext},
        prelude::{
            self as wrt_prelude, codes, BeforeBuiltinResult, BuiltinType, ComponentValue, Error,
            ErrorCategory, InterceptionResult, LinkInterceptor, LinkInterceptorStrategy, LogSink,
            Modification, Result, ValType, Value,
        },
        strategies::{
            self as intercept_strategies, DefaultValueFormatter, FirewallConfig, FirewallRule,
            FirewallStrategy, LoggingConfig,
        },
    };

    #[cfg(feature = "std")]
    use wrt_intercept::strategies::StatisticsStrategy; // Also inside mod tests for consistency if used here

    // Global alloc/core imports are at the top of the file
    // These are now correctly resolved by the compiler from the top-level imports.
    // For example, Arc will be alloc::sync::Arc.

    // Dummy log sink for testing LoggingStrategy in no_std
    #[derive(Clone)]
    struct NoStdTestSink {
        count: Arc<AtomicUsize>,
    }
    impl NoStdTestSink {
        fn new() -> Self {
            Self { count: Arc::new(AtomicUsize::new(0)) }
        }
    }

    impl LogSink for NoStdTestSink {
        fn write_log(&self, _entry: &str) {
            self.count.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn test_firewall_strategy_no_std() {
        let rule = FirewallRule::AllowFunction(
            "component_a".to_string(),
            "component_b".to_string(),
            "test_function".to_string(),
        );
        let mut config = FirewallConfig::default();
        config.rules.push(rule);
        config.default_allow = false;

        let _strategy = FirewallStrategy::new(config);
    }

    #[test]
    fn test_logging_strategy_no_std() {
        let sink = Arc::new(NoStdTestSink::new());
        let config = LoggingConfig::default();
        let _strategy = intercept_strategies::LoggingStrategy::with_formatter(
            sink.clone(),
            DefaultValueFormatter,
        )
        .with_config(config);
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_statistics_strategy() {
        let _strategy = StatisticsStrategy::new();
    }

    #[test]
    fn test_intercept_context_no_std() {
        let context =
            InterceptContext::new("test_component", BuiltinType::ResourceCreate, "test_host_id");

        assert_eq!(context.component_name, "test_component");
        assert_eq!(context.builtin_type, BuiltinType::ResourceCreate);
        assert_eq!(context.host_id, "test_host_id");
    }

    #[test]
    fn test_component_value_no_std() {
        let val = ComponentValue::S32(42);
        match val {
            ComponentValue::S32(i) => assert_eq!(i, 42),
            _ => panic!("Unexpected ComponentValue variant"),
        }
    }

    #[test]
    fn test_builtin_serialization_no_std() {
        let values = vec![ComponentValue::S32(10), ComponentValue::F64(20.5)];
        let types = vec![ValType::S32, ValType::F64];

        let serialized = BuiltinSerialization::serialize(&values).unwrap();
        let deserialized = BuiltinSerialization::deserialize(&serialized, &types).unwrap();

        assert_eq!(values, deserialized);
    }

    #[test]
    fn test_modification_no_std() {
        let _replace = Modification::Replace { offset: 0, data: vec![1, 2, 3] };
        let _insert = Modification::Insert { offset: 0, data: vec![4, 5, 6] };
        let _remove = Modification::Remove { offset: 0, length: 3 };
    }

    #[test]
    fn test_interception_result_no_std() {
        let result = InterceptionResult { modified: false, modifications: Vec::new() };
        assert!(!result.modified);
    }

    #[test]
    fn test_before_builtin_result_no_std() {
        let _continue_result = BeforeBuiltinResult::Continue(vec![ComponentValue::S32(1)]);
        let _bypass_result = BeforeBuiltinResult::Bypass(vec![ComponentValue::S32(2)]);
        format!("{:?}", _continue_result);
        format!("{:?}", _bypass_result);
    }

    #[derive(Clone)]
    struct NoStdTestStrategy;

    impl LinkInterceptorStrategy for NoStdTestStrategy {
        fn before_call(
            &self,
            _source: &str,
            _target: &str,
            _function: &str,
            args: &[Value],
        ) -> wrt_prelude::Result<Vec<Value>> {
            Ok(args.to_vec())
        }

        fn after_call(
            &self,
            _source: &str,
            _target: &str,
            _function: &str,
            _args: &[Value],
            result: wrt_prelude::Result<Vec<Value>>,
        ) -> wrt_prelude::Result<Vec<Value>> {
            result
        }

        fn clone_strategy(&self) -> Arc<dyn LinkInterceptorStrategy> {
            Arc::new(self.clone())
        }
    }

    #[test]
    fn test_link_interceptor_no_std() {
        let mut interceptor = LinkInterceptor::new("no_std_test_interceptor");
        let strategy = Arc::new(NoStdTestStrategy);
        interceptor.add_strategy(strategy);

        assert_eq!(interceptor.name(), "no_std_test_interceptor");
        assert!(interceptor.get_strategy().is_some());
    }
}
