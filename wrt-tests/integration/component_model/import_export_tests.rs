//! Import/Export integration tests

use wrt_test_registry::prelude::*;

pub fn run_tests() -> TestResult {
    let mut suite = TestSuite::new("Import/Export");

    suite.add_test("basic_imports", test_basic_imports);
    suite.add_test("basic_exports", test_basic_exports);
    suite.add_test("nested_imports", test_nested_imports);
    suite.add_test("import_resolution", test_import_resolution);

    suite.run().into()
}

fn test_basic_imports() -> RegistryTestResult {
    Ok(())
}

fn test_basic_exports() -> RegistryTestResult {
    Ok(())
}

fn test_nested_imports() -> RegistryTestResult {
    Ok(())
}

fn test_import_resolution() -> RegistryTestResult {
    Ok(())
}
