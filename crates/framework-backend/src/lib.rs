//! Contracts for lowering a language IR into a framework-specific project.
//!
//! Framework backends own project layout, framework imports, runtime bootstrap,
//! and framework-specific adapters. They consume a language IR and return
//! deterministic user-owned files.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

/// A deterministic generated project represented by relative file paths.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedProject {
	files: BTreeMap<String, String>,
}

impl GeneratedProject {
	pub fn try_new(
		files: impl IntoIterator<Item = (String, String)>,
	) -> Result<Self, ProjectError> {
		let mut canonical = BTreeMap::new();
		for (path, content) in files {
			validate_path(&path)?;
			if canonical
				.insert(path.clone(), normalize_line_endings(content))
				.is_some()
			{
				return Err(ProjectError::DUPLICATE_PATH { path });
			}
		}
		Ok(Self { files: canonical })
	}

	pub fn files(&self) -> &BTreeMap<String, String> {
		&self.files
	}

	pub fn file(&self, path: &str) -> Option<&str> {
		self.files.get(path).map(String::as_str)
	}

	/// Export the project as a deterministic ZIP archive.
	///
	/// Entries use canonical path order, normalized contents, and fixed DOS
	/// timestamps. Files are stored without compression so archive bytes do not
	/// depend on a compressor implementation or version.
	pub fn zip_archive(&self) -> Result<Vec<u8>, ArchiveError> {
		zip_archive(&self.files)
	}
}

/// Errors raised while exporting a generated project archive.
#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArchiveError {
	TOO_MANY_FILES { count: usize },
	PATH_TOO_LONG { path: String },
	FILE_TOO_LARGE { path: String },
	ARCHIVE_TOO_LARGE,
}

impl fmt::Display for ArchiveError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::TOO_MANY_FILES { count } => {
				write!(f, "ZIP archives support at most 65535 files, got {count}")
			}
			Self::PATH_TOO_LONG { path } => {
				write!(
					f,
					"generated project path is too long for a ZIP archive: {path}"
				)
			}
			Self::FILE_TOO_LARGE { path } => {
				write!(
					f,
					"generated project file is too large for a ZIP archive: {path}"
				)
			}
			Self::ARCHIVE_TOO_LARGE => {
				write!(
					f,
					"generated project archive is too large for a ZIP archive"
				)
			}
		}
	}
}

impl Error for ArchiveError {}

/// Errors raised while constructing a generated project.
#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProjectError {
	EMPTY_PATH,
	ABSOLUTE_PATH { path: String },
	PATH_TRAVERSAL { path: String },
	BACKSLASH_PATH { path: String },
	DUPLICATE_PATH { path: String },
}

impl fmt::Display for ProjectError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::EMPTY_PATH => write!(f, "generated project paths cannot be empty"),
			Self::ABSOLUTE_PATH { path } => write!(f, "generated project path is absolute: {path}"),
			Self::PATH_TRAVERSAL { path } => {
				write!(f, "generated project path escapes its root: {path}")
			}
			Self::BACKSLASH_PATH { path } => {
				write!(f, "generated project paths must use '/': {path}")
			}
			Self::DUPLICATE_PATH { path } => {
				write!(f, "generated project contains duplicate path: {path}")
			}
		}
	}
}

impl Error for ProjectError {}

#[derive(Clone, Debug)]
struct ZipEntry {
	path: String,
	checksum: u32,
	size: u32,
	local_header_offset: u32,
}

fn zip_archive(files: &BTreeMap<String, String>) -> Result<Vec<u8>, ArchiveError> {
	if files.len() > usize::from(u16::MAX) {
		return Err(ArchiveError::TOO_MANY_FILES { count: files.len() });
	}

	let mut archive = Vec::new();
	let mut entries = Vec::with_capacity(files.len());
	for (path, content) in files {
		let path_bytes = path.as_bytes();
		let size = u32::try_from(content.len())
			.map_err(|_| ArchiveError::FILE_TOO_LARGE { path: path.clone() })?;
		let path_length = u16::try_from(path_bytes.len())
			.map_err(|_| ArchiveError::PATH_TOO_LONG { path: path.clone() })?;
		let local_header_offset = archive
			.len()
			.try_into()
			.map_err(|_| ArchiveError::ARCHIVE_TOO_LARGE)?;
		let checksum = crc32(content.as_bytes());

		write_u32(&mut archive, 0x0403_4b50);
		write_u16(&mut archive, 20);
		write_u16(&mut archive, 0x0800);
		write_u16(&mut archive, 0);
		write_u16(&mut archive, 0);
		write_u16(&mut archive, 0x0021);
		write_u32(&mut archive, checksum);
		write_u32(&mut archive, size);
		write_u32(&mut archive, size);
		write_u16(&mut archive, path_length);
		write_u16(&mut archive, 0);
		archive.extend_from_slice(path_bytes);
		archive.extend_from_slice(content.as_bytes());

		entries.push(ZipEntry {
			path: path.clone(),
			checksum,
			size,
			local_header_offset,
		});
	}

	let central_directory_start = archive.len();
	let central_directory_offset = archive
		.len()
		.try_into()
		.map_err(|_| ArchiveError::ARCHIVE_TOO_LARGE)?;
	for entry in &entries {
		let path_bytes = entry.path.as_bytes();
		let path_length =
			u16::try_from(path_bytes.len()).map_err(|_| ArchiveError::PATH_TOO_LONG {
				path: entry.path.clone(),
			})?;

		write_u32(&mut archive, 0x0201_4b50);
		write_u16(&mut archive, 20);
		write_u16(&mut archive, 20);
		write_u16(&mut archive, 0x0800);
		write_u16(&mut archive, 0);
		write_u16(&mut archive, 0);
		write_u16(&mut archive, 0x0021);
		write_u32(&mut archive, entry.checksum);
		write_u32(&mut archive, entry.size);
		write_u32(&mut archive, entry.size);
		write_u16(&mut archive, path_length);
		write_u16(&mut archive, 0);
		write_u16(&mut archive, 0);
		write_u16(&mut archive, 0);
		write_u16(&mut archive, 0);
		write_u32(&mut archive, 0);
		write_u32(&mut archive, entry.local_header_offset);
		archive.extend_from_slice(path_bytes);
	}

	let central_directory_size = archive
		.len()
		.checked_sub(central_directory_start)
		.and_then(|size| u32::try_from(size).ok())
		.ok_or(ArchiveError::ARCHIVE_TOO_LARGE)?;
	let entry_count = u16::try_from(entries.len()).expect("file count was checked above");
	write_u32(&mut archive, 0x0605_4b50);
	write_u16(&mut archive, 0);
	write_u16(&mut archive, 0);
	write_u16(&mut archive, entry_count);
	write_u16(&mut archive, entry_count);
	write_u32(&mut archive, central_directory_size);
	write_u32(&mut archive, central_directory_offset);
	write_u16(&mut archive, 0);

	Ok(archive)
}

fn write_u16(output: &mut Vec<u8>, value: u16) {
	output.extend_from_slice(&value.to_le_bytes());
}

fn write_u32(output: &mut Vec<u8>, value: u32) {
	output.extend_from_slice(&value.to_le_bytes());
}

fn crc32(bytes: &[u8]) -> u32 {
	let mut checksum = u32::MAX;
	for byte in bytes {
		checksum ^= u32::from(*byte);
		for _ in 0..8 {
			let mask = 0u32.wrapping_sub(checksum & 1);
			checksum = (checksum >> 1) ^ (0xedb8_8320 & mask);
		}
	}
	!checksum
}

/// A pluggable framework backend for one language IR.
pub trait FrameworkBackend {
	type Program;
	type Error: Error;

	/// Stable framework-backend identity, such as `nestjs`.
	fn backend_id(&self) -> &'static str;

	/// Generate a deterministic project from validated language IR.
	fn generate(&self, program: &Self::Program) -> Result<GeneratedProject, Self::Error>;
}

fn normalize_line_endings(content: String) -> String {
	content.replace("\r\n", "\n").replace('\r', "\n")
}

fn validate_path(path: &str) -> Result<(), ProjectError> {
	if path.is_empty() {
		return Err(ProjectError::EMPTY_PATH);
	}
	if path.starts_with('/') {
		return Err(ProjectError::ABSOLUTE_PATH {
			path: path.to_string(),
		});
	}
	if path.contains('\\') {
		return Err(ProjectError::BACKSLASH_PATH {
			path: path.to_string(),
		});
	}
	if path.split('/').any(|segment| segment == "..") {
		return Err(ProjectError::PATH_TRAVERSAL {
			path: path.to_string(),
		});
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn generated_files_are_sorted_and_line_endings_are_canonical() {
		let project = GeneratedProject::try_new([
			("z.txt".to_string(), "z\r\n".to_string()),
			("a.txt".to_string(), "a\r".to_string()),
		])
		.unwrap();

		assert_eq!(
			project.files().keys().collect::<Vec<_>>(),
			vec!["a.txt", "z.txt"]
		);
		assert_eq!(project.file("a.txt"), Some("a\n"));
		assert_eq!(project.file("z.txt"), Some("z\n"));
	}

	#[test]
	fn generated_paths_cannot_escape_the_project_root() {
		assert_eq!(
			GeneratedProject::try_new([("../escape".to_string(), String::new())]),
			Err(ProjectError::PATH_TRAVERSAL {
				path: "../escape".to_string()
			})
		);
	}

	#[test]
	fn generated_projects_export_deterministic_zip_archives() {
		let first = GeneratedProject::try_new([
			("src/z.ts".to_string(), "z\r\n".to_string()),
			("README.md".to_string(), "readme\r".to_string()),
		])
		.unwrap();
		let second = GeneratedProject::try_new([
			("README.md".to_string(), "readme\n".to_string()),
			("src/z.ts".to_string(), "z\n".to_string()),
		])
		.unwrap();

		let first_archive = first.zip_archive().unwrap();
		let second_archive = second.zip_archive().unwrap();
		assert_eq!(first_archive, second_archive);
		assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
		assert_eq!(
			first_archive
				.windows(4)
				.filter(|signature| *signature == [0x50, 0x4b, 0x03, 0x04])
				.count(),
			2
		);
		assert_eq!(
			first_archive
				.windows(4)
				.filter(|signature| *signature == [0x50, 0x4b, 0x01, 0x02])
				.count(),
			2
		);
		assert_eq!(
			&first_archive[first_archive.len() - 22..first_archive.len() - 18],
			&[0x50, 0x4b, 0x05, 0x06]
		);
	}
}
