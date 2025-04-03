wit_bindgen::generate!({
    // Use the world defined in adapter.wit in logging-adapter/wit
    world: "adapter",
    path: "wit", // Point back to the crate's local wit directory
    export_macro_name: "export_adapter",
    // Tell wit-bindgen to generate bindings for imported interfaces
    with: {
        // Use the correct versions
        "wasi:cli/stderr@0.2.0": generate,
        "wasi:io/streams@0.2.0": generate,
    }
});

// Bring the imported interfaces into scope
use crate::wasi::cli::stderr;
use crate::wasi::io::streams;
// Use the Guest trait for the exported interface
use crate::exports::wasi::logging::logging::{Guest, Level};

struct LoggingAdapter;

// Implement the exported logging interface
impl Guest for LoggingAdapter {
    fn log(level: Level, context: String, message: String) {
        // Get the stderr stream provided by the host (imported)
        let stream: streams::OutputStream = stderr::get_stderr();

        // Format the log message (example format)
        let level_str = match level {
            Level::Trace => "TRACE",
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
            Level::Critical => "CRITICAL",
        };
        let formatted_message = format!("[{}] {}: {}\n", level_str, context, message);

        // Write the message to the stream
        // Note: This assumes blocking write is okay. Error handling is minimal.
        match stream.write(formatted_message.as_bytes()) {
            Ok(_) => {
                // Optionally flush the stream if needed, though stderr might auto-flush
                let _ = stream.flush();
            }
            Err(_) => {
                // Basic error handling: If we can't write to stderr, there's not much we can do
                // Maybe try a trap in a real scenario?
            }
        }
    }
}

// Export the implementation
export_adapter!(LoggingAdapter);
