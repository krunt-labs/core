//! Pure compiler orchestration from a structural graph document to canonical IR.
//!
//! This crate owns stage coordination and diagnostics. Semantic meaning stays
//! in plugins, while IR invariants stay in the IR crate.

use krunt_diagnostics::{
	CompilerStage, Diagnostic, DiagnosticCode, Diagnostics, GraphElementId, RelatedElement,
	Severity,
};
use krunt_graph_model::{EdgeKind, GraphDocument, Node, PortDirection, PortId, PortKind};
use krunt_ir::{
	eliminate_unreachable_nodes, BuildError, ControlEdge, DataEdge, GraphNodeId, IRBuilder,
	IRGraph, IRNode, IROp, PortRef, SourceMap,
};
use krunt_plugin_api::{CapabilityId, LoweredOperation, NodeContract, PluginRegistry};
use krunt_type_system::{check_assignable, Type};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

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
	let mut contracts = BTreeMap::new();

	for node in document.nodes() {
		let Some(plugin) = plugins.resolve(node.type_id()) else {
			diagnostics.push(missing_plugin_diagnostic(node));
			continue;
		};
		diagnostics.extend(plugin.analyze_node(node));
		let contract = plugin.contract(node);
		validate_node_contract(node, &contract, &mut diagnostics);
		contracts.insert(node.id(), contract);
	}

	if !diagnostics.has_errors() {
		analyze_data_types(document, &mut diagnostics);
	}
	if !diagnostics.has_errors() {
		resolve_capabilities(document, &contracts, &mut diagnostics);
	}
	if !diagnostics.has_errors() {
		validate_whole_graph(document, &contracts, &mut diagnostics);
	}
	if diagnostics.has_errors() {
		diagnostics.sort_canonical();
		return CompilationResult {
			ir: None,
			diagnostics,
		};
	}

	let mut builder = IRBuilder::new();
	let mut port_bindings = BTreeMap::new();

	for (index, node) in document.nodes().iter().enumerate() {
		let ir_id = krunt_ir::IrNodeId((index + 1) as u32);
		let Some(plugin) = plugins.resolve(node.type_id()) else {
			unreachable!("plugin resolution was completed during semantic analysis");
		};
		let contract = contracts
			.get(&node.id())
			.expect("contracts were collected during semantic analysis");

		match plugin.lower_node(node) {
			Ok(lowered) => {
				if !validate_lowered_operation(node, contract, &lowered, &mut diagnostics) {
					continue;
				}
				register_port_bindings(node, ir_id, &mut port_bindings, &mut diagnostics);
				let ir_node = ir_node(node, ir_id, lowered, contract);
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
			Ok(ir) => match eliminate_unreachable_nodes(ir) {
				Ok((ir, _report)) => Some(ir),
				Err(error) => {
					diagnostics.push(optimization_diagnostic(
						GraphElementId::GRAPH(
							krunt_graph_model::GraphId::new(
								document.metadata().graph_id().as_u64(),
							)
							.unwrap(),
						),
						error,
					));
					None
				}
			},
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

fn validate_node_contract(node: &Node, contract: &NodeContract, diagnostics: &mut Diagnostics) {
	let inputs: Vec<_> = node
		.ports()
		.iter()
		.filter(|port| port.direction() == PortDirection::INPUT)
		.collect();
	let outputs: Vec<_> = node
		.ports()
		.iter()
		.filter(|port| port.direction() == PortDirection::OUTPUT)
		.collect();
	validate_port_contracts(node, &inputs, contract.inputs(), "input", diagnostics);
	validate_port_contracts(node, &outputs, contract.outputs(), "output", diagnostics);
}

fn validate_port_contracts(
	node: &Node,
	actual: &[&krunt_graph_model::Port],
	expected: &[krunt_plugin_api::PortContract],
	label: &str,
	diagnostics: &mut Diagnostics,
) {
	let required_count = expected.iter().filter(|port| port.required()).count();
	if actual.len() < required_count || actual.len() > expected.len() {
		diagnostics.push(contract_diagnostic(
			"CONTRACT.PORT_COUNT",
			format!(
				"Node '{}' has {} {label} ports, but its contract requires {} to {}.",
				node.type_id().as_str(),
				actual.len(),
				required_count,
				expected.len(),
			),
			GraphElementId::NODE(krunt_graph_model::NodeId::new(node.id().as_u64()).unwrap()),
		));
	}

	for (actual_port, expected_port) in actual.iter().zip(expected) {
		if actual_port.kind() != expected_port.kind() {
			diagnostics.push(contract_diagnostic(
				"CONTRACT.PORT_KIND",
				format!(
					"Port {} has kind {:?}, but its contract requires {:?}.",
					actual_port.id().as_u64(),
					actual_port.kind(),
					expected_port.kind(),
				),
				GraphElementId::PORT(actual_port.id()),
			));
		}
	}
}

fn validate_lowered_operation(
	node: &Node,
	contract: &NodeContract,
	lowered: &LoweredOperation,
	diagnostics: &mut Diagnostics,
) -> bool {
	let expected_inputs: Vec<_> = contract
		.inputs()
		.iter()
		.filter(|port| port.kind() == PortKind::DATA)
		.collect();
	let expected_outputs: Vec<_> = contract
		.outputs()
		.iter()
		.filter(|port| port.kind() == PortKind::DATA)
		.collect();
	let mut valid = true;

	if lowered.input_types().len() != expected_inputs.len() {
		valid = false;
		diagnostics.push(contract_diagnostic(
			"CONTRACT.LOWERED_INPUTS",
			format!(
				"Node '{}' lowered {} input types, but its contract declares {} data inputs.",
				node.type_id().as_str(),
				lowered.input_types().len(),
				expected_inputs.len(),
			),
			GraphElementId::NODE(krunt_graph_model::NodeId::new(node.id().as_u64()).unwrap()),
		));
	}
	if lowered.output_types().len() != expected_outputs.len() {
		valid = false;
		diagnostics.push(contract_diagnostic(
			"CONTRACT.LOWERED_OUTPUTS",
			format!(
				"Node '{}' lowered {} output types, but its contract declares {} data outputs.",
				node.type_id().as_str(),
				lowered.output_types().len(),
				expected_outputs.len(),
			),
			GraphElementId::NODE(krunt_graph_model::NodeId::new(node.id().as_u64()).unwrap()),
		));
	}

	for (actual, expected) in lowered.input_types().iter().zip(expected_inputs) {
		if expected
			.type_id()
			.is_some_and(|expected| expected != actual)
		{
			valid = false;
			diagnostics.push(contract_diagnostic(
				"CONTRACT.LOWERED_TYPE",
				format!(
					"Node '{}' lowered an input type inconsistent with its contract.",
					node.type_id().as_str()
				),
				GraphElementId::NODE(krunt_graph_model::NodeId::new(node.id().as_u64()).unwrap()),
			));
		}
	}
	for (actual, expected) in lowered.output_types().iter().zip(expected_outputs) {
		if expected
			.type_id()
			.is_some_and(|expected| expected != actual)
		{
			valid = false;
			diagnostics.push(contract_diagnostic(
				"CONTRACT.LOWERED_TYPE",
				format!(
					"Node '{}' lowered an output type inconsistent with its contract.",
					node.type_id().as_str()
				),
				GraphElementId::NODE(krunt_graph_model::NodeId::new(node.id().as_u64()).unwrap()),
			));
		}
	}
	if lowered.async_kind() != contract.async_kind() {
		valid = false;
		diagnostics.push(contract_diagnostic(
			"CONTRACT.ASYNC_KIND",
			format!(
				"Node '{}' lowered execution semantics inconsistent with its contract.",
				node.type_id().as_str()
			),
			GraphElementId::NODE(krunt_graph_model::NodeId::new(node.id().as_u64()).unwrap()),
		));
	}
	if lowered.effects() != contract.effects() {
		valid = false;
		diagnostics.push(contract_diagnostic(
			"CONTRACT.EFFECTS",
			format!(
				"Node '{}' lowered side effects inconsistent with its contract.",
				node.type_id().as_str()
			),
			GraphElementId::NODE(krunt_graph_model::NodeId::new(node.id().as_u64()).unwrap()),
		));
	}
	valid
}

#[derive(Clone)]
struct GraphPortType {
	kind: PortKind,
	type_reference: Option<Type>,
}

fn analyze_data_types(document: &GraphDocument, diagnostics: &mut Diagnostics) {
	let mut port_types = BTreeMap::new();
	for node in document.nodes() {
		for port in node.ports() {
			let type_reference = port.type_reference().map(|reference| {
				Type::named(reference.as_str()).unwrap_or_else(|_| {
					unreachable!("validated graph type references cannot be empty")
				})
			});
			port_types.insert(
				port.id(),
				GraphPortType {
					kind: port.kind(),
					type_reference,
				},
			);
		}
	}

	for edge in document
		.edges()
		.iter()
		.filter(|edge| edge.kind() == EdgeKind::DATA)
	{
		let (Some(from), Some(to)) = (port_types.get(&edge.from()), port_types.get(&edge.to()))
		else {
			continue;
		};
		if from.kind != PortKind::DATA || to.kind != PortKind::DATA {
			continue;
		}

		let Some(from_type) = from.type_reference.as_ref() else {
			diagnostics.push(missing_type_reference_diagnostic(edge.from(), edge.id()));
			continue;
		};
		let Some(to_type) = to.type_reference.as_ref() else {
			diagnostics.push(missing_type_reference_diagnostic(edge.to(), edge.id()));
			continue;
		};

		if check_assignable(from_type, to_type).is_err() {
			diagnostics.push(type_mismatch_diagnostic(
				edge.from(),
				edge.to(),
				edge.id(),
				from_type,
				to_type,
			));
		}
	}
}

fn resolve_capabilities(
	document: &GraphDocument,
	contracts: &BTreeMap<krunt_graph_model::NodeId, NodeContract>,
	diagnostics: &mut Diagnostics,
) {
	let mut port_owners = BTreeMap::new();
	for node in document.nodes() {
		for port in node.ports() {
			port_owners.insert(port.id(), node.id());
		}
	}

	let mut predecessors: BTreeMap<krunt_graph_model::NodeId, BTreeSet<_>> = BTreeMap::new();
	for edge in document.edges() {
		let (Some(from), Some(to)) = (port_owners.get(&edge.from()), port_owners.get(&edge.to()))
		else {
			continue;
		};
		if from != to {
			predecessors.entry(*to).or_default().insert(*from);
		}
	}

	for node in document.nodes() {
		let Some(contract) = contracts.get(&node.id()) else {
			continue;
		};
		let mut available = BTreeSet::new();
		add_capabilities(contract, &mut available);
		if let Some(node_predecessors) = predecessors.get(&node.id()) {
			for predecessor in node_predecessors {
				if let Some(contract) = contracts.get(predecessor) {
					add_capabilities(contract, &mut available);
				}
			}
		}

		for requirement in contract.requirements() {
			if available.contains(requirement) {
				continue;
			}
			let related = predecessors
				.get(&node.id())
				.into_iter()
				.flatten()
				.map(|predecessor| RelatedElement::new(GraphElementId::NODE(*predecessor)))
				.collect::<Vec<_>>();
			let diagnostic = Diagnostic::new(
				DiagnosticCode::new("CAPABILITY.MISSING_REQUIREMENT").unwrap(),
				Severity::ERROR,
				CompilerStage::CAPABILITY_RESOLUTION,
				format!(
					"Node '{}' requires capability '{}', but no connected provider guarantees it.",
					node.type_id().as_str(),
					requirement.as_str(),
				),
				GraphElementId::NODE(node.id()),
			)
			.unwrap()
			.with_related(related);
			diagnostics.push(diagnostic);
		}
	}
}

fn add_capabilities(contract: &NodeContract, available: &mut BTreeSet<CapabilityId>) {
	available.extend(contract.capabilities().iter().cloned());
	available.extend(contract.guarantees().iter().cloned());
}

fn validate_whole_graph(
	document: &GraphDocument,
	contracts: &BTreeMap<krunt_graph_model::NodeId, NodeContract>,
	diagnostics: &mut Diagnostics,
) {
	if document.nodes().is_empty() {
		return;
	}

	let mut port_owners = BTreeMap::new();
	for node in document.nodes() {
		for port in node.ports() {
			port_owners.insert(port.id(), node.id());
		}
	}

	let mut adjacency: BTreeMap<krunt_graph_model::NodeId, BTreeSet<_>> = BTreeMap::new();
	for node in document.nodes() {
		adjacency.entry(node.id()).or_default();
	}
	for edge in document
		.edges()
		.iter()
		.filter(|edge| edge.kind() == EdgeKind::EXECUTION)
	{
		let (Some(from), Some(to)) = (port_owners.get(&edge.from()), port_owners.get(&edge.to()))
		else {
			continue;
		};
		adjacency.entry(*from).or_default().insert(*to);
	}
	validate_input_dependencies(document, contracts, diagnostics);

	let entry_nodes: Vec<_> = document
		.nodes()
		.iter()
		.filter(|node| {
			contracts
				.get(&node.id())
				.is_some_and(NodeContract::is_entry)
		})
		.map(Node::id)
		.collect();
	if entry_nodes.is_empty() {
		diagnostics.push(
			Diagnostic::new(
				DiagnosticCode::new("GRAPH.NO_ENTRY_POINT").unwrap(),
				Severity::ERROR,
				CompilerStage::WHOLE_GRAPH_VALIDATION,
				"The graph has no semantic entry point.",
				GraphElementId::GRAPH(
					krunt_graph_model::GraphId::new(document.metadata().graph_id().as_u64())
						.unwrap(),
				),
			)
			.unwrap(),
		);
	} else {
		let mut reachable = BTreeSet::new();
		let mut queue = VecDeque::from(entry_nodes);
		while let Some(node) = queue.pop_front() {
			if !reachable.insert(node) {
				continue;
			}
			if let Some(successors) = adjacency.get(&node) {
				queue.extend(successors.iter().copied());
			}
		}
		for node in document.nodes() {
			if !reachable.contains(&node.id()) {
				diagnostics.push(
					Diagnostic::new(
						DiagnosticCode::new("GRAPH.UNREACHABLE_NODE").unwrap(),
						Severity::ERROR,
						CompilerStage::WHOLE_GRAPH_VALIDATION,
						"This node is not reachable from any semantic entry point.",
						GraphElementId::NODE(node.id()),
					)
					.unwrap(),
				);
			}
		}
	}

	let mut states = BTreeMap::new();
	let mut stack = Vec::new();
	let mut cycles = BTreeSet::new();
	for node in document.nodes() {
		visit_execution_node(node.id(), &adjacency, &mut states, &mut stack, &mut cycles);
	}
	for cycle in cycles {
		let structured = cycle
			.iter()
			.any(|node| contracts.get(node).is_some_and(NodeContract::is_loop));
		if structured {
			continue;
		}
		for node in cycle {
			diagnostics.push(
				Diagnostic::new(
					DiagnosticCode::new("GRAPH.EXECUTION_CYCLE").unwrap(),
					Severity::ERROR,
					CompilerStage::WHOLE_GRAPH_VALIDATION,
					"Arbitrary execution cycles are not allowed; use a structured Loop node.",
					GraphElementId::NODE(node),
				)
				.unwrap(),
			);
		}
	}
}

fn validate_input_dependencies(
	document: &GraphDocument,
	contracts: &BTreeMap<krunt_graph_model::NodeId, NodeContract>,
	diagnostics: &mut Diagnostics,
) {
	let incoming_ports: BTreeSet<_> = document.edges().iter().map(|edge| edge.to()).collect();
	for node in document.nodes() {
		let Some(contract) = contracts.get(&node.id()) else {
			continue;
		};
		let inputs: Vec<_> = node
			.ports()
			.iter()
			.filter(|port| port.direction() == PortDirection::INPUT)
			.collect();
		for (port, expected) in inputs.iter().zip(contract.inputs()) {
			if expected.required() && !incoming_ports.contains(&port.id()) {
				diagnostics.push(
					Diagnostic::new(
						DiagnosticCode::new("GRAPH.MISSING_DEPENDENCY").unwrap(),
						Severity::ERROR,
						CompilerStage::WHOLE_GRAPH_VALIDATION,
						"A required input port has no incoming dependency edge.",
						GraphElementId::PORT(port.id()),
					)
					.unwrap(),
				);
			}
		}
	}
}

#[allow(non_camel_case_types)]
#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Copy)]
enum ExecutionVisitState {
	VISITING,
	VISITED,
}

fn visit_execution_node(
	node: krunt_graph_model::NodeId,
	adjacency: &BTreeMap<krunt_graph_model::NodeId, BTreeSet<krunt_graph_model::NodeId>>,
	states: &mut BTreeMap<krunt_graph_model::NodeId, ExecutionVisitState>,
	stack: &mut Vec<krunt_graph_model::NodeId>,
	cycles: &mut BTreeSet<BTreeSet<krunt_graph_model::NodeId>>,
) {
	match states.get(&node) {
		Some(ExecutionVisitState::VISITING) => {
			if let Some(start) = stack.iter().position(|current| *current == node) {
				cycles.insert(stack[start..].iter().copied().collect());
			}
			return;
		}
		Some(ExecutionVisitState::VISITED) => return,
		None => {}
	}

	states.insert(node, ExecutionVisitState::VISITING);
	stack.push(node);
	if let Some(successors) = adjacency.get(&node) {
		for successor in successors {
			visit_execution_node(*successor, adjacency, states, stack, cycles);
		}
	}
	stack.pop();
	states.insert(node, ExecutionVisitState::VISITED);
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

fn ir_node(
	node: &Node,
	id: krunt_ir::IrNodeId,
	lowered: LoweredOperation,
	contract: &NodeContract,
) -> IRNode {
	IRNode {
		id,
		op: if contract.is_entry() {
			IROp::ENTRY {
				name: lowered.operation().clone(),
			}
		} else {
			IROp::OPERATION {
				name: lowered.operation().clone(),
			}
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

fn contract_diagnostic(code: &str, message: String, primary: GraphElementId) -> Diagnostic {
	Diagnostic::new(
		DiagnosticCode::new(code).unwrap(),
		Severity::ERROR,
		CompilerStage::CONTRACT_VALIDATION,
		message,
		primary,
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

fn missing_type_reference_diagnostic(port: PortId, edge: krunt_graph_model::EdgeId) -> Diagnostic {
	Diagnostic::new(
		DiagnosticCode::new("TYPE.MISSING_REFERENCE").unwrap(),
		Severity::ERROR,
		CompilerStage::TYPE_ANALYSIS,
		"A data port connected by this edge is missing a type reference.",
		GraphElementId::PORT(port),
	)
	.unwrap()
	.with_related([RelatedElement::new(GraphElementId::EDGE(edge))])
}

fn type_mismatch_diagnostic(
	from_port: PortId,
	to_port: PortId,
	edge: krunt_graph_model::EdgeId,
	from_type: &Type,
	to_type: &Type,
) -> Diagnostic {
	let from_name = from_type.as_name().unwrap_or("any");
	let to_name = to_type.as_name().unwrap_or("any");
	Diagnostic::new(
		DiagnosticCode::new("TYPE.MISMATCH").unwrap(),
		Severity::ERROR,
		CompilerStage::TYPE_ANALYSIS,
		format!("Cannot connect data type '{from_name}' to incompatible type '{to_name}'."),
		GraphElementId::PORT(to_port),
	)
	.unwrap()
	.with_related([
		RelatedElement::new(GraphElementId::PORT(from_port)),
		RelatedElement::new(GraphElementId::EDGE(edge)),
	])
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

fn optimization_diagnostic(primary: GraphElementId, error: BuildError) -> Diagnostic {
	Diagnostic::new(
		DiagnosticCode::new("OPTIMIZATION.INVALID_IR").unwrap(),
		Severity::ERROR,
		CompilerStage::OPTIMIZATION,
		format!("The IR optimization pass found an invalid graph: {error:?}."),
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
	use krunt_plugin_api::{
		CapabilityId, NodeContract, PluginManifest, PortContract, SemanticPlugin,
	};

	struct BackendTestPlugin {
		manifest: PluginManifest,
		semantic_error: bool,
		contract_error: bool,
		lowered_type_mismatch: bool,
		capability_provider: bool,
		capability_requirement: bool,
		route_entry: bool,
	}

	impl SemanticPlugin for BackendTestPlugin {
		fn manifest(&self) -> &PluginManifest {
			&self.manifest
		}

		fn contract(&self, node: &Node) -> NodeContract {
			let execution = || PortContract::new(PortKind::EXECUTION, None, true);
			let data = || {
				PortContract::new(
					PortKind::DATA,
					Some(krunt_ir::TypeId::new("Request").unwrap()),
					true,
				)
			};
			let capability = || CapabilityId::new("request.source").unwrap();
			if node.type_id().as_str() == "test.route" {
				let outputs = if self.contract_error {
					vec![execution()]
				} else {
					vec![execution(), data()]
				};
				let capabilities = if self.capability_provider {
					vec![capability()]
				} else {
					Vec::new()
				};
				NodeContract::new([], outputs, [], capabilities, [], AsyncKind::SYNC, [])
					.with_entry(self.route_entry)
			} else {
				let requirements = if self.capability_requirement {
					vec![capability()]
				} else {
					Vec::new()
				};
				NodeContract::new(
					[execution(), data()],
					[],
					requirements,
					[],
					[],
					AsyncKind::SYNC,
					[],
				)
			}
		}

		fn analyze_node(&self, node: &Node) -> Vec<Diagnostic> {
			if self.semantic_error {
				return vec![Diagnostic::new(
					DiagnosticCode::new("SEMANTIC.TEST_FAILURE").unwrap(),
					Severity::ERROR,
					CompilerStage::SEMANTIC_ANALYSIS,
					"The test plugin rejected this node during semantic analysis.",
					GraphElementId::NODE(
						krunt_graph_model::NodeId::new(node.id().as_u64()).unwrap(),
					),
				)
				.unwrap()];
			}
			Vec::new()
		}

		fn lower_node(&self, node: &Node) -> Result<LoweredOperation, Vec<Diagnostic>> {
			let (inputs, outputs) = if node.type_id().as_str() == "test.route" {
				let output_type = if self.lowered_type_mismatch {
					"Response"
				} else {
					"Request"
				};
				(vec![], vec![krunt_ir::TypeId::new(output_type).unwrap()])
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
		document_with_response_type(Some("Request"))
	}

	fn document_with_response_type(response_type: Option<&str>) -> GraphDocument {
		document_with_options(response_type, true)
	}

	fn document_without_data_dependency() -> GraphDocument {
		document_with_options(Some("Request"), false)
	}

	fn document_with_options(
		response_type: Option<&str>,
		include_data_edge: bool,
	) -> GraphDocument {
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
		let response_port = Port::new(
			PortId::new(22).unwrap(),
			PortDirection::INPUT,
			PortKind::DATA,
		);
		let response_port = match response_type {
			Some(type_name) => {
				response_port.with_type_reference(TypeReference::new(type_name).unwrap())
			}
			None => response_port,
		};
		response.add_port(response_port).unwrap();
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
		if include_data_edge {
			builder
				.add_edge(krunt_graph_model::Edge::new(
					krunt_graph_model::EdgeId::new(32).unwrap(),
					PortId::new(12).unwrap(),
					PortId::new(22).unwrap(),
					EdgeKind::DATA,
				))
				.unwrap();
		}
		builder.build().unwrap()
	}

	fn cyclic_execution_document() -> GraphDocument {
		let metadata =
			GraphMetadata::new(GraphId::new(2).unwrap(), "backend-v1", "krunt-dev").unwrap();
		let mut first = Node::new(
			NodeId::new(1).unwrap(),
			NodeTypeId::new("test.route").unwrap(),
		);
		first
			.add_port(Port::new(
				PortId::new(101).unwrap(),
				PortDirection::INPUT,
				PortKind::EXECUTION,
			))
			.unwrap();
		first
			.add_port(Port::new(
				PortId::new(102).unwrap(),
				PortDirection::OUTPUT,
				PortKind::EXECUTION,
			))
			.unwrap();
		let mut second = Node::new(
			NodeId::new(2).unwrap(),
			NodeTypeId::new("test.response").unwrap(),
		);
		second
			.add_port(Port::new(
				PortId::new(201).unwrap(),
				PortDirection::INPUT,
				PortKind::EXECUTION,
			))
			.unwrap();
		second
			.add_port(Port::new(
				PortId::new(202).unwrap(),
				PortDirection::OUTPUT,
				PortKind::EXECUTION,
			))
			.unwrap();
		let mut builder = krunt_graph_model::GraphBuilder::new(metadata);
		builder.add_node(first).unwrap();
		builder.add_node(second).unwrap();
		builder
			.add_edge(krunt_graph_model::Edge::new(
				krunt_graph_model::EdgeId::new(301).unwrap(),
				PortId::new(102).unwrap(),
				PortId::new(201).unwrap(),
				EdgeKind::EXECUTION,
			))
			.unwrap();
		builder
			.add_edge(krunt_graph_model::Edge::new(
				krunt_graph_model::EdgeId::new(302).unwrap(),
				PortId::new(202).unwrap(),
				PortId::new(101).unwrap(),
				EdgeKind::EXECUTION,
			))
			.unwrap();
		builder.build().unwrap()
	}

	fn registry() -> PluginRegistry {
		registry_with_options(false, false, false, false, false, true)
	}

	fn registry_with_semantic_error(semantic_error: bool) -> PluginRegistry {
		registry_with_options(semantic_error, false, false, false, false, true)
	}

	fn registry_with_capabilities(provider: bool, requirement: bool) -> PluginRegistry {
		registry_with_options(false, false, false, provider, requirement, true)
	}

	fn registry_with_options(
		semantic_error: bool,
		contract_error: bool,
		lowered_type_mismatch: bool,
		capability_provider: bool,
		capability_requirement: bool,
		route_entry: bool,
	) -> PluginRegistry {
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
			.register(Box::new(BackendTestPlugin {
				manifest,
				semantic_error,
				contract_error,
				lowered_type_mismatch,
				capability_provider,
				capability_requirement,
				route_entry,
			}))
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
		assert!(matches!(
			result.ir().unwrap().node(krunt_ir::IrNodeId(1)).unwrap().op,
			IROp::ENTRY { .. }
		));
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
	fn semantic_plugin_diagnostics_block_ir_generation() {
		let result = compile(&document(), &registry_with_semantic_error(true));

		assert!(result.has_errors());
		assert!(result.ir().is_none());
		assert_eq!(
			result.diagnostics().as_slice()[0].code().as_str(),
			"SEMANTIC.TEST_FAILURE"
		);
		assert_eq!(
			result.diagnostics().as_slice()[0].stage(),
			CompilerStage::SEMANTIC_ANALYSIS
		);
	}

	#[test]
	fn graph_contract_violations_block_ir_generation() {
		let result = compile(
			&document(),
			&registry_with_options(false, true, false, false, false, true),
		);

		assert!(result.has_errors());
		assert!(result.ir().is_none());
		assert_eq!(
			result.diagnostics().as_slice()[0].code().as_str(),
			"CONTRACT.PORT_COUNT"
		);
		assert_eq!(
			result.diagnostics().as_slice()[0].stage(),
			CompilerStage::CONTRACT_VALIDATION
		);
	}

	#[test]
	fn lowered_operation_contract_violations_block_ir_generation() {
		let result = compile(
			&document(),
			&registry_with_options(false, false, true, false, false, true),
		);

		assert!(result.has_errors());
		assert!(result.ir().is_none());
		assert_eq!(
			result.diagnostics().as_slice()[0].code().as_str(),
			"CONTRACT.LOWERED_TYPE"
		);
		assert_eq!(
			result.diagnostics().as_slice()[0].primary(),
			GraphElementId::NODE(krunt_graph_model::NodeId::new(1).unwrap())
		);
	}

	#[test]
	fn missing_capability_requirements_block_ir_generation() {
		let result = compile(&document(), &registry_with_capabilities(false, true));

		assert!(result.has_errors());
		assert!(result.ir().is_none());
		let diagnostic = &result.diagnostics().as_slice()[0];
		assert_eq!(diagnostic.code().as_str(), "CAPABILITY.MISSING_REQUIREMENT");
		assert_eq!(diagnostic.stage(), CompilerStage::CAPABILITY_RESOLUTION);
		assert_eq!(diagnostic.related().len(), 1);
	}

	#[test]
	fn directly_connected_capabilities_satisfy_requirements() {
		let result = compile(&document(), &registry_with_capabilities(true, true));

		assert!(!result.has_errors());
		assert!(result.ir().is_some());
	}

	#[test]
	fn graphs_without_entry_points_block_ir_generation() {
		let result = compile(
			&document(),
			&registry_with_options(false, false, false, false, false, false),
		);

		assert!(result.has_errors());
		assert!(result.ir().is_none());
		assert_eq!(
			result.diagnostics().as_slice()[0].code().as_str(),
			"GRAPH.NO_ENTRY_POINT"
		);
		assert_eq!(
			result.diagnostics().as_slice()[0].stage(),
			CompilerStage::WHOLE_GRAPH_VALIDATION
		);
	}

	#[test]
	fn required_input_dependencies_must_be_connected() {
		let result = compile(&document_without_data_dependency(), &registry());

		assert!(result.has_errors());
		assert!(result.ir().is_none());
		assert_eq!(
			result.diagnostics().as_slice()[0].code().as_str(),
			"GRAPH.MISSING_DEPENDENCY"
		);
		assert_eq!(
			result.diagnostics().as_slice()[0].primary(),
			GraphElementId::PORT(PortId::new(22).unwrap())
		);
	}

	#[test]
	fn arbitrary_execution_cycles_are_rejected() {
		let document = cyclic_execution_document();
		let contracts = cyclic_contracts(false);
		let mut diagnostics = Diagnostics::new();
		validate_whole_graph(&document, &contracts, &mut diagnostics);

		assert_eq!(diagnostics.as_slice().len(), 2);
		assert!(diagnostics
			.as_slice()
			.iter()
			.all(|diagnostic| diagnostic.code().as_str() == "GRAPH.EXECUTION_CYCLE"));
	}

	#[test]
	fn structured_loop_contracts_allow_execution_cycles() {
		let document = cyclic_execution_document();
		let contracts = cyclic_contracts(true);
		let mut diagnostics = Diagnostics::new();
		validate_whole_graph(&document, &contracts, &mut diagnostics);

		assert!(diagnostics.is_empty());
	}

	fn cyclic_contracts(loop_node: bool) -> std::collections::BTreeMap<NodeId, NodeContract> {
		std::collections::BTreeMap::from([
			(
				NodeId::new(1).unwrap(),
				NodeContract::new(
					[PortContract::new(PortKind::EXECUTION, None, true)],
					[PortContract::new(PortKind::EXECUTION, None, true)],
					[],
					[],
					[],
					AsyncKind::SYNC,
					[],
				)
				.with_entry(true)
				.with_loop(loop_node),
			),
			(
				NodeId::new(2).unwrap(),
				NodeContract::new(
					[PortContract::new(PortKind::EXECUTION, None, true)],
					[PortContract::new(PortKind::EXECUTION, None, true)],
					[],
					[],
					[],
					AsyncKind::SYNC,
					[],
				),
			),
		])
	}

	#[test]
	fn incompatible_data_types_block_ir_generation_with_related_sources() {
		let result = compile(&document_with_response_type(Some("Response")), &registry());

		assert!(result.has_errors());
		assert!(result.ir().is_none());
		let diagnostic = &result.diagnostics().as_slice()[0];
		assert_eq!(diagnostic.code().as_str(), "TYPE.MISMATCH");
		assert_eq!(diagnostic.stage(), CompilerStage::TYPE_ANALYSIS);
		assert_eq!(
			diagnostic.primary(),
			GraphElementId::PORT(PortId::new(22).unwrap())
		);
		assert_eq!(diagnostic.related().len(), 2);
	}

	#[test]
	fn missing_data_type_reference_blocks_ir_generation() {
		let result = compile(&document_with_response_type(None), &registry());

		assert!(result.has_errors());
		assert!(result.ir().is_none());
		assert_eq!(
			result.diagnostics().as_slice()[0].code().as_str(),
			"TYPE.MISSING_REFERENCE"
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
