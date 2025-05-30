/// Basic stack trace support for debugging
/// Provides the missing 3% for stack trace generation
use wrt_foundation::{
    bounded::{BoundedVec, MAX_DWARF_FILE_TABLE},
    NoStdProvider,
};

/// A single frame in a stack trace
#[derive(Debug, Clone)]
pub struct StackFrame<'a> {
    /// Program counter for this frame
    pub pc: u32,
    /// Function information (if available)
    pub function: Option<&'a crate::FunctionInfo<'a>>,
    /// Line information (if available)
    pub line_info: Option<crate::LineInfo>,
    /// Frame depth (0 = current frame)
    pub depth: u16,
}

/// Stack trace builder
pub struct StackTrace<'a> {
    /// Collection of stack frames
    frames: BoundedVec<StackFrame<'a>, MAX_DWARF_FILE_TABLE, NoStdProvider>,
}

impl<'a> StackTrace<'a> {
    /// Create a new empty stack trace
    pub fn new() -> Self {
        Self { frames: BoundedVec::new(NoStdProvider) }
    }

    /// Add a frame to the stack trace
    pub fn push_frame(&mut self, frame: StackFrame<'a>) -> Result<(), ()> {
        self.frames.push(frame).map_err(|_| ())
    }

    /// Get the frames in the trace
    pub fn frames(&self) -> &[StackFrame<'a>] {
        self.frames.as_slice()
    }

    /// Get the current frame (depth 0)
    pub fn current_frame(&self) -> Option<&StackFrame<'a>> {
        self.frames.first()
    }

    /// Get the number of frames
    pub fn depth(&self) -> usize {
        self.frames.len()
    }

    /// Format the stack trace for display
    pub fn display<F>(
        &self,
        file_table: &'a crate::FileTable<'a>,
        mut writer: F,
    ) -> Result<(), core::fmt::Error>
    where
        F: FnMut(&str) -> Result<(), core::fmt::Error>,
    {
        for frame in self.frames.iter() {
            // Frame number
            writer("#")?;
            let mut buf = [0u8; 5];
            writer(format_u16(frame.depth, &mut buf))?;
            writer(" ")?;

            // Function name or address
            if let Some(func) = frame.function {
                if let Some(ref name) = func.name {
                    writer(name.as_str())?;
                } else {
                    writer("<unknown>")?;
                }
            } else {
                writer("0x")?;
                let mut hex_buf = [0u8; 8];
                writer(format_hex_u32(frame.pc, &mut hex_buf))?;
            }

            // Source location
            if let Some(line_info) = frame.line_info {
                writer(" at ")?;
                line_info.format_location(file_table).display(&mut writer)?;
            }

            writer("\n")?;
        }
        Ok(())
    }
}

/// Helper to build a stack trace from runtime information
pub struct StackTraceBuilder<'a> {
    debug_info: &'a crate::DwarfDebugInfo<'a>,
}

impl<'a> StackTraceBuilder<'a> {
    /// Create a new stack trace builder
    pub fn new(debug_info: &'a crate::DwarfDebugInfo<'a>) -> Self {
        Self { debug_info }
    }

    /// Build a stack trace starting from the given PC
    /// Note: This is a simplified version that only shows the current location
    /// Full stack walking requires runtime support for frame pointers
    #[cfg(all(feature = "line-info", feature = "function-info"))]
    pub fn build_from_pc(&mut self, pc: u32) -> Result<StackTrace<'a>, ()> {
        let mut trace = StackTrace::new();

        // Get function info
        let function = self.debug_info.find_function_info(pc);

        // Get line info
        let line_info = self.debug_info.find_line_info(pc).ok().flatten();

        // Add current frame
        let frame = StackFrame { pc, function, line_info, depth: 0 };

        trace.push_frame(frame)?;

        // TODO: Add support for walking up the call stack
        // This requires:
        // 1. Runtime frame pointer tracking
        // 2. Call frame information parsing (.debug_frame)
        // 3. Stack unwinding logic

        Ok(trace)
    }

    /// Build a partial trace with just PC values
    /// (when debug info is not available)
    pub fn build_minimal(&self, pcs: &[u32]) -> Result<StackTrace<'a>, ()> {
        let mut trace = StackTrace::new();

        for (i, &pc) in pcs.iter().enumerate() {
            let frame = StackFrame { pc, function: None, line_info: None, depth: i as u16 };
            trace.push_frame(frame)?;
        }

        Ok(trace)
    }
}

// Helper to format u16 without allocation
fn format_u16(mut n: u16, buf: &mut [u8]) -> &str {
    if n == 0 {
        return "0";
    }

    let mut i = buf.len();
    while n > 0 && i > 0 {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
    }

    core::str::from_utf8(&buf[i..]).unwrap_or("?")
}

// Helper to format u32 as hexadecimal
fn format_hex_u32(mut n: u32, buf: &mut [u8]) -> &str {
    let mut i = buf.len();

    if n == 0 {
        return "00000000";
    }

    while n > 0 && i > 0 {
        i -= 1;
        let digit = (n & 0xF) as u8;
        buf[i] = if digit < 10 { b'0' + digit } else { b'a' + digit - 10 };
        n >>= 4;
    }

    // Pad with zeros
    while i > 0 {
        i -= 1;
        buf[i] = b'0';
    }

    core::str::from_utf8(buf).unwrap_or("????????")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stack_trace_display() {
        let mut trace = StackTrace::new();

        // Add a frame with no debug info
        let frame1 = StackFrame { pc: 0x1000, function: None, line_info: None, depth: 0 };

        trace.push_frame(frame1).unwrap();

        // Test minimal display
        let mut output = String::new();
        let file_table = crate::FileTable::new();
        trace
            .display(&file_table, |s| {
                output.push_str(s);
                Ok(())
            })
            .unwrap();

        assert!(output.contains("#0 0x00001000"));
    }

    #[test]
    fn test_hex_formatting() {
        let mut buf = [0u8; 8];
        assert_eq!(format_hex_u32(0x1234ABCD, &mut buf), "1234abcd");
        assert_eq!(format_hex_u32(0, &mut buf), "00000000");
        assert_eq!(format_hex_u32(0xFF, &mut buf), "000000ff");
    }
}
