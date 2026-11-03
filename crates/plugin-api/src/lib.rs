//! Semantic extension and lowering interfaces for Krunt.
//!
//! The compiler owns orchestration. Plugins own domain meaning and lower their
//! node types into target-neutral operations without knowing about the editor,
//! generated files, or a specific framework.

use krunt_diagnostics::Diagnostic;
use krunt_graph_model::{Node, NodeTypeId, PluginReference};
use krunt_ir::{AsyncKind, EffectId, OperationId, TypeId};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt;

/// Serializable identity and node-type catalog for one semantic plugin.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PluginManifest {
	identity: PluginReference,
	supported_node_types: Vec<NodeTypeId>,
}

impl PluginManifest {
	pub fn new(
		identity: PluginReference,
		supported_node_types: impl IntoIterator<Item = NodeTypeId>,
	) -> Result<Self, PluginApiError> {
		let mut supported_node_types: Vec<_> = supported_node_types.into_iter().collect();
		supported_node_types.sort();
		if supported_node_types
			.windows(2)
			.any(|pair| pair[0] == pair[1])
		{
			return Err(PluginApiError::DUPLICATE_NODE_TYPE(
				supported_node_types[1].clone(),
			));
		}
		Ok(Self {
			identity,
			supported_node_types,
		})
	}

	pub fn identity(&self) -> &PluginReference {
		&self.identity
	}

	pub fn supported_node_types(&self) -> &[NodeTypeId] {
		&self.supported_node_types
	}
}

/// The target-neutral operation returned by a semantic plugin.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct LoweredOperation {
	operation: OperationId,
	input_types: Vec<TypeId>,
	output_types: Vec<TypeId>,
	async_kind: AsyncKind,
	effects: Vec<EffectId>,
}

impl LoweredOperation {
	pub fn new(
		operation: OperationId,
		input_types: impl IntoIterator<Item = TypeId>,
		output_types: impl IntoIterator<Item = TypeId>,
		async_kind: AsyncKind,
		effects: impl IntoIterator<Item = EffectId>,
	) -> Self {
		let mut effects: Vec<_> = effects.into_iter().collect();
		effects.sort();
		effects.dedup();
		Self {
			operation,
			input_types: input_types.into_iter().collect(),
			output_types: output_types.into_iter().collect(),
			async_kind,
			effects,
		}
	}

	pub fn operation(&self) -> &OperationId {
		&self.operation
	}

	pub fn input_types(&self) -> &[TypeId] {
		&self.input_types
	}

	pub fn output_types(&self) -> &[TypeId] {
		&self.output_types
	}

	pub fn async_kind(&self) -> AsyncKind {
		self.async_kind
	}

	pub fn effects(&self) -> &[EffectId] {
		&self.effects
	}
}

/// Domain plugin implementation supplied to the compiler.
pub trait SemanticPlugin: Send + Sync {
	fn manifest(&self) -> &PluginManifest;

	fn lower_node(&self, node: &Node) -> Result<LoweredOperation, Vec<Diagnostic>>;
}

/// Deterministic, explicitly-owned registry of semantic plugins.
#[derive(Default)]
pub struct PluginRegistry {
	plugins: BTreeMap<String, Box<dyn SemanticPlugin>>,
	node_types: BTreeMap<NodeTypeId, String>,
}

impl PluginRegistry {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn register(&mut self, plugin: Box<dyn SemanticPlugin>) -> Result<(), PluginApiError> {
		let manifest = plugin.manifest();
		let plugin_id = manifest.identity().id().to_owned();
		if self.plugins.contains_key(&plugin_id) {
			return Err(PluginApiError::DUPLICATE_PLUGIN(plugin_id));
		}
		for node_type in manifest.supported_node_types() {
			if let Some(existing_plugin) = self.node_types.get(node_type) {
				return Err(PluginApiError::NODE_TYPE_CONFLICT {
					node_type: node_type.clone(),
					existing_plugin: existing_plugin.clone(),
					new_plugin: plugin_id,
				});
			}
		}
		for node_type in manifest.supported_node_types() {
			self.node_types.insert(node_type.clone(), plugin_id.clone());
		}
		self.plugins.insert(plugin_id, plugin);
		Ok(())
	}

	pub fn resolve(&self, node_type: &NodeTypeId) -> Option<&dyn SemanticPlugin> {
		let plugin_id = self.node_types.get(node_type)?;
		self.plugins.get(plugin_id).map(Box::as_ref)
	}

	pub fn plugin_count(&self) -> usize {
		self.plugins.len()
	}

	pub fn node_type_count(&self) -> usize {
		self.node_types.len()
	}
}

/// Errors raised while defining or registering semantic plugins.
#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PluginApiError {
	DUPLICATE_PLUGIN(String),
	DUPLICATE_NODE_TYPE(NodeTypeId),
	NODE_TYPE_CONFLICT {
		node_type: NodeTypeId,
		existing_plugin: String,
		new_plugin: String,
	},
}

impl fmt::Display for PluginApiError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "{self:?}")
	}
}

impl std::error::Error for PluginApiError {}

#[cfg(test)]
mod tests {
	use super::*;
	use krunt_diagnostics::{CompilerStage, DiagnosticCode, GraphElementId, Severity};
	use krunt_graph_model::{GraphId, GraphMetadata, NodeId};

	struct TestPlugin {
		manifest: PluginManifest,
	}

	impl SemanticPlugin for TestPlugin {
		fn manifest(&self) -> &PluginManifest {
			&self.manifest
		}

		fn lower_node(&self, _node: &Node) -> Result<LoweredOperation, Vec<Diagnostic>> {
			Ok(LoweredOperation::new(
				OperationId::new("test.operation").unwrap(),
				[TypeId::new("Input").unwrap()],
				[TypeId::new("Output").unwrap()],
				AsyncKind::SYNC,
				[],
			))
		}
	}

	fn plugin(id: &str, node_types: &[&str]) -> TestPlugin {
		let identity = PluginReference::new(id, "0.1.0").unwrap();
		let node_types = node_types
			.iter()
			.map(|node_type| NodeTypeId::new(*node_type).unwrap());
		TestPlugin {
			manifest: PluginManifest::new(identity, node_types).unwrap(),
		}
	}

	#[test]
	fn manifest_canonicalizes_supported_node_types() {
		let manifest = PluginManifest::new(
			PluginReference::new("backend", "0.1.0").unwrap(),
			[
				NodeTypeId::new("backend.z").unwrap(),
				NodeTypeId::new("backend.a").unwrap(),
			],
		)
		.unwrap();

		assert_eq!(manifest.supported_node_types()[0].as_str(), "backend.a");
	}

	#[test]
	fn registry_resolves_node_types_without_global_mutable_state() {
		let mut registry = PluginRegistry::new();
		registry
			.register(Box::new(plugin("backend", &["backend.route"])))
			.unwrap();

		assert_eq!(registry.plugin_count(), 1);
		assert_eq!(registry.node_type_count(), 1);
		assert!(registry
			.resolve(&NodeTypeId::new("backend.route").unwrap())
			.is_some());
		assert!(registry
			.resolve(&NodeTypeId::new("unknown.node").unwrap())
			.is_none());
	}

	#[test]
	fn registry_rejects_duplicate_plugins_and_node_type_conflicts() {
		let mut registry = PluginRegistry::new();
		registry
			.register(Box::new(plugin("backend", &["backend.route"])))
			.unwrap();

		assert_eq!(
			registry.register(Box::new(plugin("backend", &["backend.other"]))),
			Err(PluginApiError::DUPLICATE_PLUGIN("backend".to_owned()))
		);
		assert!(matches!(
			registry.register(Box::new(plugin("other", &["backend.route"]))),
			Err(PluginApiError::NODE_TYPE_CONFLICT { .. })
		));
	}

	#[test]
	fn lowered_operations_are_target_neutral_and_effects_are_canonical() {
		let operation = LoweredOperation::new(
			OperationId::new("backend.query").unwrap(),
			[],
			[TypeId::new("User").unwrap()],
			AsyncKind::ASYNC,
			[
				EffectId::new("database.read").unwrap(),
				EffectId::new("database.read").unwrap(),
			],
		);

		assert_eq!(operation.effects().len(), 1);
		assert_eq!(operation.async_kind(), AsyncKind::ASYNC);
	}

	#[test]
	fn plugin_can_return_source_linked_diagnostics() {
		let diagnostic = Diagnostic::new(
			DiagnosticCode::new("PLUGIN.INVALID_NODE").unwrap(),
			Severity::ERROR,
			CompilerStage::SEMANTIC_ANALYSIS,
			"The plugin rejected this node.",
			GraphElementId::node(NodeId::new(1).unwrap()),
		)
		.unwrap();

		assert_eq!(
			diagnostic.primary(),
			GraphElementId::NODE(NodeId::new(1).unwrap())
		);
		let _metadata =
			GraphMetadata::new(GraphId::new(1).unwrap(), "backend-v1", "krunt-dev").unwrap();
	}
}
