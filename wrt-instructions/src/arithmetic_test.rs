// Only run arithmetic tests when alloc is available
#[cfg(any(feature = "std", feature = "alloc"))]
mod arithmetic_tests {
    use crate::prelude::*;
    use wrt_error::{codes, ErrorCategory};
    use crate::{
        arithmetic_ops::{ArithmeticContext, ArithmeticOp},
        instruction_traits::PureInstruction,
        Error, Value,
    };

    struct SimpleContext {
        stack: Vec<Value>,
    }

    impl SimpleContext {
        fn new() -> Self {
            Self { stack: Vec::new() }
        }
    }

    impl ArithmeticContext for SimpleContext {
        fn push_arithmetic_value(&mut self, value: Value) -> crate::Result<()> {
            self.stack.push(value);
            Ok(())
        }

        fn pop_arithmetic_value(&mut self) -> crate::Result<Value> {
            self.stack.pop().ok_or_else(|| {
                Error::new(ErrorCategory::Runtime, codes::STACK_UNDERFLOW, "Stack underflow")
            })
        }
    }

    #[test]
    fn test_i32_add() {
        let mut context = SimpleContext::new();

        // Test i32.add
        context.push_arithmetic_value(Value::I32(2)).unwrap();
        context.push_arithmetic_value(Value::I32(3)).unwrap();
        ArithmeticOp::I32Add.execute(&mut context).unwrap();
        assert_eq!(context.pop_arithmetic_value().unwrap(), Value::I32(5));
    }
}
