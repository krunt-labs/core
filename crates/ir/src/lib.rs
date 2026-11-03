//! Canonical, validated, deterministic IR for Krunt.
//!
//! This crate deliberately knows nothing about HTTP, NestJS, TypeScript, or
//! any other target. It represents the result of semantic lowering in a form
//! that multiple generators can consume.

use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt;

/// Stable identifier of a graph node in the user's source document.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct GraphNodeId(pub u64);

/// Stable identifier of a graph port in the user's source document.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct GraphPortId(pub u64);

/// Stable identifier of a graph edge in the user's source document.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct GraphEdgeId(pub u64);

/// Stable identifier of an IR node.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct IrNodeId(pub u32);

/// A source element that can explain an IR construct or diagnostic.
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", content = "id")]
pub enum GraphElementId {
	NODE(GraphNodeId),
	PORT(GraphPortId),
	EDGE(GraphEdgeId),
}

/// A resolved type identity supplied by the type system.
///
/// The IR does not infer or interpret types. It only refuses an empty type
/// identity, because an executable IR must not contain an unresolved type.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct TypeId(String);

impl TypeId {
	pub fn new(value: impl Into<String>) -> Result<Self, BuildError> {
		let value = value.into();
		if value.trim().is_empty() {
			return Err(BuildError::EmptyTypeId);
		}
		Ok(Self(value))
	}

	pub fn as_str(&self) -> &str {
		&self.0
	}
}

/// A stable semantic operation name supplied by a plugin during lowering.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct OperationId(String);

impl OperationId {
	pub fn new(value: impl Into<String>) -> Result<Self, BuildError> {
		let value = value.into();
		if value.trim().is_empty() {
			return Err(BuildError::EmptyOperationId);
		}
		Ok(Self(value))
	}

	pub fn as_str(&self) -> &str {
		&self.0
	}
}

/// Mapping from one IR node back to source graph elements.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SourceMap {
	pub primary: GraphElementId,
	pub related: Vec<GraphElementId>,
}

impl SourceMap {
	pub fn node(node_id: GraphNodeId) -> Self {
		Self {
			primary: GraphElementId::NODE(node_id),
			related: Vec::new(),
		}
	}

	pub fn with_related(mut self, related: impl IntoIterator<Item = GraphElementId>) -> Self {
		self.related.extend(related);
		self.related.sort();
		self.related.dedup();
		self.related.retain(|id| *id != self.primary);
		self
	}
}

/// Whether an operation is synchronous or produces an asynchronous value.
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum AsyncKind {
	SYNC,
	ASYNC,
}

/// A resolved observable effect of an operation.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct EffectId(String);

impl EffectId {
	pub fn new(value: impl Into<String>) -> Result<Self, BuildError> {
		let value = value.into();
		if value.trim().is_empty() {
			return Err(BuildError::EmptyEffectId);
		}
		Ok(Self(value))
	}
}

/// Domain-neutral operation kinds for the initial canonical IR.
#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub enum IROp {
	ENTRY { name: OperationId },
	OPERATION { name: OperationId },
	BRANCH,
	MERGE,
	LOOP,
	RETURN,
}

/// A reference to one input or output slot on an IR node.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct PortRef {
	pub node: IrNodeId,
	pub index: u16,
}

/// An immutable operation in the canonical IR graph.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct IRNode {
	pub id: IrNodeId,
	pub op: IROp,
	pub input_types: Vec<TypeId>,
	pub output_types: Vec<TypeId>,
	pub async_kind: AsyncKind,
	pub effects: Vec<EffectId>,
	pub source: SourceMap,
}

/// A control-flow relationship between two IR operations.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ControlEdge {
	pub from: IrNodeId,
	pub to: IrNodeId,
}

/// A typed value relationship between an output and an input slot.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct DataEdge {
	pub from: PortRef,
	pub to: PortRef,
}

/// The validated immutable IR consumed by compiler passes and generators.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct IRGraph {
	version: u32,
	nodes: Vec<IRNode>,
	control_edges: Vec<ControlEdge>,
	data_edges: Vec<DataEdge>,
}

impl IRGraph {
	pub const CURRENT_VERSION: u32 = 1;

	pub fn node(&self, id: IrNodeId) -> Option<&IRNode> {
		self.nodes
			.binary_search_by_key(&id, |node| node.id)
			.ok()
			.map(|i| &self.nodes[i])
	}

	pub fn version(&self) -> u32 {
		self.version
	}

	pub fn nodes(&self) -> &[IRNode] {
		&self.nodes
	}

	pub fn control_edges(&self) -> &[ControlEdge] {
		&self.control_edges
	}

	pub fn data_edges(&self) -> &[DataEdge] {
		&self.data_edges
	}
}

/// Errors raised while constructing an IR graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BuildError {
	DuplicateNode(IrNodeId),
	MissingNode(IrNodeId),
	DuplicateControlEdge(ControlEdge),
	DuplicateDataEdge(DataEdge),
	InvalidOutputPort(PortRef),
	InvalidInputPort(PortRef),
	TypeMismatch {
		from: TypeId,
		to: TypeId,
		edge: DataEdge,
	},
	EmptyTypeId,
	EmptyOperationId,
	EmptyEffectId,
	MissingSourceMap(IrNodeId),
}

impl fmt::Display for BuildError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{self:?}")
	}
}

impl std::error::Error for BuildError {}

/// Mutable validation boundary. `build` consumes it and returns an immutable
/// `IRGraph`, so later compiler passes cannot mutate a graph behind one
/// another's back.
#[derive(Default)]
pub struct IRBuilder {
	nodes: BTreeMap<IrNodeId, IRNode>,
	control_edges: Vec<ControlEdge>,
	data_edges: Vec<DataEdge>,
}

impl IRBuilder {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn add_node(&mut self, node: IRNode) -> Result<(), BuildError> {
		if self.nodes.contains_key(&node.id) {
			return Err(BuildError::DuplicateNode(node.id));
		}
		self.nodes.insert(node.id, node);
		Ok(())
	}

	pub fn add_control_edge(&mut self, edge: ControlEdge) -> Result<(), BuildError> {
		if self.control_edges.contains(&edge) {
			return Err(BuildError::DuplicateControlEdge(edge));
		}
		self.control_edges.push(edge);
		Ok(())
	}

	pub fn add_data_edge(&mut self, edge: DataEdge) -> Result<(), BuildError> {
		if self.data_edges.contains(&edge) {
			return Err(BuildError::DuplicateDataEdge(edge));
		}
		self.data_edges.push(edge);
		Ok(())
	}

	pub fn build(mut self) -> Result<IRGraph, BuildError> {
		for edge in &self.control_edges {
			self.require_node(edge.from)?;
			self.require_node(edge.to)?;
		}

		for edge in &self.data_edges {
			let from = self.require_node(edge.from.node)?;
			let to = self.require_node(edge.to.node)?;
			let from_type = from
				.output_types
				.get(usize::from(edge.from.index))
				.ok_or(BuildError::InvalidOutputPort(edge.from))?;
			let to_type = to
				.input_types
				.get(usize::from(edge.to.index))
				.ok_or(BuildError::InvalidInputPort(edge.to))?;
			if from_type != to_type {
				return Err(BuildError::TypeMismatch {
					from: from_type.clone(),
					to: to_type.clone(),
					edge: *edge,
				});
			}
		}

		self.control_edges.sort();
		self.data_edges.sort();

		Ok(IRGraph {
			version: IRGraph::CURRENT_VERSION,
			nodes: self.nodes.into_values().collect(),
			control_edges: self.control_edges,
			data_edges: self.data_edges,
		})
	}

	fn require_node(&self, id: IrNodeId) -> Result<&IRNode, BuildError> {
		self.nodes.get(&id).ok_or(BuildError::MissingNode(id))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn operation(name: &str, id: u32, source: u64, inputs: &[&str], outputs: &[&str]) -> IRNode {
		IRNode {
			id: IrNodeId(id),
			op: IROp::OPERATION {
				name: OperationId::new(name).unwrap(),
			},
			input_types: inputs
				.iter()
				.map(|value| TypeId::new(*value).unwrap())
				.collect(),
			output_types: outputs
				.iter()
				.map(|value| TypeId::new(*value).unwrap())
				.collect(),
			async_kind: AsyncKind::SYNC,
			effects: Vec::new(),
			source: SourceMap::node(GraphNodeId(source)),
		}
	}

	#[test]
	fn builds_a_typed_control_and_data_graph() {
		let mut builder = IRBuilder::new();
		builder
			.add_node(operation("route", 1, 10, &[], &["Request"]))
			.unwrap();
		builder
			.add_node(operation("query", 2, 20, &["Request"], &["User"]))
			.unwrap();
		builder
			.add_node(operation("response", 3, 30, &["User"], &[]))
			.unwrap();
		builder
			.add_control_edge(ControlEdge {
				from: IrNodeId(1),
				to: IrNodeId(2),
			})
			.unwrap();
		builder
			.add_control_edge(ControlEdge {
				from: IrNodeId(2),
				to: IrNodeId(3),
			})
			.unwrap();
		builder
			.add_data_edge(DataEdge {
				from: PortRef {
					node: IrNodeId(1),
					index: 0,
				},
				to: PortRef {
					node: IrNodeId(2),
					index: 0,
				},
			})
			.unwrap();
		builder
			.add_data_edge(DataEdge {
				from: PortRef {
					node: IrNodeId(2),
					index: 0,
				},
				to: PortRef {
					node: IrNodeId(3),
					index: 0,
				},
			})
			.unwrap();

		let graph = builder.build().unwrap();
		assert_eq!(graph.nodes().len(), 3);
		assert_eq!(graph.control_edges().len(), 2);
		assert_eq!(graph.data_edges().len(), 2);
		assert_eq!(
			graph.node(IrNodeId(2)).unwrap().source.primary,
			GraphElementId::NODE(GraphNodeId(20))
		);
	}

	#[test]
	fn rejects_dangling_control_edges() {
		let mut builder = IRBuilder::new();
		builder
			.add_node(operation("route", 1, 10, &[], &[]))
			.unwrap();
		builder
			.add_control_edge(ControlEdge {
				from: IrNodeId(1),
				to: IrNodeId(99),
			})
			.unwrap();

		assert_eq!(builder.build(), Err(BuildError::MissingNode(IrNodeId(99))));
	}

	#[test]
	fn rejects_duplicate_nodes_without_replacing_the_original() {
		let mut builder = IRBuilder::new();
		builder
			.add_node(operation("first", 1, 10, &[], &[]))
			.unwrap();

		assert_eq!(
			builder.add_node(operation("second", 1, 20, &[], &[])),
			Err(BuildError::DuplicateNode(IrNodeId(1)))
		);

		let graph = builder.build().unwrap();
		assert_eq!(
			graph.node(IrNodeId(1)).unwrap().source.primary,
			GraphElementId::NODE(GraphNodeId(10))
		);
	}

	#[test]
	fn rejects_invalid_data_ports() {
		let mut builder = IRBuilder::new();
		builder
			.add_node(operation("route", 1, 10, &[], &["Request"]))
			.unwrap();
		builder
			.add_node(operation("response", 2, 20, &[], &[]))
			.unwrap();
		builder
			.add_data_edge(DataEdge {
				from: PortRef {
					node: IrNodeId(1),
					index: 1,
				},
				to: PortRef {
					node: IrNodeId(2),
					index: 0,
				},
			})
			.unwrap();

		assert_eq!(
			builder.build(),
			Err(BuildError::InvalidOutputPort(PortRef {
				node: IrNodeId(1),
				index: 1
			}))
		);
	}

	#[test]
	fn rejects_type_mismatched_data_edges() {
		let mut builder = IRBuilder::new();
		builder
			.add_node(operation("route", 1, 10, &[], &["Request"]))
			.unwrap();
		builder
			.add_node(operation("response", 2, 20, &["User"], &[]))
			.unwrap();
		builder
			.add_data_edge(DataEdge {
				from: PortRef {
					node: IrNodeId(1),
					index: 0,
				},
				to: PortRef {
					node: IrNodeId(2),
					index: 0,
				},
			})
			.unwrap();

		assert!(matches!(
			builder.build(),
			Err(BuildError::TypeMismatch { .. })
		));
	}

	#[test]
	fn output_order_is_deterministic_even_when_nodes_are_inserted_differently() {
		let mut first = IRBuilder::new();
		first
			.add_node(operation("response", 3, 30, &[], &[]))
			.unwrap();
		first.add_node(operation("route", 1, 10, &[], &[])).unwrap();
		first.add_node(operation("query", 2, 20, &[], &[])).unwrap();

		let mut second = IRBuilder::new();
		second
			.add_node(operation("query", 2, 20, &[], &[]))
			.unwrap();
		second
			.add_node(operation("route", 1, 10, &[], &[]))
			.unwrap();
		second
			.add_node(operation("response", 3, 30, &[], &[]))
			.unwrap();

		let first = first.build().unwrap();
		let second = second.build().unwrap();
		assert_eq!(first, second);
		assert_eq!(
			serde_json::to_string(&first).unwrap(),
			serde_json::to_string(&second).unwrap()
		);
	}

	#[test]
	fn source_map_related_ids_are_sorted_and_deduplicated() {
		let source = SourceMap::node(GraphNodeId(10)).with_related([
			GraphElementId::EDGE(GraphEdgeId(9)),
			GraphElementId::PORT(GraphPortId(3)),
			GraphElementId::EDGE(GraphEdgeId(9)),
		]);

		assert_eq!(source.related.len(), 2);
		assert!(source.related[0] < source.related[1]);
	}

	#[test]
	fn source_map_does_not_repeat_its_primary_id() {
		let source = SourceMap::node(GraphNodeId(10)).with_related([
			GraphElementId::NODE(GraphNodeId(10)),
			GraphElementId::PORT(GraphPortId(3)),
		]);

		assert_eq!(source.related, vec![GraphElementId::PORT(GraphPortId(3))]);
	}
}
