//! String encoding integration tests

use wrt_test_registry::prelude::*;

pub fn run_tests() -> TestResult {
    let mut suite = TestSuite::new("String Encoding");

    suite.add_test("utf8_encoding", test_utf8_encoding);
    suite.add_test("utf16_encoding", test_utf16_encoding);
    suite.add_test("latin1_encoding", test_latin1_encoding);
    suite.add_test("encoding_errors", test_encoding_errors);

    suite.run().into()
}

fn test_utf8_encoding() -> RegistryTestResult {
    Ok(())
}

fn test_utf16_encoding() -> RegistryTestResult {
    Ok(())
}

fn test_latin1_encoding() -> RegistryTestResult {
    Ok(())
}

fn test_encoding_errors() -> RegistryTestResult {
    Ok(())
}
