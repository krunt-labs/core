//! Public graph-to-project orchestration for Krunt.
//!
//! This crate is an adapter above the target-neutral compiler. It registers the
//! built-in semantic plugins, selects a language/framework target, and returns
//! a deterministic generated project. The compiler core remains independent of
//! Backend DSL semantics and framework codegen.

use krunt_backend_dsl::BackendDslPlugin;
use krunt_codegen_nestjs::{NestJsBackend, NestJsGenerationError};
use krunt_codegen_typescript::{TypeScriptBackend, TypeScriptLoweringError};
use krunt_diagnostics::Diagnostics;
use krunt_framework_backend::{ArchiveError, GeneratedProject};
use krunt_graph_model::GraphDocument;
use krunt_plugin_api::PluginRegistry;
use std::error::Error;
use std::fmt;

/// The language target selected for compilation.
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum LanguageTarget {
	TYPESCRIPT,
}

/// The framework target selected for compilation.
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum FrameworkTarget {
	NESTJS,
}

/// A language/framework target pair.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CompilationTarget {
	language: LanguageTarget,
	framework: FrameworkTarget,
}

impl CompilationTarget {
	pub const TYPESCRIPT_NESTJS: Self = Self {
		language: LanguageTarget::TYPESCRIPT,
		framework: FrameworkTarget::NESTJS,
	};

	pub const fn new(language: LanguageTarget, framework: FrameworkTarget) -> Self {
		Self {
			language,
			framework,
		}
	}

	pub const fn language(self) -> LanguageTarget {
		self.language
	}

	pub const fn framework(self) -> FrameworkTarget {
		self.framework
	}
}

/// The generated artifact returned after successful compilation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompilationArtifact {
	target: CompilationTarget,
	project: GeneratedProject,
}

impl CompilationArtifact {
	pub fn target(&self) -> CompilationTarget {
		self.target
	}

	pub fn project(&self) -> &GeneratedProject {
		&self.project
	}

	pub fn into_project(self) -> GeneratedProject {
		self.project
	}

	pub fn zip_archive(&self) -> Result<Vec<u8>, ArchiveError> {
		self.project.zip_archive()
	}
}

/// Errors raised by the public compilation pipeline.
#[allow(non_camel_case_types)]
#[derive(Debug)]
pub enum CompilationError {
	DIAGNOSTICS(Diagnostics),
	UNSUPPORTED_TARGET(CompilationTarget),
	TYPESCRIPT_LOWERING(TypeScriptLoweringError),
	NESTJS_GENERATION(NestJsGenerationError),
}

impl CompilationError {
	pub fn diagnostics(&self) -> Option<&Diagnostics> {
		match self {
			Self::DIAGNOSTICS(diagnostics) => Some(diagnostics),
			_ => None,
		}
	}
}

impl fmt::Display for CompilationError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::DIAGNOSTICS(diagnostics) => {
				write!(
					formatter,
					"compilation failed with {} diagnostic(s)",
					diagnostics.as_slice().len()
				)
			}
			Self::UNSUPPORTED_TARGET(target) => write!(
				formatter,
				"unsupported compilation target: {:?}/{:?}",
				target.language, target.framework
			),
			Self::TYPESCRIPT_LOWERING(error) => {
				write!(formatter, "TypeScript lowering failed: {error}")
			}
			Self::NESTJS_GENERATION(error) => {
				write!(formatter, "NestJS generation failed: {error}")
			}
		}
	}
}

impl Error for CompilationError {}

/// Deterministic compiler-to-generator orchestration for user-facing adapters.
pub struct CompilerPipeline {
	plugins: PluginRegistry,
}

impl CompilerPipeline {
	/// Create a pipeline with the built-in Backend DSL registered.
	pub fn new() -> Self {
		let mut plugins = PluginRegistry::new();
		BackendDslPlugin::register(&mut plugins)
			.expect("the built-in Backend DSL registration must remain valid");
		Self { plugins }
	}

	/// Create a pipeline with an explicitly supplied semantic plugin registry.
	pub fn with_plugins(plugins: PluginRegistry) -> Self {
		Self { plugins }
	}

	pub fn compile(
		&self,
		document: &GraphDocument,
		target: CompilationTarget,
	) -> Result<CompilationArtifact, CompilationError> {
		if target != CompilationTarget::TYPESCRIPT_NESTJS {
			return Err(CompilationError::UNSUPPORTED_TARGET(target));
		}

		let compilation = krunt_compiler::compile(document, &self.plugins);
		let Some(ir) = compilation.ir() else {
			return Err(CompilationError::DIAGNOSTICS(compilation.into_parts().1));
		};

		let typescript =
			TypeScriptBackend::lower_graph(ir).map_err(CompilationError::TYPESCRIPT_LOWERING)?;
		let project = NestJsBackend::generate_project(&typescript)
			.map_err(CompilationError::NESTJS_GENERATION)?;
		Ok(CompilationArtifact { target, project })
	}
}

impl Default for CompilerPipeline {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use krunt_backend_dsl::{HTTP_REQUEST_TYPE, HTTP_ROUTE_NODE_TYPE, SUCCESS_RESPONSE_NODE_TYPE};
	use krunt_graph_model::{
		Edge, EdgeId, EdgeKind, GraphBuilder, GraphId, GraphMetadata, Node, NodeId, NodeTypeId,
		Port, PortDirection, PortId, PortKind, TypeReference,
	};
	use serde_json::Value;

	fn document(with_response: bool) -> GraphDocument {
		let metadata =
			GraphMetadata::new(GraphId::new(1).unwrap(), "backend-v1", "krunt-test").unwrap();
		let mut route = Node::new(
			NodeId::new(1).unwrap(),
			NodeTypeId::new(HTTP_ROUTE_NODE_TYPE).unwrap(),
		)
		.with_property("method", Value::String("GET".to_string()))
		.unwrap()
		.with_property("path", Value::String("/users".to_string()))
		.unwrap();
		route
			.add_port(Port::new(
				PortId::new(11).unwrap(),
				PortDirection::OUTPUT,
				PortKind::EXECUTION,
			))
			.unwrap();
		route
			.add_port(
				Port::new(
					PortId::new(12).unwrap(),
					PortDirection::OUTPUT,
					PortKind::DATA,
				)
				.with_type_reference(TypeReference::new(HTTP_REQUEST_TYPE).unwrap()),
			)
			.unwrap();

		let mut builder = GraphBuilder::new(metadata);
		builder.add_node(route).unwrap();
		if with_response {
			let mut response = Node::new(
				NodeId::new(2).unwrap(),
				NodeTypeId::new(SUCCESS_RESPONSE_NODE_TYPE).unwrap(),
			)
			.with_property("status_code", Value::Number(serde_json::Number::from(200)))
			.unwrap();
			response
				.add_port(Port::new(
					PortId::new(21).unwrap(),
					PortDirection::INPUT,
					PortKind::EXECUTION,
				))
				.unwrap();
			response
				.add_port(
					Port::new(
						PortId::new(22).unwrap(),
						PortDirection::INPUT,
						PortKind::DATA,
					)
					.with_type_reference(TypeReference::new("ANY").unwrap()),
				)
				.unwrap();
			builder.add_node(response).unwrap();
			builder
				.add_edge(Edge::new(
					EdgeId::new(31).unwrap(),
					PortId::new(11).unwrap(),
					PortId::new(21).unwrap(),
					EdgeKind::EXECUTION,
				))
				.unwrap();
			builder
				.add_edge(Edge::new(
					EdgeId::new(32).unwrap(),
					PortId::new(12).unwrap(),
					PortId::new(22).unwrap(),
					EdgeKind::DATA,
				))
				.unwrap();
		}
		builder.build().unwrap()
	}

	#[test]
	fn compiles_a_graph_into_a_deterministic_nestjs_artifact() {
		let pipeline = CompilerPipeline::new();
		let document = document(true);
		let first = pipeline
			.compile(&document, CompilationTarget::TYPESCRIPT_NESTJS)
			.unwrap();
		let second = pipeline
			.compile(&document, CompilationTarget::TYPESCRIPT_NESTJS)
			.unwrap();

		assert_eq!(first, second);
		assert_eq!(first.target(), CompilationTarget::TYPESCRIPT_NESTJS);
		assert!(first.project().file("src/generated.ts").is_some());
		assert!(first.project().file("src/app.controller.ts").is_some());
		assert_eq!(first.zip_archive().unwrap(), second.zip_archive().unwrap());
	}

	#[test]
	fn blocks_project_generation_when_compilation_has_errors() {
		let error = CompilerPipeline::new()
			.compile(&document(false), CompilationTarget::TYPESCRIPT_NESTJS)
			.expect_err("a route without a response must not generate a project");

		let diagnostics = error.diagnostics().expect("expected compiler diagnostics");
		assert!(diagnostics.has_errors());
		assert!(diagnostics
			.as_slice()
			.iter()
			.any(|diagnostic| diagnostic.code().as_str()
				== "BACKEND_DSL.HTTP_ROUTE_MISSING_RESPONSE"));
	}
}
