use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Kind of a filesystem entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryType {
	/// Regular file
	File,
	/// Directory
	Directory,
	/// Symbolic link
	Symlink,
	/// Anything else (sockets, pipes, devices, etc.)
	Other,
}

/// Information about a directory entry.
#[derive(Debug, Clone)]
pub struct EntryInfo {
	/// File name (not a full path)
	pub name: String,
	/// Full path to the entry
	pub path: PathBuf,
	/// Entry type
	pub entry_type: EntryType,
}

/// Returns the contents of `directory` with their names and types.
pub fn get_contents(directory: impl AsRef<Path>) -> io::Result<Vec<EntryInfo>> {
	let mut entries = Vec::new();
	let dir_path = directory.as_ref();

	for entry in fs::read_dir(dir_path)? {
		let entry = entry?;
		let path = entry.path();
		let name = entry
			.file_name()
			.to_string_lossy()
			.into_owned();

		let entry_type = classify_entry(&entry, &path);

		entries.push(EntryInfo {
			name,
			path,
			entry_type,
		});
	}

	entries.sort_by(|a, b| a.name.cmp(&b.name));

	Ok(entries)
}

fn classify_entry(entry: &fs::DirEntry, path: &Path) -> EntryType {
	match entry.file_type() {
		Ok(file_type) if file_type.is_dir() => EntryType::Directory,
		Ok(file_type) if file_type.is_file() => EntryType::File,
		Ok(file_type) if file_type.is_symlink() => EntryType::Symlink,
		Ok(_) => EntryType::Other,
		Err(_) => match fs::metadata(path) {
			Ok(metadata) => {
				if metadata.is_dir() {
					EntryType::Directory
				} else if metadata.is_file() {
					EntryType::File
				} else {
					EntryType::Other
				}
			}
			Err(_) => EntryType::Other,
		},
	}
}


