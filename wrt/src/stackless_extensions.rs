use crate::{
    error::{Error, Result},
    module::ExportKind,
    stackless::StacklessEngine,
    values::Value,
};

impl StacklessEngine {
    /// Invokes an exported function by name with the given arguments
    pub fn invoke_export(&mut self, name: &str, args: &[Value]) -> Result<Vec<Value>> {
        // Debug print for troubleshooting
        println!("DEBUG: execute called for function: add");
        println!("DEBUG: invoke_export called with name: {name}, args: {args:?}");

        // Find instance with the export
        let mut instance_idx = None;
        let mut export_func_idx = None;

        // Print out all available exports for debugging
        for (idx, instance) in self.instances.iter().enumerate() {
            println!("DEBUG: Checking instance {idx} for export {name}");
            for export in &instance.module.exports {
                println!(
                    "DEBUG: Function exports: {} (kind: {:?})",
                    export.name, export.kind
                );
                if export.name == name {
                    match export.kind {
                        ExportKind::Function => {
                            instance_idx = Some(idx);
                            export_func_idx = Some(export.index);
                            println!(
                                "DEBUG: Found matching export: {} (index {})",
                                name, export.index
                            );
                            break;
                        }
                        _ => continue,
                    }
                }
            }
            if instance_idx.is_some() {
                break;
            }
        }

        // If we couldn't find the export, return an error
        let instance_idx = instance_idx.ok_or_else(|| Error::ExportNotFound(name.to_string()))?;
        let func_idx = export_func_idx
            .ok_or_else(|| Error::Execution(format!("Export {name} is not a function")))?;

        // Convert the arguments to a Vec for the call
        let args_vec = args.to_vec();
        println!("DEBUG: Calling execute with instance_idx: {instance_idx}, func_idx: {func_idx}, args: {args_vec:?}");

        // Execute the function
        let result = self.execute(instance_idx, func_idx, args_vec);

        // Log the result
        match &result {
            Ok(values) => {
                println!("DEBUG: execute_function returned successfully with values: {values:?}");
            }
            Err(e) => println!("DEBUG: execute_function failed with error: {e:?}"),
        }

        result
    }
}
