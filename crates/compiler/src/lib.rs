//! Pure compiler orchestration from a structural graph document to canonical IR.
//!
//! This crate owns stage coordination and diagnostics. Semantic meaning stays
//! in plugins, while IR invariants stay in the IR crate.

use krunt_diagnostics::{
	CompilerStage, Diagnostic, DiagnosticCode, Diagnostics, GraphElementId, RelatedElement,
	Severity,
};
use krunt_graph_model::{EdgeKind, GraphDocument, Node, NodeId, PortDirection, PortId, PortKind};
use krunt_ir::{
	eliminate_unreachable_nodes, BuildError, ControlEdge, DataEdge, GraphNodeId, GraphSubgraphId,
	IRBuilder, IRGraph, IRNode, IROp, IRSubgraph, IRSubgraphBoundary, PortRef, SourceMap, TypeId,
};
use krunt_plugin_api::{
	CapabilityId, LoweredOperation, LoweredOperationKind, NodeContract, PluginRegistry,
	PortContract,
};
use krunt_type_system::{solve_constraints, Type, TypeConstraint, TypeSubstitution, TypeVariable};
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
	diagnostics.extend(plugins.analyze_graph(document));

	let inferred_types = if !diagnostics.has_errors() {
		infer_data_types(document, &contracts, &mut diagnostics)
	} else {
		BTreeMap::new()
	};
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

		match plugin.lower_node_in_document(node, document) {
			Ok(lowered) => {
				let lowered = specialize_lowered_operation(node, lowered, &inferred_types);
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
		let subgraph_ports: BTreeSet<_> = document
			.subgraphs()
			.iter()
			.flat_map(|subgraph| subgraph.ports().iter().map(|port| port.id()))
			.collect();
		for edge in document.edges() {
			if subgraph_ports.contains(&edge.from()) || subgraph_ports.contains(&edge.to()) {
				continue;
			}
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
		build_ir_subgraphs(document, &port_bindings, &mut builder, &mut diagnostics);
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
			.is_some_and(|expected| expected.as_str() != "ANY" && expected != actual)
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
			.is_some_and(|expected| expected.as_str() != "ANY" && expected != actual)
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
	if lowered.kind() == LoweredOperationKind::SUBGRAPH && lowered.subgraph_id().is_none() {
		valid = false;
		diagnostics.push(contract_diagnostic(
			"CONTRACT.SUBGRAPH_TARGET",
			format!(
				"Node '{}' lowered as a subgraph without a target identity.",
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

struct InferredDataConstraint {
	from_type: Type,
	to_type: Type,
	constraint: TypeConstraint,
	source: InferenceConstraintSource,
}

#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum InferenceConstraintSource {
	CONTRACT {
		node: NodeId,
		port: PortId,
	},
	EDGE {
		edge: krunt_graph_model::EdgeId,
		from_port: PortId,
		to_port: PortId,
	},
}

fn infer_data_types(
	document: &GraphDocument,
	contracts: &BTreeMap<NodeId, NodeContract>,
	diagnostics: &mut Diagnostics,
) -> BTreeMap<PortId, Type> {
	let mut port_types = BTreeMap::new();
	for node in document.nodes() {
		for port in node.ports() {
			let type_reference = port.type_reference().map(|reference| {
				let parsed = Type::from_reference(reference.as_str()).unwrap_or_else(|_| {
					unreachable!("validated graph type references cannot be empty")
				});
				replace_open_types(parsed, TypeVariable::new(port.id().as_u64()))
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

	let mut constraints = Vec::new();
	for node in document.nodes() {
		let Some(contract) = contracts.get(&node.id()) else {
			continue;
		};
		let data_inputs: Vec<_> = node
			.ports()
			.iter()
			.filter(|port| {
				port.direction() == PortDirection::INPUT && port.kind() == PortKind::DATA
			})
			.collect();
		let data_outputs: Vec<_> = node
			.ports()
			.iter()
			.filter(|port| {
				port.direction() == PortDirection::OUTPUT && port.kind() == PortKind::DATA
			})
			.collect();
		add_contract_constraints(
			node,
			&data_inputs,
			&contract
				.inputs()
				.iter()
				.filter(|port| port.kind() == PortKind::DATA)
				.collect::<Vec<_>>(),
			&port_types,
			&mut constraints,
			diagnostics,
		);
		add_contract_constraints(
			node,
			&data_outputs,
			&contract
				.outputs()
				.iter()
				.filter(|port| port.kind() == PortKind::DATA)
				.collect::<Vec<_>>(),
			&port_types,
			&mut constraints,
			diagnostics,
		);
	}

	let mut data_edges: Vec<_> = document
		.edges()
		.iter()
		.filter(|edge| edge.kind() == EdgeKind::DATA)
		.collect();
	data_edges.sort_by_key(|edge| edge.id());
	for edge in data_edges {
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
		constraints.push(InferredDataConstraint {
			from_type: from_type.clone(),
			to_type: to_type.clone(),
			constraint: TypeConstraint::assignable(from_type.clone(), to_type.clone()),
			source: InferenceConstraintSource::EDGE {
				edge: edge.id(),
				from_port: edge.from(),
				to_port: edge.to(),
			},
		});
	}

	constraints.sort_by_key(|constraint| constraint.source.clone());
	let mut accepted = Vec::new();
	for candidate in constraints {
		accepted.push(candidate);
		if solve_constraints(accepted.iter().map(|item| item.constraint.clone())).is_err() {
			let failed = accepted
				.pop()
				.expect("a failed constraint was just appended");
			let substitution =
				solve_constraints(accepted.iter().map(|item| item.constraint.clone()))
					.unwrap_or_default();
			let from_type = substitution.resolve(&failed.from_type);
			let to_type = substitution.resolve(&failed.to_type);
			match failed.source {
				InferenceConstraintSource::EDGE {
					edge,
					from_port,
					to_port,
				} => diagnostics.push(type_mismatch_diagnostic(
					from_port, to_port, edge, &from_type, &to_type,
				)),
				InferenceConstraintSource::CONTRACT { node, port } => diagnostics.push(
					contract_type_mismatch_diagnostic(node, port, &from_type, &to_type),
				),
			}
		}
	}

	let substitution =
		solve_constraints(accepted.iter().map(|item| item.constraint.clone())).unwrap_or_default();
	port_types
		.into_iter()
		.filter_map(|(port, graph_type)| {
			graph_type
				.type_reference
				.map(|type_value| (port, materialize_inferred_type(&substitution, &type_value)))
		})
		.collect()
}

fn add_contract_constraints(
	node: &Node,
	ports: &[&krunt_graph_model::Port],
	contracts: &[&PortContract],
	port_types: &BTreeMap<PortId, GraphPortType>,
	constraints: &mut Vec<InferredDataConstraint>,
	diagnostics: &mut Diagnostics,
) {
	for (port, contract) in ports.iter().zip(contracts) {
		let Some(type_id) = contract.type_id() else {
			continue;
		};
		let Ok(contract_type) = Type::from_reference(type_id.as_str()) else {
			diagnostics.push(invalid_contract_type_diagnostic(node, port.id(), type_id));
			continue;
		};
		if contract_type == Type::ANY {
			continue;
		}
		let Some(Some(port_type)) = port_types.get(&port.id()).map(|port| &port.type_reference)
		else {
			continue;
		};
		constraints.push(InferredDataConstraint {
			from_type: port_type.clone(),
			to_type: contract_type.clone(),
			constraint: TypeConstraint::assignable(port_type.clone(), contract_type),
			source: InferenceConstraintSource::CONTRACT {
				node: node.id(),
				port: port.id(),
			},
		});
	}
}

fn replace_open_types(type_value: Type, variable: TypeVariable) -> Type {
	match type_value {
		Type::ANY => Type::variable(variable),
		Type::ARRAY(element) => Type::array(replace_open_types(*element, variable)),
		Type::SET(element) => Type::set(replace_open_types(*element, variable)),
		Type::MAP(key, value) => Type::map(
			replace_open_types(*key, variable),
			replace_open_types(*value, variable),
		),
		Type::OPTION(value) => Type::option(replace_open_types(*value, variable)),
		Type::RESULT(value, error) => Type::result(
			replace_open_types(*value, variable),
			replace_open_types(*error, variable),
		),
		Type::FUTURE(value) => Type::future(replace_open_types(*value, variable)),
		Type::STREAM(value) => Type::stream(replace_open_types(*value, variable)),
		Type::UNION(members) => Type::union(
			members
				.into_iter()
				.map(|member| replace_open_types(member, variable)),
		)
		.expect("a validated union cannot be empty"),
		Type::INTERSECTION(members) => Type::intersection(
			members
				.into_iter()
				.map(|member| replace_open_types(member, variable)),
		)
		.expect("a validated intersection cannot be empty"),
		Type::RECORD(fields) => Type::record(
			fields
				.into_iter()
				.map(|(name, field)| {
					(
						name,
						krunt_type_system::RecordField::new(
							replace_open_types(field.type_value().clone(), variable),
							field.required(),
						),
					)
				})
				.collect(),
		),
		other => other,
	}
}

fn materialize_inferred_type(substitution: &TypeSubstitution, type_value: &Type) -> Type {
	match substitution.resolve(type_value) {
		Type::VARIABLE(_) => Type::ANY,
		Type::ARRAY(element) => Type::array(materialize_inferred_type(substitution, &element)),
		Type::SET(element) => Type::set(materialize_inferred_type(substitution, &element)),
		Type::MAP(key, value) => Type::map(
			materialize_inferred_type(substitution, &key),
			materialize_inferred_type(substitution, &value),
		),
		Type::OPTION(value) => Type::option(materialize_inferred_type(substitution, &value)),
		Type::RESULT(value, error) => Type::result(
			materialize_inferred_type(substitution, &value),
			materialize_inferred_type(substitution, &error),
		),
		Type::FUTURE(value) => Type::future(materialize_inferred_type(substitution, &value)),
		Type::STREAM(value) => Type::stream(materialize_inferred_type(substitution, &value)),
		Type::UNION(members) => Type::union(
			members
				.into_iter()
				.map(|member| materialize_inferred_type(substitution, &member)),
		)
		.expect("a resolved union cannot be empty"),
		Type::INTERSECTION(members) => Type::intersection(
			members
				.into_iter()
				.map(|member| materialize_inferred_type(substitution, &member)),
		)
		.expect("a resolved intersection cannot be empty"),
		Type::RECORD(fields) => Type::record(
			fields
				.into_iter()
				.map(|(name, field)| {
					(
						name,
						krunt_type_system::RecordField::new(
							materialize_inferred_type(substitution, field.type_value()),
							field.required(),
						),
					)
				})
				.collect(),
		),
		resolved => resolved,
	}
}

fn specialize_lowered_operation(
	node: &Node,
	lowered: LoweredOperation,
	inferred_types: &BTreeMap<PortId, Type>,
) -> LoweredOperation {
	let data_inputs: Vec<_> = node
		.ports()
		.iter()
		.filter(|port| port.direction() == PortDirection::INPUT && port.kind() == PortKind::DATA)
		.collect();
	let data_outputs: Vec<_> = node
		.ports()
		.iter()
		.filter(|port| port.direction() == PortDirection::OUTPUT && port.kind() == PortKind::DATA)
		.collect();
	let input_types = specialize_type_ids(lowered.input_types(), &data_inputs, inferred_types);
	let output_types = specialize_type_ids(lowered.output_types(), &data_outputs, inferred_types);
	lowered
		.with_input_types(input_types)
		.with_output_types(output_types)
}

fn specialize_type_ids(
	type_ids: &[TypeId],
	ports: &[&krunt_graph_model::Port],
	inferred_types: &BTreeMap<PortId, Type>,
) -> Vec<TypeId> {
	type_ids
		.iter()
		.enumerate()
		.map(|(index, type_id)| {
			if type_id.as_str() != "ANY" {
				return type_id.clone();
			}
			let Some(port) = ports.get(index) else {
				return type_id.clone();
			};
			let Some(type_value) = inferred_types.get(&port.id()) else {
				return type_id.clone();
			};
			TypeId::new(type_value.to_reference()).expect("canonical type references are non-empty")
		})
		.collect()
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
			!document
				.subgraphs()
				.iter()
				.any(|subgraph| subgraph.nodes().contains(&node.id()))
				&& contracts
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
			if document
				.subgraphs()
				.iter()
				.any(|subgraph| subgraph.nodes().contains(&node.id()))
			{
				continue;
			}
			if contracts
				.get(&node.id())
				.is_some_and(|contract| is_declaration_node(node, contract))
			{
				continue;
			}
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

fn is_declaration_node(node: &Node, contract: &NodeContract) -> bool {
	node.ports().is_empty() && contract.inputs().is_empty() && contract.outputs().is_empty()
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
	let mut execution_input_index = 0u16;
	let mut execution_output_index = 0u16;
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
			(PortDirection::INPUT, PortKind::EXECUTION) => {
				let index = execution_input_index;
				execution_input_index = execution_input_index.saturating_add(1);
				index
			}
			(PortDirection::OUTPUT, PortKind::EXECUTION) => {
				let index = execution_output_index;
				execution_output_index = execution_output_index.saturating_add(1);
				index
			}
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
	if input_index == u16::MAX
		|| output_index == u16::MAX
		|| execution_input_index == u16::MAX
		|| execution_output_index == u16::MAX
	{
		diagnostics.push(
			Diagnostic::new(
				DiagnosticCode::new("LOWERING.TOO_MANY_PORTS").unwrap(),
				Severity::ERROR,
				CompilerStage::LOWERING,
				"A node has too many ports for the canonical IR.",
				GraphElementId::NODE(krunt_graph_model::NodeId::new(node.id().as_u64()).unwrap()),
			)
			.unwrap(),
		);
	}
}

fn build_ir_subgraphs(
	document: &GraphDocument,
	port_bindings: &BTreeMap<PortId, PortBinding>,
	builder: &mut IRBuilder,
	diagnostics: &mut Diagnostics,
) {
	let node_ir_ids: BTreeMap<_, _> = document
		.nodes()
		.iter()
		.enumerate()
		.map(|(index, node)| (node.id(), krunt_ir::IrNodeId((index + 1) as u32)))
		.collect();

	for subgraph in document.subgraphs() {
		let nodes: Vec<_> = subgraph
			.nodes()
			.iter()
			.filter_map(|node| node_ir_ids.get(node).copied())
			.collect();
		let mut inputs = Vec::new();
		let mut outputs = Vec::new();

		for source_boundary in subgraph.ports() {
			let Some(edge) =
				document
					.edges()
					.iter()
					.find(|edge| match source_boundary.direction() {
						PortDirection::OUTPUT => edge.from() == source_boundary.id(),
						PortDirection::INPUT => edge.to() == source_boundary.id(),
					})
			else {
				continue;
			};
			let connected_port = match source_boundary.direction() {
				PortDirection::OUTPUT => edge.to(),
				PortDirection::INPUT => edge.from(),
			};
			let Some(binding) = port_bindings.get(&connected_port).copied() else {
				diagnostics.push(missing_port_diagnostic(connected_port));
				continue;
			};
			if !subgraph
				.nodes()
				.iter()
				.any(|node| node_ir_ids.get(node) == Some(&binding.ir_node))
			{
				continue;
			}
			let boundary = IRSubgraphBoundary {
				port: krunt_ir::GraphPortId(source_boundary.id().as_u64()),
				target: PortRef {
					node: binding.ir_node,
					index: binding.index,
				},
			};
			match source_boundary.direction() {
				PortDirection::OUTPUT => inputs.push(boundary),
				PortDirection::INPUT => outputs.push(boundary),
			}
		}

		let ir_subgraph = IRSubgraph {
			id: GraphSubgraphId(subgraph.id().as_u64()),
			nodes,
			inputs,
			outputs,
		};
		if let Err(error) = builder.add_subgraph(ir_subgraph) {
			diagnostics.push(ir_build_diagnostic(
				GraphElementId::GRAPH(
					krunt_graph_model::GraphId::new(document.metadata().graph_id().as_u64())
						.unwrap(),
				),
				error,
			));
		}
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
		from: PortRef {
			node: from.ir_node,
			index: from.index,
		},
		to: PortRef {
			node: to.ir_node,
			index: to.index,
		},
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
		op: if lowered.returns() {
			IROp::RETURN
		} else if contract.is_entry() {
			IROp::ENTRY {
				name: lowered.operation().clone(),
			}
		} else {
			match lowered.kind() {
				LoweredOperationKind::DECLARATION => IROp::DECLARATION {
					name: lowered.operation().clone(),
				},
				LoweredOperationKind::OPERATION => IROp::OPERATION {
					name: lowered.operation().clone(),
				},
				LoweredOperationKind::BRANCH => IROp::BRANCH,
				LoweredOperationKind::TRY_CATCH => IROp::TRY_CATCH,
				LoweredOperationKind::MERGE => IROp::MERGE,
				LoweredOperationKind::LOOP => IROp::LOOP,
				LoweredOperationKind::SUBGRAPH => IROp::SUBGRAPH {
					id: GraphSubgraphId(
						lowered
							.subgraph_id()
							.expect("subgraph lowering was validated before IR construction"),
					),
				},
			}
		},
		input_types: lowered.input_types().to_vec(),
		output_types: lowered.output_types().to_vec(),
		async_kind: lowered.async_kind(),
		effects: lowered.effects().to_vec(),
		attributes: lowered.attributes().clone(),
		returns: lowered.returns(),
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

fn invalid_contract_type_diagnostic(node: &Node, port: PortId, type_id: &TypeId) -> Diagnostic {
	Diagnostic::new(
		DiagnosticCode::new("TYPE.CONTRACT_REFERENCE").unwrap(),
		Severity::ERROR,
		CompilerStage::TYPE_ANALYSIS,
		format!(
			"Node contract '{}' declares invalid data type '{}'.",
			node.type_id().as_str(),
			type_id.as_str(),
		),
		GraphElementId::PORT(port),
	)
	.unwrap()
	.with_related([RelatedElement::new(GraphElementId::NODE(node.id()))])
}

fn type_mismatch_diagnostic(
	from_port: PortId,
	to_port: PortId,
	edge: krunt_graph_model::EdgeId,
	from_type: &Type,
	to_type: &Type,
) -> Diagnostic {
	let from_name = from_type.to_reference();
	let to_name = to_type.to_reference();
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

fn contract_type_mismatch_diagnostic(
	node: NodeId,
	port: PortId,
	from_type: &Type,
	to_type: &Type,
) -> Diagnostic {
	let from_name = from_type.to_reference();
	let to_name = to_type.to_reference();
	Diagnostic::new(
		DiagnosticCode::new("TYPE.CONTRACT_MISMATCH").unwrap(),
		Severity::ERROR,
		CompilerStage::TYPE_ANALYSIS,
		format!("Data type '{from_name}' does not satisfy the node contract type '{to_name}'."),
		GraphElementId::PORT(port),
	)
	.unwrap()
	.with_related([RelatedElement::new(GraphElementId::NODE(node))])
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
	use krunt_backend_dsl::{
		BackendDslPlugin, DATABASE_READ_EFFECT, ENTITY_FIELDS_PROPERTY, ENTITY_NAME_PROPERTY,
		ENTITY_NODE_TYPE, HTTP_REQUEST_EFFECT, HTTP_REQUEST_METHOD_PROPERTY,
		HTTP_REQUEST_NODE_TYPE, HTTP_REQUEST_TYPE, HTTP_REQUEST_URL_PROPERTY, HTTP_ROUTE_NODE_TYPE,
		HTTP_ROUTE_REQUEST_ENTITY_PROPERTY, LOGGING_EFFECT, LOGGING_LEVEL_PROPERTY,
		LOGGING_NODE_TYPE, QUERY_CARDINALITY_PROPERTY, QUERY_NAME_PROPERTY, QUERY_NODE_TYPE,
		QUERY_REPOSITORY_PROPERTY, REPOSITORY_ENTITY_PROPERTY, REPOSITORY_NAME_PROPERTY,
		REPOSITORY_NODE_TYPE, RESPONSE_BODY_TYPE_PROPERTY, STORAGE_KEY_PROPERTY, STORAGE_NODE_TYPE,
		STORAGE_OPERATION_PROPERTY, STORAGE_READ_EFFECT, SUCCESS_RESPONSE_NODE_TYPE,
	};
	use krunt_graph_model::{
		Edge, EdgeId, EdgeKind, GraphBuilder, GraphId, GraphMetadata, NodeId, NodeTypeId,
		PluginReference, Port, PortDirection, PortId, PortKind, TypeReference,
	};
	use krunt_ir::{AsyncKind, EffectId, IROp};
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

	fn persistence_document() -> GraphDocument {
		let metadata =
			GraphMetadata::new(GraphId::new(3).unwrap(), "backend-v1", "krunt-dev").unwrap();
		let mut route = Node::new(
			NodeId::new(1).unwrap(),
			NodeTypeId::new(HTTP_ROUTE_NODE_TYPE).unwrap(),
		)
		.with_property("method", serde_json::Value::String("GET".into()))
		.unwrap()
		.with_property("path", serde_json::Value::String("/users".into()))
		.unwrap()
		.with_property(
			HTTP_ROUTE_REQUEST_ENTITY_PROPERTY,
			serde_json::Value::String("User".into()),
		)
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

		let mut query = Node::new(
			NodeId::new(2).unwrap(),
			NodeTypeId::new(QUERY_NODE_TYPE).unwrap(),
		)
		.with_property(
			QUERY_NAME_PROPERTY,
			serde_json::Value::String("ListUsers".into()),
		)
		.unwrap()
		.with_property(
			QUERY_REPOSITORY_PROPERTY,
			serde_json::Value::String("UserRepository".into()),
		)
		.unwrap();
		query = query
			.with_property(
				QUERY_CARDINALITY_PROPERTY,
				serde_json::Value::String("many".into()),
			)
			.unwrap();
		query
			.add_port(Port::new(
				PortId::new(21).unwrap(),
				PortDirection::INPUT,
				PortKind::EXECUTION,
			))
			.unwrap();
		query
			.add_port(Port::new(
				PortId::new(22).unwrap(),
				PortDirection::OUTPUT,
				PortKind::EXECUTION,
			))
			.unwrap();
		query
			.add_port(
				Port::new(
					PortId::new(23).unwrap(),
					PortDirection::OUTPUT,
					PortKind::DATA,
				)
				.with_type_reference(TypeReference::new("ANY").unwrap()),
			)
			.unwrap();

		let mut response = Node::new(
			NodeId::new(3).unwrap(),
			NodeTypeId::new(SUCCESS_RESPONSE_NODE_TYPE).unwrap(),
		);
		response = response
			.with_property(
				RESPONSE_BODY_TYPE_PROPERTY,
				serde_json::Value::String("Array<User>".into()),
			)
			.unwrap();
		response
			.add_port(Port::new(
				PortId::new(31).unwrap(),
				PortDirection::INPUT,
				PortKind::EXECUTION,
			))
			.unwrap();
		response
			.add_port(
				Port::new(
					PortId::new(32).unwrap(),
					PortDirection::INPUT,
					PortKind::DATA,
				)
				.with_type_reference(TypeReference::new("ANY").unwrap()),
			)
			.unwrap();

		let entity = Node::new(
			NodeId::new(4).unwrap(),
			NodeTypeId::new(ENTITY_NODE_TYPE).unwrap(),
		)
		.with_property(
			ENTITY_NAME_PROPERTY,
			serde_json::Value::String("User".into()),
		)
		.unwrap()
		.with_property(
			ENTITY_FIELDS_PROPERTY,
			serde_json::json!({
				"email": {"type": "String", "required": false},
				"id": {"type": "Integer", "required": true},
				"profile": {
					"type": {
						"kind": "record",
						"fields": {
							"displayName": {"type": "String", "required": true},
							"bio": {"type": "String", "required": false}
						}
					},
					"required": false
				},
				"tags": {
					"type": {"kind": "array", "items": "String"},
					"required": true
				}
			}),
		)
		.unwrap();
		let repository = Node::new(
			NodeId::new(5).unwrap(),
			NodeTypeId::new(REPOSITORY_NODE_TYPE).unwrap(),
		)
		.with_property(
			REPOSITORY_NAME_PROPERTY,
			serde_json::Value::String("UserRepository".into()),
		)
		.unwrap()
		.with_property(
			REPOSITORY_ENTITY_PROPERTY,
			serde_json::Value::String("User".into()),
		)
		.unwrap();

		let mut builder = GraphBuilder::new(metadata);
		for node in [route, query, response, entity, repository] {
			builder.add_node(node).unwrap();
		}
		for edge in [
			Edge::new(
				EdgeId::new(101).unwrap(),
				PortId::new(11).unwrap(),
				PortId::new(21).unwrap(),
				EdgeKind::EXECUTION,
			),
			Edge::new(
				EdgeId::new(102).unwrap(),
				PortId::new(22).unwrap(),
				PortId::new(31).unwrap(),
				EdgeKind::EXECUTION,
			),
			Edge::new(
				EdgeId::new(103).unwrap(),
				PortId::new(23).unwrap(),
				PortId::new(32).unwrap(),
				EdgeKind::DATA,
			),
		] {
			builder.add_edge(edge).unwrap();
		}
		builder.build().unwrap()
	}

	fn effects_document() -> GraphDocument {
		let metadata =
			GraphMetadata::new(GraphId::new(4).unwrap(), "backend-v1", "krunt-dev").unwrap();
		let mut route = Node::new(
			NodeId::new(1).unwrap(),
			NodeTypeId::new(HTTP_ROUTE_NODE_TYPE).unwrap(),
		)
		.with_property("method", serde_json::Value::String("GET".into()))
		.unwrap()
		.with_property("path", serde_json::Value::String("/effects".into()))
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

		let mut http_request = Node::new(
			NodeId::new(2).unwrap(),
			NodeTypeId::new(HTTP_REQUEST_NODE_TYPE).unwrap(),
		)
		.with_property(
			HTTP_REQUEST_METHOD_PROPERTY,
			serde_json::Value::String("GET".into()),
		)
		.unwrap()
		.with_property(
			HTTP_REQUEST_URL_PROPERTY,
			serde_json::Value::String("https://example.test/users".into()),
		)
		.unwrap();
		http_request
			.add_port(Port::new(
				PortId::new(21).unwrap(),
				PortDirection::INPUT,
				PortKind::EXECUTION,
			))
			.unwrap();
		http_request
			.add_port(
				Port::new(
					PortId::new(22).unwrap(),
					PortDirection::INPUT,
					PortKind::DATA,
				)
				.with_type_reference(TypeReference::new("ANY").unwrap()),
			)
			.unwrap();
		http_request
			.add_port(Port::new(
				PortId::new(23).unwrap(),
				PortDirection::OUTPUT,
				PortKind::EXECUTION,
			))
			.unwrap();
		http_request
			.add_port(
				Port::new(
					PortId::new(24).unwrap(),
					PortDirection::OUTPUT,
					PortKind::DATA,
				)
				.with_type_reference(TypeReference::new("ANY").unwrap()),
			)
			.unwrap();

		let mut storage = Node::new(
			NodeId::new(3).unwrap(),
			NodeTypeId::new(STORAGE_NODE_TYPE).unwrap(),
		)
		.with_property(
			STORAGE_OPERATION_PROPERTY,
			serde_json::Value::String("READ".into()),
		)
		.unwrap()
		.with_property(
			STORAGE_KEY_PROPERTY,
			serde_json::Value::String("users".into()),
		)
		.unwrap();
		storage
			.add_port(Port::new(
				PortId::new(31).unwrap(),
				PortDirection::INPUT,
				PortKind::EXECUTION,
			))
			.unwrap();
		storage
			.add_port(
				Port::new(
					PortId::new(32).unwrap(),
					PortDirection::INPUT,
					PortKind::DATA,
				)
				.with_type_reference(TypeReference::new("ANY").unwrap()),
			)
			.unwrap();
		storage
			.add_port(Port::new(
				PortId::new(33).unwrap(),
				PortDirection::OUTPUT,
				PortKind::EXECUTION,
			))
			.unwrap();
		storage
			.add_port(
				Port::new(
					PortId::new(34).unwrap(),
					PortDirection::OUTPUT,
					PortKind::DATA,
				)
				.with_type_reference(TypeReference::new("ANY").unwrap()),
			)
			.unwrap();

		let mut logging = Node::new(
			NodeId::new(4).unwrap(),
			NodeTypeId::new(LOGGING_NODE_TYPE).unwrap(),
		)
		.with_property(
			LOGGING_LEVEL_PROPERTY,
			serde_json::Value::String("INFO".into()),
		)
		.unwrap();
		logging
			.add_port(Port::new(
				PortId::new(41).unwrap(),
				PortDirection::INPUT,
				PortKind::EXECUTION,
			))
			.unwrap();
		logging
			.add_port(
				Port::new(
					PortId::new(42).unwrap(),
					PortDirection::INPUT,
					PortKind::DATA,
				)
				.with_type_reference(TypeReference::new("ANY").unwrap()),
			)
			.unwrap();
		logging
			.add_port(Port::new(
				PortId::new(43).unwrap(),
				PortDirection::OUTPUT,
				PortKind::EXECUTION,
			))
			.unwrap();

		let mut response = Node::new(
			NodeId::new(5).unwrap(),
			NodeTypeId::new(SUCCESS_RESPONSE_NODE_TYPE).unwrap(),
		);
		response
			.add_port(Port::new(
				PortId::new(51).unwrap(),
				PortDirection::INPUT,
				PortKind::EXECUTION,
			))
			.unwrap();
		response
			.add_port(
				Port::new(
					PortId::new(52).unwrap(),
					PortDirection::INPUT,
					PortKind::DATA,
				)
				.with_type_reference(TypeReference::new("ANY").unwrap()),
			)
			.unwrap();

		let mut builder = GraphBuilder::new(metadata);
		for node in [route, http_request, storage, logging, response] {
			builder.add_node(node).unwrap();
		}
		for edge in [
			Edge::new(
				EdgeId::new(201).unwrap(),
				PortId::new(11).unwrap(),
				PortId::new(21).unwrap(),
				EdgeKind::EXECUTION,
			),
			Edge::new(
				EdgeId::new(202).unwrap(),
				PortId::new(23).unwrap(),
				PortId::new(31).unwrap(),
				EdgeKind::EXECUTION,
			),
			Edge::new(
				EdgeId::new(203).unwrap(),
				PortId::new(33).unwrap(),
				PortId::new(41).unwrap(),
				EdgeKind::EXECUTION,
			),
			Edge::new(
				EdgeId::new(204).unwrap(),
				PortId::new(43).unwrap(),
				PortId::new(51).unwrap(),
				EdgeKind::EXECUTION,
			),
			Edge::new(
				EdgeId::new(205).unwrap(),
				PortId::new(24).unwrap(),
				PortId::new(42).unwrap(),
				EdgeKind::DATA,
			),
		] {
			builder.add_edge(edge).unwrap();
		}
		builder.build().unwrap()
	}

	fn document_with_response_type(response_type: Option<&str>) -> GraphDocument {
		document_with_options(response_type, true)
	}

	fn document_with_open_data_types() -> GraphDocument {
		document_with_route_type(Some("ANY"), Some("ANY"), true)
	}

	fn document_without_data_dependency() -> GraphDocument {
		document_with_options(Some("Request"), false)
	}

	fn document_with_options(
		response_type: Option<&str>,
		include_data_edge: bool,
	) -> GraphDocument {
		document_with_route_type(Some("Request"), response_type, include_data_edge)
	}

	fn document_with_route_type(
		route_type: Option<&str>,
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
		let route_port = Port::new(
			PortId::new(12).unwrap(),
			PortDirection::OUTPUT,
			PortKind::DATA,
		);
		let route_port = match route_type {
			Some(type_name) => {
				route_port.with_type_reference(TypeReference::new(type_name).unwrap())
			}
			None => route_port,
		};
		route.add_port(route_port).unwrap();
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

	fn contracts_for(
		document: &GraphDocument,
		registry: &PluginRegistry,
	) -> BTreeMap<NodeId, NodeContract> {
		document
			.nodes()
			.iter()
			.map(|node| {
				(
					node.id(),
					registry
						.resolve(node.type_id())
						.expect("test nodes must have registered plugins")
						.contract(node),
				)
			})
			.collect()
	}

	#[test]
	fn compiles_plugin_lowered_nodes_and_both_edge_kinds() {
		let result = compile(&document(), &registry());

		assert!(!result.has_errors());
		assert_eq!(result.diagnostics().as_slice().len(), 0);
		assert_eq!(result.ir().unwrap().nodes().len(), 2);
		assert_eq!(result.ir().unwrap().control_edges().len(), 1);
		assert_eq!(result.ir().unwrap().data_edges().len(), 1);
		assert_eq!(
			result.ir().unwrap().control_edges()[0].from,
			PortRef {
				node: krunt_ir::IrNodeId(1),
				index: 0,
			}
		);
		assert_eq!(
			result.ir().unwrap().control_edges()[0].to,
			PortRef {
				node: krunt_ir::IrNodeId(2),
				index: 0,
			}
		);
		assert!(matches!(
			result.ir().unwrap().node(krunt_ir::IrNodeId(1)).unwrap().op,
			IROp::ENTRY { .. }
		));
	}

	#[test]
	fn compiles_query_persistence_effect_into_canonical_ir() {
		let mut registry = PluginRegistry::new();
		BackendDslPlugin::register(&mut registry).unwrap();

		let result = compile(&persistence_document(), &registry);

		assert!(!result.has_errors(), "{:#?}", result.diagnostics());
		let query = result
			.ir()
			.unwrap()
			.nodes()
			.iter()
			.find(
				|node| matches!(&node.op, IROp::OPERATION { name } if name.as_str() == QUERY_NODE_TYPE),
			)
			.expect("query should be lowered into IR");
		assert_eq!(query.async_kind, AsyncKind::ASYNC);
		assert_eq!(query.output_types.len(), 1);
		assert_eq!(query.output_types[0].as_str(), "Array<User>");
		assert_eq!(query.attributes["backend.persistence.entity"], "User");
		assert_eq!(
			query.attributes["backend.persistence.output_type"],
			"Array<User>"
		);
		assert_eq!(
			query.effects,
			vec![EffectId::new(DATABASE_READ_EFFECT).unwrap()]
		);
		assert_eq!(query.attributes["backend.persistence.kind"], "QUERY");
		let response = result
			.ir()
			.unwrap()
			.nodes()
			.iter()
			.find(|node| node.returns)
			.expect("response should be lowered as a return operation");
		assert_eq!(response.input_types[0].as_str(), "Array<User>");
		assert_eq!(response.attributes["response.body_type"], "Array<User>");
		assert!(result.ir().unwrap().nodes().iter().any(|node| {
			matches!(&node.op, IROp::DECLARATION { name } if name.as_str() == ENTITY_NODE_TYPE)
		}));
		let entity = result
			.ir()
			.unwrap()
			.nodes()
			.iter()
			.find(
				|node| matches!(&node.op, IROp::DECLARATION { name } if name.as_str() == ENTITY_NODE_TYPE),
			)
			.expect("entity should be lowered into IR");
		assert_eq!(
			entity.attributes["backend.entity.fields"],
			"{\"email\":{\"type\":\"String\",\"required\":false},\"id\":{\"type\":\"Integer\",\"required\":true},\"profile\":{\"type\":{\"kind\":\"record\",\"fields\":{\"bio\":{\"type\":\"String\",\"required\":false},\"displayName\":{\"type\":\"String\",\"required\":true}}},\"required\":false},\"tags\":{\"type\":{\"kind\":\"array\",\"items\":\"String\"},\"required\":true}}"
		);
		let route = result
			.ir()
			.unwrap()
			.nodes()
			.iter()
			.find(|node| matches!(&node.op, IROp::ENTRY { .. }))
			.expect("route should be lowered into IR");
		assert_eq!(route.attributes["backend.http.request_entity"], "User");
		assert!(result.ir().unwrap().nodes().iter().any(|node| {
			matches!(&node.op, IROp::DECLARATION { name } if name.as_str() == REPOSITORY_NODE_TYPE)
		}));
	}

	#[test]
	fn compiles_effect_nodes_and_preserves_their_effect_metadata() {
		let mut registry = PluginRegistry::new();
		BackendDslPlugin::register(&mut registry).unwrap();

		let result = compile(&effects_document(), &registry);

		assert!(!result.has_errors(), "{:#?}", result.diagnostics());
		let graph = result.ir().unwrap();
		let http_request = graph
			.nodes()
			.iter()
			.find(
				|node| matches!(&node.op, IROp::OPERATION { name } if name.as_str() == HTTP_REQUEST_NODE_TYPE),
			)
			.expect("HTTP Request should be lowered into IR");
		assert_eq!(http_request.async_kind, AsyncKind::ASYNC);
		assert_eq!(
			http_request.effects,
			vec![EffectId::new(HTTP_REQUEST_EFFECT).unwrap()]
		);

		let storage = graph
			.nodes()
			.iter()
			.find(
				|node| matches!(&node.op, IROp::OPERATION { name } if name.as_str() == STORAGE_NODE_TYPE),
			)
			.expect("Storage should be lowered into IR");
		assert_eq!(storage.async_kind, AsyncKind::ASYNC);
		assert_eq!(
			storage.effects,
			vec![EffectId::new(STORAGE_READ_EFFECT).unwrap()]
		);

		let logging = graph
			.nodes()
			.iter()
			.find(
				|node| matches!(&node.op, IROp::OPERATION { name } if name.as_str() == LOGGING_NODE_TYPE),
			)
			.expect("Logging should be lowered into IR");
		assert_eq!(logging.async_kind, AsyncKind::SYNC);
		assert_eq!(
			logging.effects,
			vec![EffectId::new(LOGGING_EFFECT).unwrap()]
		);
		assert_eq!(graph.control_edges().len(), 4);
		assert_eq!(graph.data_edges().len(), 1);
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
		let diagnostic = result
			.diagnostics()
			.as_slice()
			.iter()
			.find(|diagnostic| diagnostic.code().as_str() == "TYPE.MISMATCH")
			.expect("the incompatible edge should have a type mismatch diagnostic");
		assert_eq!(diagnostic.code().as_str(), "TYPE.MISMATCH");
		assert_eq!(diagnostic.stage(), CompilerStage::TYPE_ANALYSIS);
		assert_eq!(
			diagnostic.primary(),
			GraphElementId::PORT(PortId::new(22).unwrap())
		);
		assert_eq!(diagnostic.related().len(), 2);
	}

	#[test]
	fn contract_types_seed_inference_for_open_ports() {
		let document = document_with_open_data_types();
		let registry = registry();
		let mut diagnostics = Diagnostics::new();
		let inferred = infer_data_types(
			&document,
			&contracts_for(&document, &registry),
			&mut diagnostics,
		);

		assert!(diagnostics.is_empty());
		assert_eq!(
			inferred.get(&PortId::new(12).unwrap()),
			Some(&Type::named("Request").unwrap())
		);
		assert_eq!(
			inferred.get(&PortId::new(22).unwrap()),
			Some(&Type::named("Request").unwrap())
		);
	}

	#[test]
	fn contract_type_mismatches_block_ir_generation_with_source_links() {
		let document = document_with_route_type(Some("Response"), Some("ANY"), true);
		let result = compile(&document, &registry());

		assert!(result.has_errors());
		assert!(result.ir().is_none());
		let diagnostic = result
			.diagnostics()
			.as_slice()
			.iter()
			.find(|diagnostic| diagnostic.code().as_str() == "TYPE.CONTRACT_MISMATCH")
			.expect("the contract mismatch should be reported");
		assert_eq!(
			diagnostic.primary(),
			GraphElementId::PORT(PortId::new(12).unwrap())
		);
		assert_eq!(diagnostic.related().len(), 1);
		assert_eq!(
			diagnostic.related()[0].element(),
			GraphElementId::NODE(NodeId::new(1).unwrap())
		);
	}

	#[test]
	fn open_data_ports_are_inferred_from_connected_concrete_ports() {
		let document = document_with_response_type(Some("ANY"));
		let registry = registry();
		let mut diagnostics = Diagnostics::new();
		let inferred = infer_data_types(
			&document,
			&contracts_for(&document, &registry),
			&mut diagnostics,
		);

		assert!(diagnostics.is_empty());
		assert_eq!(
			inferred.get(&PortId::new(12).unwrap()),
			Some(&Type::named("Request").unwrap())
		);
		assert_eq!(
			inferred.get(&PortId::new(22).unwrap()),
			Some(&Type::named("Request").unwrap())
		);
	}

	#[test]
	fn inferred_types_specialize_open_lowered_operation_slots() {
		let document = document_with_response_type(Some("ANY"));
		let registry = registry();
		let mut diagnostics = Diagnostics::new();
		let inferred = infer_data_types(
			&document,
			&contracts_for(&document, &registry),
			&mut diagnostics,
		);
		let response = &document.nodes()[1];
		let lowered = LoweredOperation::new(
			krunt_ir::OperationId::new("test.response").unwrap(),
			[TypeId::new("ANY").unwrap()],
			[],
			AsyncKind::SYNC,
			[],
		);

		let specialized = specialize_lowered_operation(response, lowered, &inferred);

		assert_eq!(specialized.input_types()[0].as_str(), "Request");
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
