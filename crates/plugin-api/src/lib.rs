//! Semantic extension and lowering interfaces for Krunt.
//!
//! The compiler owns orchestration. Plugins own domain meaning and lower their
//! node types into target-neutral operations without knowing about the editor,
//! generated files, or a specific framework.

use krunt_diagnostics::Diagnostic;
use krunt_graph_model::{GraphDocument, Node, NodeTypeId, PluginReference, PortKind};
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

/// The target-neutral control shape returned by a semantic plugin.
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum LoweredOperationKind {
	OPERATION,
	BRANCH,
	MERGE,
	LOOP,
}

/// The target-neutral operation returned by a semantic plugin.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct LoweredOperation {
	operation: OperationId,
	kind: LoweredOperationKind,
	input_types: Vec<TypeId>,
	output_types: Vec<TypeId>,
	async_kind: AsyncKind,
	effects: Vec<EffectId>,
	attributes: BTreeMap<String, String>,
	returns: bool,
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
			kind: LoweredOperationKind::OPERATION,
			input_types: input_types.into_iter().collect(),
			output_types: output_types.into_iter().collect(),
			async_kind,
			effects,
			attributes: BTreeMap::new(),
			returns: false,
		}
	}

	/// Set the canonical control shape for this lowered operation.
	pub fn with_kind(mut self, kind: LoweredOperationKind) -> Self {
		self.kind = kind;
		self
	}

	/// Mark this operation as a target-neutral return/terminal operation.
	pub fn with_return(mut self, returns: bool) -> Self {
		self.returns = returns;
		self
	}

	/// Attach deterministic, target-neutral semantic metadata to the lowered
	/// operation. Namespaced keys keep domain attributes out of universal IR
	/// structure while allowing later language/framework backends to consume
	/// declared semantics.
	pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
		self.attributes.insert(key.into(), value.into());
		self
	}

	pub fn operation(&self) -> &OperationId {
		&self.operation
	}

	pub fn kind(&self) -> LoweredOperationKind {
		self.kind
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

	pub fn attributes(&self) -> &BTreeMap<String, String> {
		&self.attributes
	}

	pub fn returns(&self) -> bool {
		self.returns
	}
}

/// A capability or guarantee declared by a semantic contract.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct CapabilityId(String);

impl CapabilityId {
	pub fn new(value: impl Into<String>) -> Result<Self, ContractError> {
		let value = value.into();
		if value.trim().is_empty() {
			return Err(ContractError::EMPTY_CAPABILITY);
		}
		Ok(Self(value))
	}

	pub fn as_str(&self) -> &str {
		&self.0
	}
}

/// The expected shape of one input or output port.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PortContract {
	kind: PortKind,
	type_id: Option<TypeId>,
	required: bool,
}

impl PortContract {
	pub fn new(kind: PortKind, type_id: Option<TypeId>, required: bool) -> Self {
		Self {
			kind,
			type_id,
			required,
		}
	}

	pub fn kind(&self) -> PortKind {
		self.kind
	}

	pub fn type_id(&self) -> Option<&TypeId> {
		self.type_id.as_ref()
	}

	pub fn required(&self) -> bool {
		self.required
	}
}

/// Target-neutral semantic requirements and guarantees for one node type.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct NodeContract {
	inputs: Vec<PortContract>,
	outputs: Vec<PortContract>,
	requirements: Vec<CapabilityId>,
	capabilities: Vec<CapabilityId>,
	guarantees: Vec<CapabilityId>,
	async_kind: AsyncKind,
	effects: Vec<EffectId>,
	entry: bool,
	loop_node: bool,
}

impl NodeContract {
	pub fn new(
		inputs: impl IntoIterator<Item = PortContract>,
		outputs: impl IntoIterator<Item = PortContract>,
		requirements: impl IntoIterator<Item = CapabilityId>,
		capabilities: impl IntoIterator<Item = CapabilityId>,
		guarantees: impl IntoIterator<Item = CapabilityId>,
		async_kind: AsyncKind,
		effects: impl IntoIterator<Item = EffectId>,
	) -> Self {
		let mut requirements: Vec<_> = requirements.into_iter().collect();
		requirements.sort();
		requirements.dedup();
		let mut capabilities: Vec<_> = capabilities.into_iter().collect();
		capabilities.sort();
		capabilities.dedup();
		let mut guarantees: Vec<_> = guarantees.into_iter().collect();
		guarantees.sort();
		guarantees.dedup();
		let mut effects: Vec<_> = effects.into_iter().collect();
		effects.sort();
		effects.dedup();
		Self {
			inputs: inputs.into_iter().collect(),
			outputs: outputs.into_iter().collect(),
			requirements,
			capabilities,
			guarantees,
			async_kind,
			effects,
			entry: false,
			loop_node: false,
		}
	}

	pub fn with_entry(mut self, entry: bool) -> Self {
		self.entry = entry;
		self
	}

	pub fn inputs(&self) -> &[PortContract] {
		&self.inputs
	}

	pub fn outputs(&self) -> &[PortContract] {
		&self.outputs
	}

	pub fn requirements(&self) -> &[CapabilityId] {
		&self.requirements
	}

	pub fn capabilities(&self) -> &[CapabilityId] {
		&self.capabilities
	}

	pub fn guarantees(&self) -> &[CapabilityId] {
		&self.guarantees
	}

	pub fn async_kind(&self) -> AsyncKind {
		self.async_kind
	}

	pub fn effects(&self) -> &[EffectId] {
		&self.effects
	}

	pub fn is_entry(&self) -> bool {
		self.entry
	}

	pub fn with_loop(mut self, loop_node: bool) -> Self {
		self.loop_node = loop_node;
		self
	}

	pub fn is_loop(&self) -> bool {
		self.loop_node
	}
}

/// Domain plugin implementation supplied to the compiler.
pub trait SemanticPlugin: Send + Sync {
	fn manifest(&self) -> &PluginManifest;

	fn contract(&self, node: &Node) -> NodeContract;

	fn analyze_graph(&self, _document: &GraphDocument) -> Vec<Diagnostic> {
		Vec::new()
	}

	fn analyze_node(&self, _node: &Node) -> Vec<Diagnostic> {
		Vec::new()
	}

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

	pub fn analyze_graph(&self, document: &GraphDocument) -> Vec<Diagnostic> {
		self.plugins
			.values()
			.flat_map(|plugin| plugin.analyze_graph(document))
			.collect()
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

/// Errors raised while defining a target-neutral node contract.
#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContractError {
	EMPTY_CAPABILITY,
}

impl fmt::Display for ContractError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "{self:?}")
	}
}

impl std::error::Error for ContractError {}

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

		fn contract(&self, _node: &Node) -> NodeContract {
			NodeContract::new([], [], [], [], [], AsyncKind::SYNC, [])
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
	fn contracts_canonicalize_capabilities_and_effects() {
		let contract = NodeContract::new(
			[PortContract::new(PortKind::DATA, None, true)],
			[],
			[
				CapabilityId::new("storage.read").unwrap(),
				CapabilityId::new("async").unwrap(),
				CapabilityId::new("storage.read").unwrap(),
			],
			[
				CapabilityId::new("pure").unwrap(),
				CapabilityId::new("pure").unwrap(),
			],
			[],
			AsyncKind::SYNC,
			[
				EffectId::new("database.read").unwrap(),
				EffectId::new("database.read").unwrap(),
			],
		)
		.with_entry(true)
		.with_loop(true);

		assert_eq!(contract.inputs().len(), 1);
		assert_eq!(contract.requirements().len(), 2);
		assert_eq!(contract.capabilities().len(), 1);
		assert_eq!(contract.effects().len(), 1);
		assert!(contract.is_entry());
		assert!(contract.is_loop());
	}

	#[test]
	fn empty_capability_ids_are_rejected() {
		assert_eq!(CapabilityId::new(" "), Err(ContractError::EMPTY_CAPABILITY));
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
