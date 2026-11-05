//! TypeScript language IR for Krunt.
//!
//! This crate translates universal IR into a deterministic TypeScript-shaped
//! representation. It does not generate NestJS decorators, choose project
//! filenames beyond its language module boundary, or render source text yet.
//! Those concerns belong to the framework backend and source renderer layers.

use krunt_ir::{AsyncKind, EffectId, GraphElementId, IRGraph, IROp, SourceMap};
use krunt_language_backend::LanguageBackend;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// Version of the serialized TypeScript language IR schema.
pub const TYPESCRIPT_IR_VERSION: u32 = 1;

/// A TypeScript language program containing deterministic modules.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TypeScriptProgram {
	version: u32,
	modules: Vec<TypeScriptModule>,
}

impl TypeScriptProgram {
	pub const CURRENT_VERSION: u32 = TYPESCRIPT_IR_VERSION;

	pub fn version(&self) -> u32 {
		self.version
	}

	pub fn modules(&self) -> &[TypeScriptModule] {
		&self.modules
	}
}

/// A TypeScript module in the language IR.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TypeScriptModule {
	path: String,
	imports: Vec<TypeScriptImport>,
	functions: Vec<TypeScriptFunction>,
}

impl TypeScriptModule {
	pub fn path(&self) -> &str {
		&self.path
	}

	pub fn imports(&self) -> &[TypeScriptImport] {
		&self.imports
	}

	pub fn functions(&self) -> &[TypeScriptFunction] {
		&self.functions
	}
}

/// A future-ready import declaration owned by the TypeScript language layer.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TypeScriptImport {
	module: String,
	names: Vec<String>,
}

impl TypeScriptImport {
	pub fn module(&self) -> &str {
		&self.module
	}

	pub fn names(&self) -> &[String] {
		&self.names
	}
}

/// A deterministic executable function assembled from universal IR nodes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TypeScriptFunction {
	name: String,
	async_kind: AsyncKind,
	operations: Vec<TypeScriptOperation>,
	control_edges: Vec<TypeScriptControlEdge>,
	data_edges: Vec<TypeScriptDataEdge>,
}

impl TypeScriptFunction {
	pub fn name(&self) -> &str {
		&self.name
	}

	pub fn async_kind(&self) -> AsyncKind {
		self.async_kind
	}

	pub fn operations(&self) -> &[TypeScriptOperation] {
		&self.operations
	}

	pub fn control_edges(&self) -> &[TypeScriptControlEdge] {
		&self.control_edges
	}

	pub fn data_edges(&self) -> &[TypeScriptDataEdge] {
		&self.data_edges
	}
}

/// A language-level operation that still retains universal source identity.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TypeScriptOperation {
	id: TypeScriptOperationId,
	kind: TypeScriptOperationKind,
	input_types: Vec<TypeScriptType>,
	output_types: Vec<TypeScriptType>,
	async_kind: AsyncKind,
	effects: Vec<EffectId>,
	source: SourceMap,
}

impl TypeScriptOperation {
	pub fn id(&self) -> TypeScriptOperationId {
		self.id
	}

	pub fn kind(&self) -> &TypeScriptOperationKind {
		&self.kind
	}

	pub fn input_types(&self) -> &[TypeScriptType] {
		&self.input_types
	}

	pub fn output_types(&self) -> &[TypeScriptType] {
		&self.output_types
	}

	pub fn async_kind(&self) -> AsyncKind {
		self.async_kind
	}

	pub fn effects(&self) -> &[EffectId] {
		&self.effects
	}

	pub fn source(&self) -> &SourceMap {
		&self.source
	}
}

/// Stable operation identity within the TypeScript language IR.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct TypeScriptOperationId(u32);

impl TypeScriptOperationId {
	pub const fn new(value: u32) -> Self {
		Self(value)
	}

	pub const fn as_u32(self) -> u32 {
		self.0
	}
}

/// TypeScript representation of a universal IR operation kind.
#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub enum TypeScriptOperationKind {
	ENTRY { name: String },
	OPERATION { name: String },
	BRANCH,
	MERGE,
	LOOP,
	RETURN,
}

/// A TypeScript type identity produced from universal IR type identity.
#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub enum TypeScriptType {
	ANY,
	NAMED { name: String },
}

/// A control-flow edge between TypeScript operations.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct TypeScriptControlEdge {
	from: TypeScriptOperationId,
	to: TypeScriptOperationId,
}

impl TypeScriptControlEdge {
	pub fn from(&self) -> TypeScriptOperationId {
		self.from
	}

	pub fn to(&self) -> TypeScriptOperationId {
		self.to
	}
}

/// A typed data edge between TypeScript operation ports.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct TypeScriptDataEdge {
	from: TypeScriptPortRef,
	to: TypeScriptPortRef,
}

impl TypeScriptDataEdge {
	pub fn from(&self) -> TypeScriptPortRef {
		self.from
	}

	pub fn to(&self) -> TypeScriptPortRef {
		self.to
	}
}

/// A reference to an operation port in TypeScript IR.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct TypeScriptPortRef {
	node: TypeScriptOperationId,
	index: u16,
}

impl TypeScriptPortRef {
	pub const fn new(node: TypeScriptOperationId, index: u16) -> Self {
		Self { node, index }
	}

	pub fn node(&self) -> TypeScriptOperationId {
		self.node
	}

	pub fn index(&self) -> u16 {
		self.index
	}
}

/// Errors raised while lowering universal IR to TypeScript IR.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TypeScriptLoweringError {
	EmptyOperationName { node_id: TypeScriptOperationId },
}

impl fmt::Display for TypeScriptLoweringError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::EmptyOperationName { node_id } => {
				write!(
					f,
					"TypeScript operation {} has an empty name.",
					node_id.as_u32()
				)
			}
		}
	}
}

impl std::error::Error for TypeScriptLoweringError {}

/// The first language backend implementation for Krunt.
#[derive(Clone, Copy, Debug, Default)]
pub struct TypeScriptBackend;

impl TypeScriptBackend {
	pub const BACKEND_ID: &'static str = "typescript";

	pub fn lower_graph(graph: &IRGraph) -> Result<TypeScriptProgram, TypeScriptLoweringError> {
		<Self as LanguageBackend>::lower(&Self, graph)
	}
}

impl LanguageBackend for TypeScriptBackend {
	type Output = TypeScriptProgram;
	type Error = TypeScriptLoweringError;

	fn backend_id(&self) -> &'static str {
		Self::BACKEND_ID
	}

	fn lower(&self, graph: &IRGraph) -> Result<Self::Output, Self::Error> {
		let operations = graph
			.nodes()
			.iter()
			.map(lower_operation)
			.collect::<Result<Vec<_>, _>>()?;
		let async_kind = if operations
			.iter()
			.any(|operation| operation.async_kind == AsyncKind::ASYNC)
		{
			AsyncKind::ASYNC
		} else {
			AsyncKind::SYNC
		};
		let function = TypeScriptFunction {
			name: "execute".to_string(),
			async_kind,
			operations,
			control_edges: graph
				.control_edges()
				.iter()
				.map(|edge| TypeScriptControlEdge {
					from: TypeScriptOperationId::new(edge.from.0),
					to: TypeScriptOperationId::new(edge.to.0),
				})
				.collect(),
			data_edges: graph
				.data_edges()
				.iter()
				.map(|edge| TypeScriptDataEdge {
					from: TypeScriptPortRef::new(
						TypeScriptOperationId::new(edge.from.node.0),
						edge.from.index,
					),
					to: TypeScriptPortRef::new(
						TypeScriptOperationId::new(edge.to.node.0),
						edge.to.index,
					),
				})
				.collect(),
		};

		Ok(TypeScriptProgram {
			version: TypeScriptProgram::CURRENT_VERSION,
			modules: vec![TypeScriptModule {
				path: "generated.ts".to_string(),
				imports: Vec::new(),
				functions: vec![function],
			}],
		})
	}
}

/// Errors raised while rendering TypeScript language IR into source files.
#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TypeScriptRenderError {
	EMPTY_MODULE_PATH,
	ABSOLUTE_MODULE_PATH { path: String },
	PATH_TRAVERSAL { path: String },
	BACKSLASH_MODULE_PATH { path: String },
	DUPLICATE_MODULE_PATH { path: String },
}

impl fmt::Display for TypeScriptRenderError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::EMPTY_MODULE_PATH => write!(f, "TypeScript module paths cannot be empty."),
			Self::ABSOLUTE_MODULE_PATH { path } => {
				write!(f, "TypeScript module path is absolute: {path}")
			}
			Self::PATH_TRAVERSAL { path } => {
				write!(f, "TypeScript module path escapes its root: {path}")
			}
			Self::BACKSLASH_MODULE_PATH { path } => {
				write!(f, "TypeScript module paths must use '/': {path}")
			}
			Self::DUPLICATE_MODULE_PATH { path } => {
				write!(
					f,
					"TypeScript program contains duplicate module path: {path}"
				)
			}
		}
	}
}

impl std::error::Error for TypeScriptRenderError {}

/// Deterministic source renderer for the TypeScript language IR.
#[derive(Clone, Copy, Debug, Default)]
pub struct TypeScriptSourceRenderer;

impl TypeScriptSourceRenderer {
	/// Render each language module to a normalized relative `.ts` file.
	pub fn render_program(
		program: &TypeScriptProgram,
	) -> Result<BTreeMap<String, String>, TypeScriptRenderError> {
		let mut files = BTreeMap::new();
		for module in program.modules() {
			validate_module_path(module.path())?;
			if files
				.insert(module.path().to_string(), render_module(module))
				.is_some()
			{
				return Err(TypeScriptRenderError::DUPLICATE_MODULE_PATH {
					path: module.path().to_string(),
				});
			}
		}
		Ok(files)
	}
}

fn render_module(module: &TypeScriptModule) -> String {
	let mut source = String::new();
	for import in module.imports() {
		let names = import
			.names()
			.iter()
			.map(|name| sanitize_identifier(name, "imported_value"))
			.collect::<BTreeSet<_>>()
			.into_iter()
			.collect::<Vec<_>>();
		if names.is_empty() {
			source.push_str(&format!("import \"{}\";\n", import.module()));
		} else {
			source.push_str(&format!(
				"import {{ {} }} from \"{}\";\n",
				names.join(", "),
				import.module()
			));
		}
	}
	if !module.imports().is_empty() {
		source.push('\n');
	}

	for function in module.functions() {
		source.push_str(&render_function(function));
		source.push('\n');
	}
	source
}

fn render_function(function: &TypeScriptFunction) -> String {
	let mut source = String::new();
	let declarations = operation_declarations(function);
	for declaration in declarations.values() {
		source.push_str("declare function ");
		source.push_str(declaration);
		source.push_str("(...args: unknown[]): unknown;\n");
	}
	if !declarations.is_empty() {
		source.push('\n');
	}

	let function_name = sanitize_identifier(&function.name, "execute");
	let async_prefix = if function.async_kind == AsyncKind::ASYNC {
		"async "
	} else {
		""
	};
	let return_type = if function.async_kind == AsyncKind::ASYNC {
		"Promise<unknown>"
	} else {
		"unknown"
	};
	source.push_str(&format!(
		"export {}function {}(): {} {{\n",
		async_prefix, function_name, return_type
	));

	let data_inputs = data_inputs(function);
	for operation in &function.operations {
		source.push_str("  // krunt:source=");
		source.push_str(&source_marker(operation.source()));
		source.push('\n');
		source.push_str("  // krunt:operation=");
		source.push_str(&operation.id.as_u32().to_string());
		source.push(' ');
		source.push_str(&render_operation_kind(operation.kind()));
		source.push('\n');
		if !operation.input_types.is_empty() || !operation.output_types.is_empty() {
			source.push_str("  // krunt:types=");
			source.push_str(&render_types(&operation.input_types));
			source.push_str(" -> ");
			source.push_str(&render_types(&operation.output_types));
			source.push('\n');
		}

		match operation.kind() {
			TypeScriptOperationKind::ENTRY { name }
			| TypeScriptOperationKind::OPERATION { name } => {
				render_named_operation(&mut source, operation, name, &data_inputs);
			}
			TypeScriptOperationKind::BRANCH
			| TypeScriptOperationKind::MERGE
			| TypeScriptOperationKind::LOOP => {
				source.push_str("  const ");
				source.push_str(&operation_variable(operation.id));
				source.push_str(": unknown = undefined;\n");
			}
			TypeScriptOperationKind::RETURN => {
				if let Some(value) = first_data_input(operation.id, &data_inputs) {
					source.push_str("  return ");
					source.push_str(&operation_variable(value));
					source.push_str(";\n");
				} else {
					source.push_str("  return;\n");
				}
			}
		}
	}
	source.push_str("}\n");
	source
}

fn operation_declarations(function: &TypeScriptFunction) -> BTreeMap<String, String> {
	function
		.operations
		.iter()
		.filter_map(|operation| match operation.kind() {
			TypeScriptOperationKind::ENTRY { name }
			| TypeScriptOperationKind::OPERATION { name } => Some((
				operation.id.as_u32().to_string(),
				operation_callee(operation.id, name),
			)),
			_ => None,
		})
		.collect()
}

fn render_named_operation(
	source: &mut String,
	operation: &TypeScriptOperation,
	name: &str,
	data_inputs: &BTreeMap<TypeScriptOperationId, Vec<Option<TypeScriptOperationId>>>,
) {
	let callee = operation_callee(operation.id, name);
	let arguments = data_inputs
		.get(&operation.id)
		.map(|inputs| {
			inputs
				.iter()
				.map(|input| {
					input
						.map(operation_variable)
						.unwrap_or_else(|| "undefined".to_string())
				})
				.collect::<Vec<_>>()
				.join(", ")
		})
		.unwrap_or_default();
	let await_prefix = if operation.async_kind == AsyncKind::ASYNC {
		"await "
	} else {
		""
	};
	let invocation = format!("{await_prefix}{callee}({arguments})");
	if operation.output_types.is_empty() {
		source.push_str("  ");
		source.push_str(&invocation);
		source.push_str(";\n");
	} else {
		source.push_str("  const ");
		source.push_str(&operation_variable(operation.id));
		source.push_str(" = ");
		source.push_str(&invocation);
		source.push_str(";\n");
	}
}

fn data_inputs(
	function: &TypeScriptFunction,
) -> BTreeMap<TypeScriptOperationId, Vec<Option<TypeScriptOperationId>>> {
	let mut inputs: BTreeMap<_, BTreeMap<_, _>> = BTreeMap::new();
	for edge in &function.data_edges {
		inputs
			.entry(edge.to.node)
			.or_default()
			.insert(edge.to.index, edge.from.node);
	}
	inputs
		.into_iter()
		.map(|(node, values)| {
			let Some(max_index) = values.keys().max().copied() else {
				return (node, Vec::new());
			};
			let mut ordered = vec![None; usize::from(max_index) + 1];
			for (index, value) in values {
				ordered[usize::from(index)] = Some(value);
			}
			(node, ordered)
		})
		.collect()
}

fn first_data_input(
	operation: TypeScriptOperationId,
	inputs: &BTreeMap<TypeScriptOperationId, Vec<Option<TypeScriptOperationId>>>,
) -> Option<TypeScriptOperationId> {
	inputs
		.get(&operation)
		.and_then(|values| values.iter().flatten().next().copied())
}

fn operation_variable(id: TypeScriptOperationId) -> String {
	format!("operation_{}", id.as_u32())
}

fn operation_callee(id: TypeScriptOperationId, name: &str) -> String {
	format!(
		"krunt_{}_{}",
		sanitize_identifier(name, "operation"),
		id.as_u32()
	)
}

fn render_operation_kind(kind: &TypeScriptOperationKind) -> String {
	match kind {
		TypeScriptOperationKind::ENTRY { name } => format!("ENTRY {name}"),
		TypeScriptOperationKind::OPERATION { name } => format!("OPERATION {name}"),
		TypeScriptOperationKind::BRANCH => "BRANCH".to_string(),
		TypeScriptOperationKind::MERGE => "MERGE".to_string(),
		TypeScriptOperationKind::LOOP => "LOOP".to_string(),
		TypeScriptOperationKind::RETURN => "RETURN".to_string(),
	}
}

fn render_types(types: &[TypeScriptType]) -> String {
	types
		.iter()
		.map(|type_id| match type_id {
			TypeScriptType::ANY => "any".to_string(),
			TypeScriptType::NAMED { name } => name.clone(),
		})
		.collect::<Vec<_>>()
		.join(", ")
}

fn source_marker(source: &SourceMap) -> String {
	let primary = graph_element_marker(&source.primary);
	if source.related.is_empty() {
		return primary;
	}
	let related = source
		.related
		.iter()
		.map(graph_element_marker)
		.collect::<Vec<_>>()
		.join(",");
	format!("{primary};related={related}")
}

fn graph_element_marker(element: &GraphElementId) -> String {
	match element {
		GraphElementId::NODE(id) => format!("node:{}", id.0),
		GraphElementId::PORT(id) => format!("port:{}", id.0),
		GraphElementId::EDGE(id) => format!("edge:{}", id.0),
	}
}

fn sanitize_identifier(value: &str, fallback: &str) -> String {
	let mut identifier = value
		.chars()
		.map(|character| {
			if character.is_ascii_alphanumeric() || character == '_' || character == '$' {
				character
			} else {
				'_'
			}
		})
		.collect::<String>();
	if identifier.is_empty() {
		identifier = fallback.to_string();
	}
	if identifier
		.chars()
		.next()
		.is_some_and(|character| character.is_ascii_digit())
	{
		identifier.insert(0, '_');
	}
	identifier
}

fn validate_module_path(path: &str) -> Result<(), TypeScriptRenderError> {
	if path.is_empty() {
		return Err(TypeScriptRenderError::EMPTY_MODULE_PATH);
	}
	if path.starts_with('/') {
		return Err(TypeScriptRenderError::ABSOLUTE_MODULE_PATH {
			path: path.to_string(),
		});
	}
	if path.contains('\\') {
		return Err(TypeScriptRenderError::BACKSLASH_MODULE_PATH {
			path: path.to_string(),
		});
	}
	if path.split('/').any(|segment| segment == "..") {
		return Err(TypeScriptRenderError::PATH_TRAVERSAL {
			path: path.to_string(),
		});
	}
	Ok(())
}

fn lower_operation(
	node: &krunt_ir::IRNode,
) -> Result<TypeScriptOperation, TypeScriptLoweringError> {
	let id = TypeScriptOperationId::new(node.id.0);
	let kind = match &node.op {
		IROp::ENTRY { name } => {
			if name.as_str().trim().is_empty() {
				return Err(TypeScriptLoweringError::EmptyOperationName { node_id: id });
			}
			TypeScriptOperationKind::ENTRY {
				name: name.as_str().to_string(),
			}
		}
		IROp::OPERATION { name } => {
			if name.as_str().trim().is_empty() {
				return Err(TypeScriptLoweringError::EmptyOperationName { node_id: id });
			}
			TypeScriptOperationKind::OPERATION {
				name: name.as_str().to_string(),
			}
		}
		IROp::BRANCH => TypeScriptOperationKind::BRANCH,
		IROp::MERGE => TypeScriptOperationKind::MERGE,
		IROp::LOOP => TypeScriptOperationKind::LOOP,
		IROp::RETURN => TypeScriptOperationKind::RETURN,
	};

	Ok(TypeScriptOperation {
		id,
		kind,
		input_types: node.input_types.iter().map(lower_type).collect(),
		output_types: node.output_types.iter().map(lower_type).collect(),
		async_kind: node.async_kind,
		effects: node.effects.clone(),
		source: node.source.clone(),
	})
}

fn lower_type(type_id: &krunt_ir::TypeId) -> TypeScriptType {
	match type_id.as_str() {
		"ANY" | "any" => TypeScriptType::ANY,
		name => TypeScriptType::NAMED {
			name: name.to_string(),
		},
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use krunt_ir::{
		ControlEdge, DataEdge, GraphElementId, GraphNodeId, IRBuilder, IRNode, IROp, OperationId,
		PortRef, TypeId,
	};

	fn graph() -> IRGraph {
		let mut builder = IRBuilder::new();
		builder
			.add_node(IRNode {
				id: krunt_ir::IrNodeId(1),
				op: IROp::ENTRY {
					name: OperationId::new("route").unwrap(),
				},
				input_types: Vec::new(),
				output_types: vec![TypeId::new("Request").unwrap()],
				async_kind: AsyncKind::SYNC,
				effects: Vec::new(),
				source: SourceMap::node(GraphNodeId(10)),
			})
			.unwrap();
		builder
			.add_node(IRNode {
				id: krunt_ir::IrNodeId(2),
				op: IROp::OPERATION {
					name: OperationId::new("transform").unwrap(),
				},
				input_types: vec![TypeId::new("Request").unwrap()],
				output_types: vec![TypeId::new("Response").unwrap()],
				async_kind: AsyncKind::ASYNC,
				effects: Vec::new(),
				source: SourceMap::node(GraphNodeId(20)),
			})
			.unwrap();
		builder
			.add_control_edge(ControlEdge {
				from: krunt_ir::IrNodeId(1),
				to: krunt_ir::IrNodeId(2),
			})
			.unwrap();
		builder
			.add_data_edge(DataEdge {
				from: PortRef {
					node: krunt_ir::IrNodeId(1),
					index: 0,
				},
				to: PortRef {
					node: krunt_ir::IrNodeId(2),
					index: 0,
				},
			})
			.unwrap();
		builder.build().unwrap()
	}

	#[test]
	fn lowers_ir_into_a_deterministic_typescript_program() {
		let backend = TypeScriptBackend;
		let program = backend.lower(&graph()).unwrap();
		let module = &program.modules()[0];
		let function = &module.functions()[0];

		assert_eq!(backend.backend_id(), "typescript");
		assert_eq!(program.version(), TYPESCRIPT_IR_VERSION);
		assert_eq!(module.path(), "generated.ts");
		assert_eq!(function.name(), "execute");
		assert_eq!(function.async_kind(), AsyncKind::ASYNC);
		assert_eq!(function.operations().len(), 2);
		assert_eq!(function.control_edges().len(), 1);
		assert_eq!(function.data_edges().len(), 1);
		assert_eq!(
			function.operations()[0].source().primary,
			GraphElementId::NODE(GraphNodeId(10))
		);
		assert_eq!(
			function.operations()[1].output_types()[0],
			TypeScriptType::NAMED {
				name: "Response".to_string(),
			}
		);
	}

	#[test]
	fn serializes_the_same_program_identically() {
		let first =
			serde_json::to_string(&TypeScriptBackend::lower_graph(&graph()).unwrap()).unwrap();
		let second =
			serde_json::to_string(&TypeScriptBackend::lower_graph(&graph()).unwrap()).unwrap();

		assert_eq!(first, second);
		assert!(first.contains("generated.ts"));
		assert!(first.contains("Request"));
		assert!(first.contains("Response"));
	}
}
