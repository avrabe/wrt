use wrt_foundation::{
    bounded::{BoundedVec, MAX_DWARF_FILE_TABLE},
    NoStdProvider,
};

/// File table support for resolving file indices to paths
/// Provides the missing 2% for source file path resolution
use crate::strings::{DebugString, StringTable};

/// A file entry in the DWARF file table
#[derive(Debug, Clone)]
pub struct FileEntry<'a> {
    /// File path (may be relative or absolute)
    pub path: DebugString<'a>,
    /// Directory index (0 = current directory)
    pub dir_index: u32,
    /// Last modification time (0 = unknown)
    pub mod_time: u64,
    /// File size in bytes (0 = unknown)
    pub size: u64,
}

/// File table for resolving file indices to paths
#[derive(Debug)]
pub struct FileTable<'a> {
    /// Directory entries
    directories: BoundedVec<DebugString<'a>, MAX_DWARF_FILE_TABLE, NoStdProvider>,
    /// File entries
    files: BoundedVec<FileEntry<'a>, MAX_DWARF_FILE_TABLE, NoStdProvider>,
}

impl<'a> FileTable<'a> {
    /// Create a new empty file table
    pub fn new() -> Self {
        Self { directories: BoundedVec::new(NoStdProvider), files: BoundedVec::new(NoStdProvider) }
    }

    /// Add a directory entry
    pub fn add_directory(&mut self, dir: DebugString<'a>) -> Result<u32, ()> {
        let index = self.directories.len() as u32;
        self.directories.push(dir).map_err(|_| ())?;
        Ok(index)
    }

    /// Add a file entry
    pub fn add_file(&mut self, file: FileEntry<'a>) -> Result<u32, ()> {
        let index = self.files.len() as u32;
        self.files.push(file).map_err(|_| ())?;
        Ok(index)
    }

    /// Get a file entry by index (1-based as per DWARF spec)
    pub fn get_file(&self, index: u16) -> Option<&FileEntry<'a>> {
        if index == 0 {
            return None; // 0 means no file in DWARF
        }
        self.files.get((index - 1) as usize)
    }

    /// Get a directory by index (0 = compilation directory)
    pub fn get_directory(&self, index: u32) -> Option<&DebugString<'a>> {
        if index == 0 {
            return None; // 0 = compilation directory (not stored here)
        }
        self.directories.get((index - 1) as usize)
    }

    /// Get the full path for a file
    /// Returns directory + "/" + filename (or just filename if no directory)
    pub fn get_full_path(&self, file_index: u16) -> Option<FilePath<'a>> {
        let file = self.get_file(file_index)?;

        if file.dir_index == 0 {
            // File is relative to compilation directory
            Some(FilePath { directory: None, filename: file.path.clone() })
        } else {
            // File has explicit directory
            let directory = self.get_directory(file.dir_index)?;
            Some(FilePath { directory: Some(directory.clone()), filename: file.path.clone() })
        }
    }

    /// Get the number of files in the table
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Get the number of directories in the table
    pub fn directory_count(&self) -> usize {
        self.directories.len()
    }
}

/// Represents a resolved file path
#[derive(Debug, Clone)]
pub struct FilePath<'a> {
    /// Directory component (None = relative to compilation directory)
    pub directory: Option<DebugString<'a>>,
    /// Filename component
    pub filename: DebugString<'a>,
}

impl<'a> FilePath<'a> {
    /// Check if this is a relative path
    pub fn is_relative(&self) -> bool {
        self.directory.is_none() || !self.directory.as_ref().unwrap().as_str().starts_with('/')
    }

    /// Get the filename only (no directory)
    pub fn filename(&self) -> &str {
        self.filename.as_str()
    }

    /// Format as a path string (directory/filename)
    /// Note: In no_alloc environment, we can't allocate a new string,
    /// so this is primarily for display purposes
    pub fn display<F>(&self, mut writer: F) -> Result<(), core::fmt::Error>
    where
        F: FnMut(&str) -> Result<(), core::fmt::Error>,
    {
        if let Some(ref dir) = self.directory {
            writer(dir.as_str())?;
            writer("/")?;
        }
        writer(self.filename.as_str())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strings::StringTable;

    #[test]
    fn test_file_table() {
        // Create mock string data
        let string_data = b"\0src\0lib\0main.rs\0utils.rs\0tests\0";
        let string_table = StringTable::new(string_data);

        let mut file_table = FileTable::new();

        // Add directories
        let src_dir = string_table.get_string(1).unwrap();
        let lib_dir = string_table.get_string(5).unwrap();
        let tests_dir = string_table.get_string(25).unwrap();

        assert_eq!(file_table.add_directory(src_dir), Ok(1));
        assert_eq!(file_table.add_directory(lib_dir), Ok(2));
        assert_eq!(file_table.add_directory(tests_dir), Ok(3));

        // Add files
        let main_rs = FileEntry {
            path: string_table.get_string(9).unwrap(),
            dir_index: 1,
            mod_time: 0,
            size: 0,
        };

        let utils_rs = FileEntry {
            path: string_table.get_string(17).unwrap(),
            dir_index: 1,
            mod_time: 0,
            size: 0,
        };

        assert_eq!(file_table.add_file(main_rs), Ok(1));
        assert_eq!(file_table.add_file(utils_rs), Ok(2));

        // Test retrieval
        assert_eq!(file_table.file_count(), 2);
        assert_eq!(file_table.directory_count(), 3);

        // Test full path resolution
        let path1 = file_table.get_full_path(1).unwrap();
        assert_eq!(path1.filename(), "main.rs");
        assert_eq!(path1.directory.unwrap().as_str(), "src");

        let path2 = file_table.get_full_path(2).unwrap();
        assert_eq!(path2.filename(), "utils.rs");
        assert_eq!(path2.directory.unwrap().as_str(), "src");
    }

    #[test]
    fn test_file_path_display() {
        let string_data = b"\0src\0main.rs\0";
        let string_table = StringTable::new(string_data);

        let path = FilePath {
            directory: Some(string_table.get_string(1).unwrap()),
            filename: string_table.get_string(5).unwrap(),
        };

        let mut output = String::new();
        path.display(|s| {
            output.push_str(s);
            Ok(())
        })
        .unwrap();

        assert_eq!(output, "src/main.rs");
    }
}
