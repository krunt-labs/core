//! Pure compiler orchestration from a structural graph document to canonical IR.
//!
//! This crate owns stage coordination and diagnostics. Semantic meaning stays
//! in plugins, while IR invariants stay in the IR crate.

use krunt_diagnostics::{
	CompilerStage, Diagnostic, DiagnosticCode, Diagnostics, GraphElementId, Severity,
};
use krunt_graph_model::{EdgeKind, GraphDocument, Node, PortDirection, PortId, PortKind};
use krunt_ir::{
	BuildError, ControlEdge, DataEdge, GraphNodeId, IRBuilder, IRGraph, IRNode, IROp, PortRef,
	SourceMap,
};
use krunt_plugin_api::{LoweredOperation, PluginRegistry};
use std::collections::BTreeMap;

/// Result of compiling a graph document.
#[derive(Debug)]
pub struct CompilationResult {
	ir: Option<IRGraph>,
	diagnostics: Diagnostics,
}

impl CompilationResult {
	pub fn ir(&self) -> Option<&IRGraph> {
		self.ir.as_ref()
	}

	pub fn diagnostics(&self) -> &Diagnostics {
		&self.diagnostics
	}

	pub fn has_errors(&self) -> bool {
		self.diagnostics.has_errors()
	}

	pub fn into_parts(self) -> (Option<IRGraph>, Diagnostics) {
		(self.ir, self.diagnostics)
	}
}

/// Compile a validated structural graph through semantic plugins into IR.
pub fn compile(document: &GraphDocument, plugins: &PluginRegistry) -> CompilationResult {
	let mut diagnostics = Diagnostics::new();
	let mut builder = IRBuilder::new();
	let mut port_bindings = BTreeMap::new();

	for (index, node) in document.nodes().iter().enumerate() {
		let ir_id = krunt_ir::IrNodeId((index + 1) as u32);
		let Some(plugin) = plugins.resolve(node.type_id()) else {
			diagnostics.push(missing_plugin_diagnostic(node));
			continue;
		};

		match plugin.lower_node(node) {
			Ok(lowered) => {
				register_port_bindings(node, ir_id, &mut port_bindings, &mut diagnostics);
				let ir_node = ir_node(node, ir_id, lowered);
				if let Err(error) = builder.add_node(ir_node) {
					diagnostics.push(ir_build_diagnostic(
						GraphElementId::node(
							krunt_graph_model::NodeId::new(node.id().as_u64()).unwrap(),
						),
						error,
					));
				}
			}
			Err(plugin_diagnostics) => diagnostics.extend(plugin_diagnostics),
		}
	}

	if !diagnostics.has_errors() {
		for edge in document.edges() {
			match edge.kind() {
				EdgeKind::EXECUTION => add_control_edge(
					edge.from(),
					edge.to(),
					&port_bindings,
					&mut builder,
					&mut diagnostics,
				),
				EdgeKind::DATA => add_data_edge(
					edge.from(),
					edge.to(),
					&port_bindings,
					&mut builder,
					&mut diagnostics,
				),
			}
		}
	}

	let ir = if diagnostics.has_errors() {
		None
	} else {
		match builder.build() {
			Ok(ir) => Some(ir),
			Err(error) => {
				diagnostics.push(ir_build_diagnostic(
					GraphElementId::GRAPH(
						krunt_graph_model::GraphId::new(document.metadata().graph_id().as_u64())
							.unwrap(),
					),
					error,
				));
				None
			}
		}
	};

	diagnostics.sort_canonical();
	CompilationResult { ir, diagnostics }
}

#[derive(Clone, Copy)]
struct PortBinding {
	source_port: PortId,
	ir_node: krunt_ir::IrNodeId,
	kind: PortKind,
	index: u16,
}

fn register_port_bindings(
	node: &Node,
	ir_node: krunt_ir::IrNodeId,
	port_bindings: &mut BTreeMap<PortId, PortBinding>,
	diagnostics: &mut Diagnostics,
) {
	let mut input_index = 0u16;
	let mut output_index = 0u16;
	for port in node.ports() {
		let index = match (port.direction(), port.kind()) {
			(PortDirection::INPUT, PortKind::DATA) => {
				let index = input_index;
				input_index = input_index.saturating_add(1);
				index
			}
			(PortDirection::OUTPUT, PortKind::DATA) => {
				let index = output_index;
				output_index = output_index.saturating_add(1);
				index
			}
			_ => 0,
		};
		port_bindings.insert(
			port.id(),
			PortBinding {
				source_port: port.id(),
				ir_node,
				kind: port.kind(),
				index,
			},
		);
	}
	if input_index == u16::MAX || output_index == u16::MAX {
		diagnostics.push(
			Diagnostic::new(
				DiagnosticCode::new("LOWERING.TOO_MANY_DATA_PORTS").unwrap(),
				Severity::ERROR,
				CompilerStage::LOWERING,
				"A node has too many data ports for the canonical IR.",
				GraphElementId::NODE(krunt_graph_model::NodeId::new(node.id().as_u64()).unwrap()),
			)
			.unwrap(),
		);
	}
}

fn add_control_edge(
	from: PortId,
	to: PortId,
	port_bindings: &BTreeMap<PortId, PortBinding>,
	builder: &mut IRBuilder,
	diagnostics: &mut Diagnostics,
) {
	let Some(from) = port_bindings.get(&from).copied() else {
		diagnostics.push(missing_port_diagnostic(from));
		return;
	};
	let Some(to) = port_bindings.get(&to).copied() else {
		diagnostics.push(missing_port_diagnostic(to));
		return;
	};
	if from.kind != PortKind::EXECUTION || to.kind != PortKind::EXECUTION {
		diagnostics.push(
			Diagnostic::new(
				DiagnosticCode::new("LOWERING.EDGE_KIND_MISMATCH").unwrap(),
				Severity::ERROR,
				CompilerStage::LOWERING,
				"An execution edge did not resolve to execution ports.",
				GraphElementId::PORT(to.source_port),
			)
			.unwrap(),
		);
		return;
	}
	if let Err(error) = builder.add_control_edge(ControlEdge {
		from: from.ir_node,
		to: to.ir_node,
	}) {
		diagnostics.push(ir_build_diagnostic(
			GraphElementId::PORT(from.source_port),
			error,
		));
	}
}

fn add_data_edge(
	from: PortId,
	to: PortId,
	port_bindings: &BTreeMap<PortId, PortBinding>,
	builder: &mut IRBuilder,
	diagnostics: &mut Diagnostics,
) {
	let Some(from_binding) = port_bindings.get(&from).copied() else {
		diagnostics.push(missing_port_diagnostic(from));
		return;
	};
	let Some(to_binding) = port_bindings.get(&to).copied() else {
		diagnostics.push(missing_port_diagnostic(to));
		return;
	};
	if from_binding.kind != PortKind::DATA || to_binding.kind != PortKind::DATA {
		diagnostics.push(
			Diagnostic::new(
				DiagnosticCode::new("LOWERING.EDGE_KIND_MISMATCH").unwrap(),
				Severity::ERROR,
				CompilerStage::LOWERING,
				"A data edge did not resolve to data ports.",
				GraphElementId::PORT(to),
			)
			.unwrap(),
		);
		return;
	}
	if let Err(error) = builder.add_data_edge(DataEdge {
		from: PortRef {
			node: from_binding.ir_node,
			index: from_binding.index,
		},
		to: PortRef {
			node: to_binding.ir_node,
			index: to_binding.index,
		},
	}) {
		diagnostics.push(ir_build_diagnostic(GraphElementId::PORT(from), error));
	}
}

fn ir_node(node: &Node, id: krunt_ir::IrNodeId, lowered: LoweredOperation) -> IRNode {
	IRNode {
		id,
		op: IROp::OPERATION {
			name: lowered.operation().clone(),
		},
		input_types: lowered.input_types().to_vec(),
		output_types: lowered.output_types().to_vec(),
		async_kind: lowered.async_kind(),
		effects: lowered.effects().to_vec(),
		source: SourceMap::node(GraphNodeId(node.id().as_u64())),
	}
}

fn missing_plugin_diagnostic(node: &Node) -> Diagnostic {
	Diagnostic::new(
		DiagnosticCode::new("PLUGIN.MISSING_NODE_TYPE").unwrap(),
		Severity::ERROR,
		CompilerStage::PLUGIN_RESOLUTION,
		format!(
			"No semantic plugin is registered for node type '{}'.",
			node.type_id().as_str()
		),
		GraphElementId::NODE(krunt_graph_model::NodeId::new(node.id().as_u64()).unwrap()),
	)
	.unwrap()
}

fn missing_port_diagnostic(port: PortId) -> Diagnostic {
	Diagnostic::new(
		DiagnosticCode::new("LOWERING.MISSING_PORT").unwrap(),
		Severity::ERROR,
		CompilerStage::LOWERING,
		"An edge references a port that was not lowered.",
		GraphElementId::PORT(port),
	)
	.unwrap()
}

fn ir_build_diagnostic(primary: GraphElementId, error: BuildError) -> Diagnostic {
	Diagnostic::new(
		DiagnosticCode::new("LOWERING.INVALID_IR").unwrap(),
		Severity::ERROR,
		CompilerStage::LOWERING,
		format!("The lowered operation violated an IR invariant: {error:?}."),
		primary,
	)
	.unwrap()
}

#[cfg(test)]
mod tests {
	use super::*;
	use krunt_graph_model::{
		GraphId, GraphMetadata, NodeId, NodeTypeId, PluginReference, Port, TypeReference,
	};
	use krunt_ir::AsyncKind;
	use krunt_plugin_api::{PluginManifest, SemanticPlugin};

	struct BackendTestPlugin {
		manifest: PluginManifest,
	}

	impl SemanticPlugin for BackendTestPlugin {
		fn manifest(&self) -> &PluginManifest {
			&self.manifest
		}

		fn lower_node(&self, node: &Node) -> Result<LoweredOperation, Vec<Diagnostic>> {
			let (inputs, outputs) = if node.type_id().as_str() == "test.route" {
				(vec![], vec![krunt_ir::TypeId::new("Request").unwrap()])
			} else {
				(vec![krunt_ir::TypeId::new("Request").unwrap()], vec![])
			};
			Ok(LoweredOperation::new(
				krunt_ir::OperationId::new(node.type_id().as_str()).unwrap(),
				inputs,
				outputs,
				AsyncKind::SYNC,
				[],
			))
		}
	}

	fn document() -> GraphDocument {
		let metadata =
			GraphMetadata::new(GraphId::new(1).unwrap(), "backend-v1", "krunt-dev").unwrap();
		let mut route = Node::new(
			NodeId::new(1).unwrap(),
			NodeTypeId::new("test.route").unwrap(),
		);
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
				.with_type_reference(TypeReference::new("Request").unwrap()),
			)
			.unwrap();
		let mut response = Node::new(
			NodeId::new(2).unwrap(),
			NodeTypeId::new("test.response").unwrap(),
		);
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
				.with_type_reference(TypeReference::new("Request").unwrap()),
			)
			.unwrap();
		let mut builder = krunt_graph_model::GraphBuilder::new(metadata);
		builder.add_node(route).unwrap();
		builder.add_node(response).unwrap();
		builder
			.add_edge(krunt_graph_model::Edge::new(
				krunt_graph_model::EdgeId::new(31).unwrap(),
				PortId::new(11).unwrap(),
				PortId::new(21).unwrap(),
				EdgeKind::EXECUTION,
			))
			.unwrap();
		builder
			.add_edge(krunt_graph_model::Edge::new(
				krunt_graph_model::EdgeId::new(32).unwrap(),
				PortId::new(12).unwrap(),
				PortId::new(22).unwrap(),
				EdgeKind::DATA,
			))
			.unwrap();
		builder.build().unwrap()
	}

	fn registry() -> PluginRegistry {
		let manifest = PluginManifest::new(
			PluginReference::new("test-backend", "0.1.0").unwrap(),
			[
				NodeTypeId::new("test.route").unwrap(),
				NodeTypeId::new("test.response").unwrap(),
			],
		)
		.unwrap();
		let mut registry = PluginRegistry::new();
		registry
			.register(Box::new(BackendTestPlugin { manifest }))
			.unwrap();
		registry
	}

	#[test]
	fn compiles_plugin_lowered_nodes_and_both_edge_kinds() {
		let result = compile(&document(), &registry());

		assert!(!result.has_errors());
		assert_eq!(result.diagnostics().as_slice().len(), 0);
		assert_eq!(result.ir().unwrap().nodes().len(), 2);
		assert_eq!(result.ir().unwrap().control_edges().len(), 1);
		assert_eq!(result.ir().unwrap().data_edges().len(), 1);
	}

	#[test]
	fn missing_plugin_blocks_ir_generation_with_a_node_diagnostic() {
		let result = compile(&document(), &PluginRegistry::new());

		assert!(result.has_errors());
		assert!(result.ir().is_none());
		assert_eq!(
			result.diagnostics().as_slice()[0].code().as_str(),
			"PLUGIN.MISSING_NODE_TYPE"
		);
	}

	#[test]
	fn repeated_compilation_is_deterministic() {
		let first = compile(&document(), &registry());
		let second = compile(&document(), &registry());

		assert_eq!(first.diagnostics(), second.diagnostics());
		assert_eq!(first.ir(), second.ir());
	}
}
