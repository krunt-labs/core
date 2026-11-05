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
}

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
}
