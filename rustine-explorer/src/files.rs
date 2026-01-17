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

/// Result of a file operation. Returned on success so the UI can show detailed
/// feedback about what happened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileOpResult {
	/// A new file or directory was created. Contains the created path.
	Created(PathBuf),
	/// A file or directory was deleted. Contains the deleted path.
	Deleted(PathBuf),
	/// A path was renamed from -> to.
	Renamed { from: PathBuf, to: PathBuf },
	/// A path was copied from -> to.
	Copied { from: PathBuf, to: PathBuf },
	/// A path was moved from -> to (may be implemented via rename or copy+delete).
	Moved { from: PathBuf, to: PathBuf },
}

/// Returns the contents of `directory` with their names and types.
pub fn get_contents(directory: impl AsRef<Path>, show_hidden: bool) -> io::Result<Vec<EntryInfo>> {
	let mut entries = Vec::new();
	let dir_path = directory.as_ref();

	for entry in fs::read_dir(dir_path)? {
		let entry = entry?;
		let path = entry.path();
		let name = entry
			.file_name()
			.to_string_lossy()
			.into_owned();

		if !show_hidden && name.starts_with('.') {
			continue;
		}

		let entry_type = classify_entry(&entry, &path);

		entries.push(EntryInfo {
			name,
			path,
			entry_type,
		});
	}

	entries.sort_by(|a, b| {
		let type_order = |entry_type: &EntryType| match entry_type {
			EntryType::Directory => 0,
			EntryType::File => 1,
			EntryType::Symlink => 2,
			EntryType::Other => 3,
		};

		let a_order = type_order(&a.entry_type);
		let b_order = type_order(&b.entry_type);

		a_order.cmp(&b_order).then_with(|| a.name.cmp(&b.name))
	});

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


/// Delete a file or directory at `path`.
/// Directories are removed recursively.
pub fn delete_path(path: impl AsRef<Path>) -> io::Result<FileOpResult> {
	let path = path.as_ref();
	let meta = fs::symlink_metadata(path)?;

	if meta.file_type().is_dir() && !meta.file_type().is_symlink() {
		fs::remove_dir_all(path)?;
	} else {
		// files and symlinks
		fs::remove_file(path)?;
	}

	Ok(FileOpResult::Deleted(path.to_path_buf()))
}

/// Rename/move `src` to `dst` path. If a simple `fs::rename` fails (for
/// example across filesystems), falls back to copy+delete.
pub fn rename_path(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<FileOpResult> {
	let src = src.as_ref().to_path_buf();
	let dst = dst.as_ref().to_path_buf();

	if let Err(_e) = fs::rename(&src, &dst) {
		// fallback: copy then delete
		copy_recursively(&src, &dst)?;
		delete_path(&src)?;
		return Ok(FileOpResult::Renamed { from: src, to: dst });
	}

	Ok(FileOpResult::Renamed { from: src, to: dst })
}

fn copy_recursively(src: &Path, dst: &Path) -> io::Result<()> {
	let meta = fs::symlink_metadata(src)?;

	if dst.exists() {
		return Err(io::Error::new(
			io::ErrorKind::AlreadyExists,
			format!("destination already exists: {}", dst.display()),
		));
	}

	if meta.file_type().is_symlink() {
		// Prefer to replicate the symlink on Unix. Otherwise copy target contents.
		#[cfg(unix)]
		{
			use std::os::unix::fs as unix_fs;
			let target = fs::read_link(src)?;
			unix_fs::symlink(target, dst)?;
			return Ok(());
		}
		#[cfg(not(unix))]
		{
			// fallthrough to copying file contents
		}
	}

	if meta.is_dir() {
		fs::create_dir_all(dst)?;
		for entry in fs::read_dir(src)? {
			let entry = entry?;
			let file_name = entry.file_name();
			let child_src = entry.path();
			let child_dst = dst.join(file_name);
			copy_recursively(&child_src, &child_dst)?;
		}
	} else {
		// regular file: copy metadata+contents
		if let Some(parent) = dst.parent() {
			fs::create_dir_all(parent)?;
		}
		fs::copy(src, dst)?;
	}

	Ok(())
}

/// Copy `src` (file or directory) into `dest_dir`. The entry's name is preserved.
/// Returns the destination path on success.
pub fn copy_to_dir(src: impl AsRef<Path>, dest_dir: impl AsRef<Path>) -> io::Result<FileOpResult> {
	let src = src.as_ref().to_path_buf();
	let dest_dir = dest_dir.as_ref();

	let name = src
		.file_name()
		.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "source has no file name"))?;

	let dst = dest_dir.join(name);
	copy_recursively(&src, &dst)?;
	Ok(FileOpResult::Copied { from: src, to: dst })
}

/// Move `src` (file or directory) into `dest_dir`. This will attempt a filesystem
/// rename first and fall back to copy+delete if necessary. Returns the destination path.
pub fn move_to_dir(src: impl AsRef<Path>, dest_dir: impl AsRef<Path>) -> io::Result<FileOpResult> {
	let src = src.as_ref().to_path_buf();
	let dest_dir = dest_dir.as_ref();

	let name = src
		.file_name()
		.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "source has no file name"))?;

	let dst = dest_dir.join(name);
	if let Err(_e) = fs::rename(&src, &dst) {
		// fallback
		copy_recursively(&src, &dst)?;
		delete_path(&src)?;
		return Ok(FileOpResult::Moved { from: src, to: dst });
	}

	Ok(FileOpResult::Moved { from: src, to: dst })
}

/// Create a new file or directory inside `dest_dir` with the given `name`.
/// If `name` ends with a path separator (`/` or `\`), a directory is created.
/// Otherwise an empty file is created. Returns the full path to the created entry.
pub fn create_new(dest_dir: impl AsRef<Path>, name: &str) -> io::Result<FileOpResult> {
	let dest_dir = dest_dir.as_ref();

	let is_dir = name.ends_with('/') || name.ends_with('\\');
	let clean_name = if is_dir {
		name.trim_end_matches(|c| c == '/' || c == '\\')
	} else {
		name
	};

	if clean_name.is_empty() {
		return Err(io::Error::new(io::ErrorKind::InvalidInput, "name is empty"));
	}

	let dst = dest_dir.join(clean_name);
	if dst.exists() {
		return Err(io::Error::new(
			io::ErrorKind::AlreadyExists,
			format!("destination already exists: {}", dst.display()),
		));
	}

	if is_dir {
		fs::create_dir_all(&dst)?;
	} else {
		if let Some(parent) = dst.parent() {
			fs::create_dir_all(parent)?;
		}
		fs::File::create(&dst)?;
	}

	Ok(FileOpResult::Created(dst))
}


