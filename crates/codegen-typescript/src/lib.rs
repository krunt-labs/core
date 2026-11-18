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
	declarations: Vec<TypeScriptDeclaration>,
	functions: Vec<TypeScriptFunction>,
}

impl TypeScriptModule {
	pub fn path(&self) -> &str {
		&self.path
	}

	pub fn imports(&self) -> &[TypeScriptImport] {
		&self.imports
	}

	pub fn declarations(&self) -> &[TypeScriptDeclaration] {
		&self.declarations
	}

	pub fn functions(&self) -> &[TypeScriptFunction] {
		&self.functions
	}
}

/// A module-level semantic declaration preserved from universal IR.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TypeScriptDeclaration {
	id: TypeScriptOperationId,
	name: String,
	attributes: BTreeMap<String, String>,
	source: SourceMap,
}

impl TypeScriptDeclaration {
	pub fn id(&self) -> TypeScriptOperationId {
		self.id
	}

	pub fn name(&self) -> &str {
		&self.name
	}

	pub fn attributes(&self) -> &BTreeMap<String, String> {
		&self.attributes
	}

	pub fn source(&self) -> &SourceMap {
		&self.source
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
	kind: TypeScriptFunctionKind,
	subgraph_id: Option<u64>,
	entry: Option<TypeScriptOperationId>,
	async_kind: AsyncKind,
	requires_runtime: bool,
	operations: Vec<TypeScriptOperation>,
	control_edges: Vec<TypeScriptControlEdge>,
	data_edges: Vec<TypeScriptDataEdge>,
}

impl TypeScriptFunction {
	pub fn name(&self) -> &str {
		&self.name
	}

	pub fn kind(&self) -> TypeScriptFunctionKind {
		self.kind
	}

	pub fn subgraph_id(&self) -> Option<u64> {
		self.subgraph_id
	}

	pub fn entry(&self) -> Option<TypeScriptOperationId> {
		self.entry
	}

	pub fn async_kind(&self) -> AsyncKind {
		self.async_kind
	}

	pub fn requires_runtime(&self) -> bool {
		self.requires_runtime
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

/// The role of a generated TypeScript function.
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum TypeScriptFunctionKind {
	ENTRY,
	SUBGRAPH,
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
	attributes: BTreeMap<String, String>,
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

	pub fn attributes(&self) -> &BTreeMap<String, String> {
		&self.attributes
	}

	pub fn source(&self) -> &SourceMap {
		&self.source
	}

	/// Whether this operation requires the generated runtime adapter.
	pub fn requires_runtime(&self) -> bool {
		is_runtime_operation(self)
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
	DECLARATION { name: String },
	ENTRY { name: String },
	OPERATION { name: String },
	BRANCH,
	MERGE,
	LOOP,
	SUBGRAPH { id: u64 },
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
	from: TypeScriptPortRef,
	to: TypeScriptPortRef,
}

impl TypeScriptControlEdge {
	pub fn from(&self) -> TypeScriptOperationId {
		self.from.node()
	}

	pub fn to(&self) -> TypeScriptOperationId {
		self.to.node()
	}

	pub fn from_port(&self) -> TypeScriptPortRef {
		self.from
	}

	pub fn to_port(&self) -> TypeScriptPortRef {
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
		let all_operations = graph
			.nodes()
			.iter()
			.map(lower_operation)
			.collect::<Result<Vec<_>, _>>()?;
		let subgraph_node_ids: BTreeSet<_> = graph
			.subgraphs()
			.iter()
			.flat_map(|subgraph| {
				subgraph
					.nodes()
					.iter()
					.map(|node| TypeScriptOperationId::new(node.0))
			})
			.collect();
		let subgraph_operations: BTreeMap<_, Vec<_>> = graph
			.subgraphs()
			.iter()
			.map(|subgraph| {
				let members: BTreeSet<_> = subgraph
					.nodes()
					.iter()
					.map(|node| TypeScriptOperationId::new(node.0))
					.collect();
				(
					subgraph.id().0,
					all_operations
						.iter()
						.filter(|operation| members.contains(&operation.id))
						.cloned()
						.collect(),
				)
			})
			.collect();
		let mut subgraph_requires_runtime: BTreeMap<_, bool> = subgraph_operations
			.iter()
			.map(|(id, operations)| {
				(
					*id,
					operations.iter().any(TypeScriptOperation::requires_runtime),
				)
			})
			.collect();
		loop {
			let mut changed = false;
			for (id, operations) in &subgraph_operations {
				let requires_runtime =
					operations_require_runtime(operations, &subgraph_requires_runtime);
				if subgraph_requires_runtime.get(id).copied() != Some(requires_runtime) {
					subgraph_requires_runtime.insert(*id, requires_runtime);
					changed = true;
				}
			}
			if !changed {
				break;
			}
		}
		let declarations = all_operations
			.iter()
			.filter_map(|operation| match operation.kind() {
				TypeScriptOperationKind::DECLARATION { name } => Some(TypeScriptDeclaration {
					id: operation.id,
					name: name.clone(),
					attributes: operation.attributes.clone(),
					source: operation.source.clone(),
				}),
				_ => None,
			})
			.collect();
		let http_entries = all_operations
			.iter()
			.filter_map(|operation| {
				if matches!(operation.kind(), TypeScriptOperationKind::ENTRY { .. })
					&& is_http_entry(operation)
				{
					Some(operation.id)
				} else {
					None
				}
			})
			.collect::<Vec<_>>();

		let functions = if http_entries.is_empty() {
			let operations = all_operations
				.iter()
				.filter(|operation| !subgraph_node_ids.contains(&operation.id))
				.cloned()
				.collect::<Vec<_>>();
			let requires_runtime =
				operations_require_runtime(&operations, &subgraph_requires_runtime);
			vec![lower_function(
				"execute",
				graph,
				&operations,
				TypeScriptFunctionKind::ENTRY,
				None,
				None,
				requires_runtime,
			)]
		} else {
			http_entries
				.into_iter()
				.map(|entry| {
					let reachable = reachable_operations(graph, entry);
					let operations = all_operations
						.iter()
						.filter(|operation| {
							reachable.contains(&operation.id)
								&& !subgraph_node_ids.contains(&operation.id)
						})
						.cloned()
						.collect::<Vec<_>>();
					let requires_runtime =
						operations_require_runtime(&operations, &subgraph_requires_runtime);
					lower_function(
						&format!("execute_{}", entry.as_u32()),
						graph,
						&operations,
						TypeScriptFunctionKind::ENTRY,
						None,
						None,
						requires_runtime,
					)
				})
				.collect()
		};
		let mut functions = functions;
		for subgraph in graph.subgraphs() {
			let operations = subgraph_operations
				.get(&subgraph.id().0)
				.expect("subgraph operations were collected from the same IR graph");
			let entry = subgraph
				.inputs()
				.first()
				.map(|boundary| TypeScriptOperationId::new(boundary.target.node.0));
			functions.push(lower_function(
				&format!("subgraph_{}", subgraph.id().0),
				graph,
				operations,
				TypeScriptFunctionKind::SUBGRAPH,
				Some(subgraph.id().0),
				entry,
				subgraph_requires_runtime
					.get(&subgraph.id().0)
					.copied()
					.unwrap_or(false),
			));
		}

		Ok(TypeScriptProgram {
			version: TypeScriptProgram::CURRENT_VERSION,
			modules: vec![TypeScriptModule {
				path: "generated.ts".to_string(),
				imports: Vec::new(),
				declarations,
				functions,
			}],
		})
	}
}

fn is_http_entry(operation: &TypeScriptOperation) -> bool {
	operation.attributes().contains_key("backend.http.method")
		&& operation.attributes().contains_key("backend.http.path")
}

fn reachable_operations(
	graph: &IRGraph,
	entry: TypeScriptOperationId,
) -> BTreeSet<TypeScriptOperationId> {
	let mut reachable = BTreeSet::new();
	let mut worklist = vec![entry];
	while let Some(node) = worklist.pop() {
		if !reachable.insert(node) {
			continue;
		}
		for edge in graph.control_edges() {
			if edge.from.node.0 == node.as_u32() {
				worklist.push(TypeScriptOperationId::new(edge.to.node.0));
			}
		}
		for edge in graph.data_edges() {
			if edge.to.node.0 == node.as_u32() {
				worklist.push(TypeScriptOperationId::new(edge.from.node.0));
			}
		}
	}
	reachable
}

fn lower_function(
	name: &str,
	graph: &IRGraph,
	operations: &[TypeScriptOperation],
	kind: TypeScriptFunctionKind,
	subgraph_id: Option<u64>,
	entry: Option<TypeScriptOperationId>,
	requires_runtime: bool,
) -> TypeScriptFunction {
	let operation_ids: BTreeSet<_> = operations.iter().map(TypeScriptOperation::id).collect();
	let async_kind = if operations
		.iter()
		.any(|operation| operation.async_kind == AsyncKind::ASYNC)
	{
		AsyncKind::ASYNC
	} else {
		AsyncKind::SYNC
	};
	TypeScriptFunction {
		name: name.to_string(),
		kind,
		subgraph_id,
		entry: entry.or_else(|| {
			operations
				.iter()
				.find(|operation| matches!(operation.kind(), TypeScriptOperationKind::ENTRY { .. }))
				.map(TypeScriptOperation::id)
		}),
		async_kind,
		requires_runtime,
		operations: operations.to_vec(),
		control_edges: graph
			.control_edges()
			.iter()
			.filter(|edge| {
				operation_ids.contains(&TypeScriptOperationId::new(edge.from.node.0))
					&& operation_ids.contains(&TypeScriptOperationId::new(edge.to.node.0))
			})
			.map(|edge| TypeScriptControlEdge {
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
		data_edges: graph
			.data_edges()
			.iter()
			.filter(|edge| {
				operation_ids.contains(&TypeScriptOperationId::new(edge.from.node.0))
					&& operation_ids.contains(&TypeScriptOperationId::new(edge.to.node.0))
			})
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
	}
}

fn operations_require_runtime(
	operations: &[TypeScriptOperation],
	subgraph_requires_runtime: &BTreeMap<u64, bool>,
) -> bool {
	operations.iter().any(|operation| {
		operation.requires_runtime()
			|| match operation.kind() {
				TypeScriptOperationKind::SUBGRAPH { id } => {
					subgraph_requires_runtime.get(id).copied().unwrap_or(false)
				}
				_ => false,
			}
	})
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
	let subgraph_runtime_requirements: BTreeMap<_, _> = module
		.functions()
		.iter()
		.filter_map(|function| {
			function
				.subgraph_id()
				.map(|id| (id, function.requires_runtime()))
		})
		.collect();
	if module
		.functions()
		.iter()
		.any(TypeScriptFunction::requires_runtime)
	{
		source.push_str(&runtime_interface());
		source.push('\n');
	}

	for function in module.functions() {
		source.push_str(&render_function(function, &subgraph_runtime_requirements));
		source.push('\n');
	}
	source
}

fn render_function(
	function: &TypeScriptFunction,
	subgraph_runtime_requirements: &BTreeMap<u64, bool>,
) -> String {
	let mut source = String::new();
	let declarations = operation_declarations(function);
	for declaration in declarations.values() {
		source.push_str("declare function ");
		source.push_str(declaration);
		source.push('\n');
	}
	if !declarations.is_empty() {
		source.push('\n');
	}

	let function_name = sanitize_identifier(&function.name, "execute");
	let accepts_request = function.operations.iter().any(is_http_entry);
	let requires_runtime = function.requires_runtime();
	let async_prefix = if function.async_kind == AsyncKind::ASYNC {
		"async "
	} else {
		""
	};
	let response_type = if function.kind() == TypeScriptFunctionKind::SUBGRAPH {
		"void".to_string()
	} else {
		http_response_type(function)
	};
	let return_type = if function.async_kind == AsyncKind::ASYNC {
		format!("Promise<{response_type}>")
	} else {
		response_type
	};
	let request_parameter = http_request_parameter(function);
	let parameters = match (accepts_request, requires_runtime) {
		(true, true) => format!("{request_parameter}, runtime: KruntRuntime"),
		(true, false) => request_parameter,
		(false, true) => "runtime: KruntRuntime".to_string(),
		(false, false) => String::new(),
	};
	source.push_str(&format!(
		"export {}function {}({}): {} {{\n",
		async_prefix, function_name, parameters, return_type,
	));

	let data_inputs = data_inputs(function);
	let output_arities = output_arities(function);
	if has_structured_control_flow(function) {
		render_structured_function_body(
			&mut source,
			function,
			&data_inputs,
			&output_arities,
			subgraph_runtime_requirements,
		);
	} else {
		render_linear_function_body(&mut source, function, &data_inputs, &output_arities);
	}
	source.push_str("}\n");
	source
}

fn has_structured_control_flow(function: &TypeScriptFunction) -> bool {
	function.operations.iter().any(|operation| {
		matches!(
			operation.kind(),
			TypeScriptOperationKind::BRANCH
				| TypeScriptOperationKind::MERGE
				| TypeScriptOperationKind::LOOP
				| TypeScriptOperationKind::SUBGRAPH { .. }
		)
	})
}

fn render_linear_function_body(
	source: &mut String,
	function: &TypeScriptFunction,
	data_inputs: &BTreeMap<TypeScriptOperationId, Vec<Option<TypeScriptPortRef>>>,
	output_arities: &BTreeMap<TypeScriptOperationId, usize>,
) {
	for operation in &function.operations {
		render_operation_comments(source, operation, "  ");
		match operation.kind() {
			TypeScriptOperationKind::DECLARATION { .. } => {}
			TypeScriptOperationKind::ENTRY { name }
			| TypeScriptOperationKind::OPERATION { name } => {
				if is_http_entry(operation) {
					source.push_str("  const ");
					source.push_str(&operation_variable(operation.id));
					source.push_str(" = request;\n");
				} else {
					render_named_operation(source, operation, name, data_inputs, output_arities);
				}
			}
			TypeScriptOperationKind::BRANCH
			| TypeScriptOperationKind::MERGE
			| TypeScriptOperationKind::LOOP
			| TypeScriptOperationKind::SUBGRAPH { .. } => {
				unreachable!("structured control flow is rendered by the structured body renderer")
			}
			TypeScriptOperationKind::RETURN => {
				render_return(source, operation, data_inputs, output_arities, "  ");
			}
		}
	}
}

fn render_structured_function_body(
	source: &mut String,
	function: &TypeScriptFunction,
	data_inputs: &BTreeMap<TypeScriptOperationId, Vec<Option<TypeScriptPortRef>>>,
	output_arities: &BTreeMap<TypeScriptOperationId, usize>,
	subgraph_runtime_requirements: &BTreeMap<u64, bool>,
) {
	for operation in &function.operations {
		if operation.output_types.is_empty() {
			continue;
		}
		source.push_str("  let ");
		source.push_str(&operation_variable(operation.id));
		source.push_str(": ");
		source.push_str(&render_operation_output_type(operation));
		source.push_str(" = undefined as unknown as ");
		source.push_str(&render_operation_output_type(operation));
		source.push_str(";\n");
	}

	let start = function.entry.map(|entry| entry.as_u32());
	source.push_str("  let control: number | null = ");
	source.push_str(
		&start
			.map(|id| id.to_string())
			.unwrap_or_else(|| "null".to_string()),
	);
	source.push_str(";\n");
	source.push_str("  while (control !== null) {\n");
	source.push_str("    switch (control) {\n");

	for operation in &function.operations {
		if matches!(
			operation.kind(),
			TypeScriptOperationKind::DECLARATION { .. }
		) {
			continue;
		}
		source.push_str("      case ");
		source.push_str(&operation.id.as_u32().to_string());
		source.push_str(": {\n");
		render_operation_comments(source, operation, "        ");

		match operation.kind() {
			TypeScriptOperationKind::ENTRY { name }
			| TypeScriptOperationKind::OPERATION { name } => {
				if is_http_entry(operation) {
					source.push_str("        ");
					source.push_str(&operation_variable(operation.id));
					source.push_str(" = request;\n");
				} else if matches!(operation.kind(), TypeScriptOperationKind::OPERATION { .. }) {
					render_named_operation_with_assignment(
						source,
						operation,
						name,
						data_inputs,
						output_arities,
						true,
					);
				}
				render_next_control(source, function, operation, 0, "        ");
				source.push_str("        break;\n");
			}
			TypeScriptOperationKind::BRANCH => {
				let condition = data_input_expression(operation.id, 0, data_inputs, output_arities);
				let when_true = control_successor(function, operation.id, 0);
				let when_false = control_successor(function, operation.id, 1);
				source.push_str("        control = ");
				source.push_str(&condition);
				source.push_str(" ? ");
				source.push_str(&render_control_target(when_true));
				source.push_str(" : ");
				source.push_str(&render_control_target(when_false));
				source.push_str(";\n");
				source.push_str("        break;\n");
			}
			TypeScriptOperationKind::LOOP => {
				let condition = data_input_expression(operation.id, 0, data_inputs, output_arities);
				let body = control_successor(function, operation.id, 0);
				let done = control_successor(function, operation.id, 1);
				source.push_str("        control = ");
				source.push_str(&condition);
				source.push_str(" ? ");
				source.push_str(&render_control_target(body));
				source.push_str(" : ");
				source.push_str(&render_control_target(done));
				source.push_str(";\n");
				source.push_str("        break;\n");
			}
			TypeScriptOperationKind::MERGE => {
				render_next_control(source, function, operation, 0, "        ");
				source.push_str("        break;\n");
			}
			TypeScriptOperationKind::SUBGRAPH { id } => {
				source.push_str("        // krunt:subgraph=");
				source.push_str(&id.to_string());
				source.push('\n');
				source.push_str("        await subgraph_");
				source.push_str(&id.to_string());
				source.push('(');
				if subgraph_runtime_requirements
					.get(id)
					.copied()
					.unwrap_or(false)
				{
					source.push_str("runtime");
				}
				source.push_str(");\n");
				render_next_control(source, function, operation, 0, "        ");
				source.push_str("        break;\n");
			}
			TypeScriptOperationKind::RETURN => {
				render_return(source, operation, data_inputs, output_arities, "        ");
			}
			TypeScriptOperationKind::DECLARATION { .. } => unreachable!(),
		}
		source.push_str("      }\n");
	}

	source.push_str("      default: {\n");
	source.push_str("        control = null;\n");
	source.push_str("        break;\n");
	source.push_str("      }\n");
	source.push_str("    }\n");
	source.push_str("  }\n");
	if function.kind() == TypeScriptFunctionKind::SUBGRAPH {
		source.push_str("  return;\n");
	} else {
		source.push_str("  return undefined as never;\n");
	}
}

fn render_operation_comments(source: &mut String, operation: &TypeScriptOperation, indent: &str) {
	source.push_str(indent);
	source.push_str("// krunt:source=");
	source.push_str(&source_marker(operation.source()));
	source.push('\n');
	source.push_str(indent);
	source.push_str("// krunt:operation=");
	source.push_str(&operation.id.as_u32().to_string());
	source.push(' ');
	source.push_str(&render_operation_kind(operation.kind()));
	source.push('\n');
	if !operation.input_types.is_empty() || !operation.output_types.is_empty() {
		source.push_str(indent);
		source.push_str("// krunt:types=");
		source.push_str(&render_types(&operation.input_types));
		source.push_str(" -> ");
		source.push_str(&render_types(&operation.output_types));
		source.push('\n');
	}
}

fn render_return(
	source: &mut String,
	operation: &TypeScriptOperation,
	data_inputs: &BTreeMap<TypeScriptOperationId, Vec<Option<TypeScriptPortRef>>>,
	output_arities: &BTreeMap<TypeScriptOperationId, usize>,
	indent: &str,
) {
	source.push_str(indent);
	source.push_str("return ");
	if let Some(value) = first_data_input(operation.id, data_inputs) {
		source.push_str(&render_data_reference(value, output_arities));
	} else {
		source.push_str("undefined as never");
	}
	source.push_str(";\n");
}

fn control_successor(
	function: &TypeScriptFunction,
	operation: TypeScriptOperationId,
	output_index: u16,
) -> Option<TypeScriptOperationId> {
	function
		.control_edges
		.iter()
		.find(|edge| edge.from.node == operation && edge.from.index == output_index)
		.map(|edge| edge.to.node)
}

fn render_control_target(target: Option<TypeScriptOperationId>) -> String {
	target
		.map(|id| id.as_u32().to_string())
		.unwrap_or_else(|| "null".to_string())
}

fn render_next_control(
	source: &mut String,
	function: &TypeScriptFunction,
	operation: &TypeScriptOperation,
	output_index: u16,
	indent: &str,
) {
	source.push_str(indent);
	source.push_str("control = ");
	source.push_str(&render_control_target(control_successor(
		function,
		operation.id,
		output_index,
	)));
	source.push_str(";\n");
}

fn http_request_parameter(function: &TypeScriptFunction) -> String {
	let request_type = function
		.operations
		.iter()
		.find(|operation| is_http_entry(operation))
		.and_then(|operation| operation.attributes().get("backend.http.request_entity"));

	match request_type {
		Some(request_type) => format!("request: {request_type}"),
		None => "request: unknown = undefined".to_string(),
	}
}

fn http_response_type(function: &TypeScriptFunction) -> String {
	function
		.operations
		.iter()
		.find(|operation| matches!(operation.kind(), TypeScriptOperationKind::RETURN))
		.and_then(|operation| operation.attributes().get("response.body_type"))
		.filter(|type_name| type_name.as_str() != "ANY")
		.cloned()
		.unwrap_or_else(|| "unknown".to_string())
}

fn operation_declarations(function: &TypeScriptFunction) -> BTreeMap<String, String> {
	function
		.operations
		.iter()
		.filter_map(|operation| {
			if is_identity_transform(operation) || operation.requires_runtime() {
				return None;
			}
			match operation.kind() {
				TypeScriptOperationKind::ENTRY { name }
				| TypeScriptOperationKind::OPERATION { name }
					if !is_http_entry(operation) =>
				{
					Some((
						operation.id.as_u32().to_string(),
						render_operation_declaration(operation, name),
					))
				}
				_ => None,
			}
		})
		.collect()
}

fn render_operation_declaration(operation: &TypeScriptOperation, name: &str) -> String {
	let parameters = operation
		.input_types()
		.iter()
		.enumerate()
		.map(|(index, type_value)| {
			format!(
				"input_{index}: {}",
				render_type_annotation(Some(type_value))
			)
		})
		.collect::<Vec<_>>()
		.join(", ");
	let output_type = if operation.output_types().is_empty() {
		"void".to_string()
	} else {
		render_operation_output_type(operation)
	};
	let return_type = if operation.async_kind() == AsyncKind::ASYNC {
		format!("Promise<{output_type}>")
	} else {
		output_type
	};
	format!(
		"{}({parameters}): {return_type};",
		operation_callee(operation.id, name)
	)
}

fn is_identity_transform(operation: &TypeScriptOperation) -> bool {
	operation
		.attributes()
		.get("backend.transform.kind")
		.is_some_and(|kind| kind == "IDENTITY")
}

fn is_runtime_operation(operation: &TypeScriptOperation) -> bool {
	if operation
		.attributes()
		.contains_key("backend.persistence.kind")
	{
		return true;
	}
	matches!(
		operation
			.attributes()
			.get("backend.effect.kind")
			.map(String::as_str),
		Some("HTTP_REQUEST" | "LOGGING" | "STORAGE")
	)
}

fn runtime_interface() -> &'static str {
	r#"export interface KruntRuntime {
  httpRequest(method: string, url: string, request: unknown): Promise<unknown>;
  query<TOutput>(name: string, repository: string): Promise<TOutput>;
  mutation<TInput, TOutput>(name: string, repository: string, value: TInput): Promise<TOutput>;
  log(level: string, message: unknown): void;
  storageRead(key: string): Promise<unknown>;
  storageWrite(key: string, value: unknown): Promise<unknown>;
}
"#
}

fn render_named_operation(
	source: &mut String,
	operation: &TypeScriptOperation,
	name: &str,
	data_inputs: &BTreeMap<TypeScriptOperationId, Vec<Option<TypeScriptPortRef>>>,
	output_arities: &BTreeMap<TypeScriptOperationId, usize>,
) {
	render_named_operation_with_assignment(
		source,
		operation,
		name,
		data_inputs,
		output_arities,
		false,
	);
}

fn render_named_operation_with_assignment(
	source: &mut String,
	operation: &TypeScriptOperation,
	name: &str,
	data_inputs: &BTreeMap<TypeScriptOperationId, Vec<Option<TypeScriptPortRef>>>,
	output_arities: &BTreeMap<TypeScriptOperationId, usize>,
	mutable_assignment: bool,
) {
	if is_identity_transform(operation) {
		if mutable_assignment {
			source.push_str("        ");
		} else {
			source.push_str("  const ");
		}
		source.push_str(&operation_variable(operation.id));
		source.push_str(" = ");
		if let Some(value) = first_data_input(operation.id, data_inputs) {
			source.push_str(&render_data_reference(value, output_arities));
		} else {
			source.push_str("undefined");
		}
		source.push_str(";\n");
		return;
	}
	if let Some(invocation) = runtime_invocation(operation, data_inputs, output_arities) {
		render_invocation(source, operation, &invocation, mutable_assignment);
		return;
	}

	let callee = operation_callee(operation.id, name);
	let arguments = data_inputs
		.get(&operation.id)
		.map(|inputs| {
			inputs
				.iter()
				.map(|input| {
					input
						.map(|input| render_data_reference(input, output_arities))
						.unwrap_or_else(|| "undefined".to_string())
				})
				.collect::<Vec<_>>()
				.join(", ")
		})
		.unwrap_or_default();
	render_invocation(
		source,
		operation,
		&format!("{}({})", callee, arguments),
		mutable_assignment,
	);
}

fn render_invocation(
	source: &mut String,
	operation: &TypeScriptOperation,
	invocation: &str,
	mutable_assignment: bool,
) {
	let await_prefix = if operation.async_kind == AsyncKind::ASYNC {
		"await "
	} else {
		""
	};
	let invocation = format!("{await_prefix}{invocation}");
	if operation.output_types.is_empty() {
		if mutable_assignment {
			source.push_str("        ");
		} else {
			source.push_str("  ");
		}
		source.push_str(&invocation);
		source.push_str(";\n");
	} else {
		if mutable_assignment {
			source.push_str("        ");
			source.push_str(&operation_variable(operation.id));
		} else {
			source.push_str("  const ");
			source.push_str(&operation_variable(operation.id));
		}
		source.push_str(" = ");
		source.push_str(&invocation);
		source.push_str(";\n");
	}
}

fn runtime_invocation(
	operation: &TypeScriptOperation,
	data_inputs: &BTreeMap<TypeScriptOperationId, Vec<Option<TypeScriptPortRef>>>,
	output_arities: &BTreeMap<TypeScriptOperationId, usize>,
) -> Option<String> {
	let input = |index| data_input_expression(operation.id, index, data_inputs, output_arities);
	let attributes = operation.attributes();
	if attributes
		.get("backend.persistence.kind")
		.map(String::as_str)
		== Some("QUERY")
	{
		return Some(format!(
			"runtime.query<{}>({}, {})",
			render_operation_output_type(operation),
			string_literal(attributes.get("backend.query.name")?),
			string_literal(attributes.get("backend.query.repository")?),
		));
	}
	if attributes
		.get("backend.persistence.kind")
		.map(String::as_str)
		== Some("MUTATION")
	{
		return Some(format!(
			"runtime.mutation<{}, {}>({}, {}, {})",
			render_type_annotation(operation.input_types().first()),
			render_operation_output_type(operation),
			string_literal(attributes.get("backend.mutation.name")?),
			string_literal(attributes.get("backend.mutation.repository")?),
			input(0),
		));
	}

	match attributes.get("backend.effect.kind").map(String::as_str) {
		Some("HTTP_REQUEST") => Some(format!(
			"runtime.httpRequest({}, {}, {})",
			string_literal(attributes.get("backend.http_request.method")?),
			string_literal(attributes.get("backend.http_request.url")?),
			input(0),
		)),
		Some("LOGGING") => Some(format!(
			"runtime.log({}, {})",
			string_literal(attributes.get("backend.logging.level")?),
			input(0),
		)),
		Some("STORAGE") => match attributes
			.get("backend.storage.operation")
			.map(String::as_str)
		{
			Some("READ") => Some(format!(
				"runtime.storageRead({})",
				string_literal(attributes.get("backend.storage.key")?),
			)),
			Some("WRITE") => Some(format!(
				"runtime.storageWrite({}, {})",
				string_literal(attributes.get("backend.storage.key")?),
				input(0),
			)),
			_ => None,
		},
		_ => None,
	}
}

fn render_type_annotation(type_value: Option<&TypeScriptType>) -> String {
	match type_value {
		Some(TypeScriptType::ANY) | None => "unknown".to_string(),
		Some(TypeScriptType::NAMED { name }) => name.clone(),
	}
}

fn render_operation_output_type(operation: &TypeScriptOperation) -> String {
	if is_http_entry(operation) {
		return operation
			.attributes()
			.get("backend.http.request_entity")
			.cloned()
			.unwrap_or_else(|| "unknown".to_string());
	}
	match operation.output_types().len() {
		0 => "unknown".to_string(),
		1 => render_type_annotation(operation.output_types().first()),
		_ => format!(
			"[{}]",
			operation
				.output_types()
				.iter()
				.map(|type_value| render_type_annotation(Some(type_value)))
				.collect::<Vec<_>>()
				.join(", ")
		),
	}
}

fn data_input_expression(
	operation: TypeScriptOperationId,
	index: usize,
	data_inputs: &BTreeMap<TypeScriptOperationId, Vec<Option<TypeScriptPortRef>>>,
	output_arities: &BTreeMap<TypeScriptOperationId, usize>,
) -> String {
	data_inputs
		.get(&operation)
		.and_then(|inputs| inputs.get(index))
		.and_then(|input| input.map(|input| render_data_reference(input, output_arities)))
		.unwrap_or_else(|| "undefined".to_string())
}

fn string_literal(value: &str) -> String {
	format!("\"{}\"", escape_double_quoted(value))
}

fn escape_double_quoted(value: &str) -> String {
	value
		.chars()
		.flat_map(|character| match character {
			'\\' => "\\\\".chars().collect::<Vec<_>>(),
			'"' => "\\\"".chars().collect::<Vec<_>>(),
			'\n' => "\\n".chars().collect::<Vec<_>>(),
			'\r' => "\\r".chars().collect::<Vec<_>>(),
			'\t' => "\\t".chars().collect::<Vec<_>>(),
			character if character.is_control() => {
				format!("\\u{:04x}", character as u32).chars().collect()
			}
			character => vec![character],
		})
		.collect()
}

fn data_inputs(
	function: &TypeScriptFunction,
) -> BTreeMap<TypeScriptOperationId, Vec<Option<TypeScriptPortRef>>> {
	let mut inputs: BTreeMap<_, BTreeMap<_, _>> = BTreeMap::new();
	for edge in &function.data_edges {
		inputs
			.entry(edge.to.node)
			.or_default()
			.insert(edge.to.index, edge.from);
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
	inputs: &BTreeMap<TypeScriptOperationId, Vec<Option<TypeScriptPortRef>>>,
) -> Option<TypeScriptPortRef> {
	inputs
		.get(&operation)
		.and_then(|values| values.iter().flatten().next().copied())
}

fn output_arities(function: &TypeScriptFunction) -> BTreeMap<TypeScriptOperationId, usize> {
	function
		.operations
		.iter()
		.map(|operation| (operation.id(), operation.output_types().len()))
		.collect()
}

fn render_data_reference(
	port: TypeScriptPortRef,
	output_arities: &BTreeMap<TypeScriptOperationId, usize>,
) -> String {
	let variable = operation_variable(port.node());
	if output_arities.get(&port.node()).copied().unwrap_or(1) > 1 {
		format!("{variable}[{}]", port.index())
	} else {
		variable
	}
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
		TypeScriptOperationKind::DECLARATION { name } => format!("DECLARATION {name}"),
		TypeScriptOperationKind::ENTRY { name } => format!("ENTRY {name}"),
		TypeScriptOperationKind::OPERATION { name } => format!("OPERATION {name}"),
		TypeScriptOperationKind::BRANCH => "BRANCH".to_string(),
		TypeScriptOperationKind::MERGE => "MERGE".to_string(),
		TypeScriptOperationKind::LOOP => "LOOP".to_string(),
		TypeScriptOperationKind::SUBGRAPH { id } => format!("SUBGRAPH {id}"),
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
		IROp::DECLARATION { name } => TypeScriptOperationKind::DECLARATION {
			name: name.as_str().to_string(),
		},
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
		IROp::SUBGRAPH { id } => TypeScriptOperationKind::SUBGRAPH { id: id.0 },
		IROp::RETURN => TypeScriptOperationKind::RETURN,
	};

	Ok(TypeScriptOperation {
		id,
		kind,
		input_types: node.input_types.iter().map(lower_type).collect(),
		output_types: node.output_types.iter().map(lower_type).collect(),
		async_kind: node.async_kind,
		effects: node.effects.clone(),
		attributes: node.attributes.clone(),
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
				attributes: BTreeMap::new(),
				returns: false,
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
				attributes: BTreeMap::new(),
				returns: false,
				source: SourceMap::node(GraphNodeId(20)),
			})
			.unwrap();
		builder
			.add_control_edge(ControlEdge {
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
		assert_eq!(function.control_edges()[0].from_port().index(), 0);
		assert_eq!(function.control_edges()[0].to_port().index(), 0);
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

	#[test]
	fn preserves_module_declarations_for_framework_backends() {
		let mut builder = IRBuilder::new();
		builder
			.add_node(IRNode {
				id: krunt_ir::IrNodeId(1),
				op: IROp::DECLARATION {
					name: OperationId::new("backend.entity").unwrap(),
				},
				input_types: Vec::new(),
				output_types: Vec::new(),
				async_kind: AsyncKind::SYNC,
				effects: Vec::new(),
				attributes: BTreeMap::from([
					("backend.entity.name".to_string(), "User".to_string()),
					(
						"backend.entity.fields".to_string(),
						"{\"id\":{\"type\":\"Integer\",\"required\":true}}".to_string(),
					),
				]),
				returns: false,
				source: SourceMap::node(GraphNodeId(10)),
			})
			.unwrap();
		builder
			.add_node(IRNode {
				id: krunt_ir::IrNodeId(2),
				op: IROp::ENTRY {
					name: OperationId::new("route").unwrap(),
				},
				input_types: Vec::new(),
				output_types: Vec::new(),
				async_kind: AsyncKind::SYNC,
				effects: Vec::new(),
				attributes: BTreeMap::new(),
				returns: false,
				source: SourceMap::node(GraphNodeId(20)),
			})
			.unwrap();

		let program = TypeScriptBackend::lower_graph(&builder.build().unwrap()).unwrap();
		let declaration = &program.modules()[0].declarations()[0];

		assert_eq!(declaration.id().as_u32(), 1);
		assert_eq!(declaration.name(), "backend.entity");
		assert_eq!(declaration.attributes()["backend.entity.name"], "User");
		assert_eq!(
			declaration.attributes()["backend.entity.fields"],
			"{\"id\":{\"type\":\"Integer\",\"required\":true}}"
		);
		assert_eq!(
			declaration.source().primary,
			GraphElementId::NODE(GraphNodeId(10))
		);
	}

	#[test]
	fn renders_identity_transform_as_a_data_assignment() {
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
				attributes: BTreeMap::new(),
				returns: false,
				source: SourceMap::node(GraphNodeId(10)),
			})
			.unwrap();
		let mut attributes = BTreeMap::new();
		attributes.insert("backend.transform.kind".to_string(), "IDENTITY".to_string());
		builder
			.add_node(IRNode {
				id: krunt_ir::IrNodeId(2),
				op: IROp::OPERATION {
					name: OperationId::new("backend.transform").unwrap(),
				},
				input_types: vec![TypeId::new("Request").unwrap()],
				output_types: vec![TypeId::new("Request").unwrap()],
				async_kind: AsyncKind::SYNC,
				effects: Vec::new(),
				attributes,
				returns: false,
				source: SourceMap::node(GraphNodeId(20)),
			})
			.unwrap();
		builder
			.add_node(IRNode {
				id: krunt_ir::IrNodeId(3),
				op: IROp::RETURN,
				input_types: vec![TypeId::new("Request").unwrap()],
				output_types: Vec::new(),
				async_kind: AsyncKind::SYNC,
				effects: Vec::new(),
				attributes: BTreeMap::new(),
				returns: true,
				source: SourceMap::node(GraphNodeId(30)),
			})
			.unwrap();
		for (from, to) in [(1, 2), (2, 3)] {
			builder
				.add_control_edge(ControlEdge {
					from: PortRef {
						node: krunt_ir::IrNodeId(from),
						index: 0,
					},
					to: PortRef {
						node: krunt_ir::IrNodeId(to),
						index: 0,
					},
				})
				.unwrap();
		}
		for (from, to) in [(1, 2), (2, 3)] {
			builder
				.add_data_edge(DataEdge {
					from: PortRef {
						node: krunt_ir::IrNodeId(from),
						index: 0,
					},
					to: PortRef {
						node: krunt_ir::IrNodeId(to),
						index: 0,
					},
				})
				.unwrap();
		}

		let program = TypeScriptBackend::lower_graph(&builder.build().unwrap()).unwrap();
		let source = TypeScriptSourceRenderer::render_program(&program)
			.unwrap()
			.remove("generated.ts")
			.unwrap();

		assert!(source.contains("const operation_2 = operation_1;"));
		assert!(source.contains("return operation_2;"));
		assert!(!source.contains("declare function krunt_backend_transform"));
	}

	#[test]
	fn renders_typed_declarations_for_non_runtime_operations() {
		let source = TypeScriptSourceRenderer::render_program(
			&TypeScriptBackend::lower_graph(&graph()).unwrap(),
		)
		.unwrap()
		.remove("generated.ts")
		.unwrap();

		assert!(source
			.contains("declare function krunt_transform_2(input_0: Request): Promise<Response>;"));
		assert!(source.contains("const operation_2 = await krunt_transform_2(operation_1);"));
		assert!(!source.contains("...args: unknown[]"));
	}

	#[test]
	fn preserves_source_port_indexes_when_rendering_data_flow() {
		let mut builder = IRBuilder::new();
		builder
			.add_node(IRNode {
				id: krunt_ir::IrNodeId(1),
				op: IROp::ENTRY {
					name: OperationId::new("backend.http_route").unwrap(),
				},
				input_types: Vec::new(),
				output_types: vec![TypeId::new("Request").unwrap()],
				async_kind: AsyncKind::SYNC,
				effects: Vec::new(),
				attributes: BTreeMap::from([
					("backend.http.method".to_string(), "GET".to_string()),
					("backend.http.path".to_string(), "/split".to_string()),
				]),
				returns: false,
				source: SourceMap::node(GraphNodeId(10)),
			})
			.unwrap();
		builder
			.add_node(IRNode {
				id: krunt_ir::IrNodeId(2),
				op: IROp::OPERATION {
					name: OperationId::new("split").unwrap(),
				},
				input_types: vec![TypeId::new("Request").unwrap()],
				output_types: vec![TypeId::new("Left").unwrap(), TypeId::new("Right").unwrap()],
				async_kind: AsyncKind::SYNC,
				effects: Vec::new(),
				attributes: BTreeMap::new(),
				returns: false,
				source: SourceMap::node(GraphNodeId(20)),
			})
			.unwrap();
		builder
			.add_node(IRNode {
				id: krunt_ir::IrNodeId(3),
				op: IROp::OPERATION {
					name: OperationId::new("consume").unwrap(),
				},
				input_types: vec![TypeId::new("Right").unwrap()],
				output_types: vec![TypeId::new("Result").unwrap()],
				async_kind: AsyncKind::SYNC,
				effects: Vec::new(),
				attributes: BTreeMap::new(),
				returns: false,
				source: SourceMap::node(GraphNodeId(30)),
			})
			.unwrap();
		builder
			.add_node(IRNode {
				id: krunt_ir::IrNodeId(4),
				op: IROp::RETURN,
				input_types: vec![TypeId::new("Result").unwrap()],
				output_types: Vec::new(),
				async_kind: AsyncKind::SYNC,
				effects: Vec::new(),
				attributes: BTreeMap::from([(
					"response.body_type".to_string(),
					"Result".to_string(),
				)]),
				returns: true,
				source: SourceMap::node(GraphNodeId(40)),
			})
			.unwrap();

		for (from, to) in [(1, 2), (2, 3), (3, 4)] {
			builder
				.add_control_edge(ControlEdge {
					from: PortRef {
						node: krunt_ir::IrNodeId(from),
						index: 0,
					},
					to: PortRef {
						node: krunt_ir::IrNodeId(to),
						index: 0,
					},
				})
				.unwrap();
		}
		for (from, from_index, to) in [(1, 0, 2), (2, 1, 3), (3, 0, 4)] {
			builder
				.add_data_edge(DataEdge {
					from: PortRef {
						node: krunt_ir::IrNodeId(from),
						index: from_index,
					},
					to: PortRef {
						node: krunt_ir::IrNodeId(to),
						index: 0,
					},
				})
				.unwrap();
		}

		let source = TypeScriptSourceRenderer::render_program(
			&TypeScriptBackend::lower_graph(&builder.build().unwrap()).unwrap(),
		)
		.unwrap()
		.remove("generated.ts")
		.unwrap();

		assert!(source.contains("declare function krunt_split_2(input_0: Request): [Left, Right];"));
		assert!(source.contains("declare function krunt_consume_3(input_0: Right): Result;"));
		assert!(source.contains("const operation_3 = krunt_consume_3(operation_2[1]);"));
		assert!(source.contains("return operation_3;"));
	}

	#[test]
	fn partitions_http_entries_into_deterministic_execution_functions() {
		let mut builder = IRBuilder::new();
		for (id, method, path) in [(1, "GET", "/users"), (3, "POST", "/posts")] {
			let mut attributes = BTreeMap::new();
			attributes.insert("backend.http.method".to_string(), method.to_string());
			attributes.insert("backend.http.path".to_string(), path.to_string());
			builder
				.add_node(IRNode {
					id: krunt_ir::IrNodeId(id),
					op: IROp::ENTRY {
						name: OperationId::new("backend.http_route").unwrap(),
					},
					input_types: Vec::new(),
					output_types: vec![TypeId::new("ResponseBody").unwrap()],
					async_kind: AsyncKind::SYNC,
					effects: Vec::new(),
					attributes,
					returns: false,
					source: SourceMap::node(GraphNodeId(id as u64)),
				})
				.unwrap();
			builder
				.add_node(IRNode {
					id: krunt_ir::IrNodeId(id + 1),
					op: IROp::RETURN,
					input_types: vec![TypeId::new("ResponseBody").unwrap()],
					output_types: Vec::new(),
					async_kind: AsyncKind::SYNC,
					effects: Vec::new(),
					attributes: BTreeMap::new(),
					returns: true,
					source: SourceMap::node(GraphNodeId((id + 1) as u64)),
				})
				.unwrap();
			builder
				.add_control_edge(ControlEdge {
					from: PortRef {
						node: krunt_ir::IrNodeId(id),
						index: 0,
					},
					to: PortRef {
						node: krunt_ir::IrNodeId(id + 1),
						index: 0,
					},
				})
				.unwrap();
			builder
				.add_data_edge(DataEdge {
					from: PortRef {
						node: krunt_ir::IrNodeId(id),
						index: 0,
					},
					to: PortRef {
						node: krunt_ir::IrNodeId(id + 1),
						index: 0,
					},
				})
				.unwrap();
		}

		let program = TypeScriptBackend::lower_graph(&builder.build().unwrap()).unwrap();
		assert_eq!(
			program.modules()[0]
				.functions()
				.iter()
				.map(TypeScriptFunction::name)
				.collect::<Vec<_>>(),
			["execute_1", "execute_3"]
		);
		let generated = TypeScriptSourceRenderer::render_program(&program).unwrap();
		let source = generated.get("generated.ts").unwrap();
		assert!(source.contains("export function execute_1(request: unknown = undefined): unknown"));
		assert!(source.contains("export function execute_3(request: unknown = undefined): unknown"));
		assert!(source.contains("return operation_1;"));
		assert!(source.contains("return operation_3;"));
		assert!(!source.contains("declare function krunt_backend_http_route"));
	}

	#[test]
	fn renders_a_typed_http_request_parameter_from_route_metadata() {
		let mut builder = IRBuilder::new();
		let attributes = BTreeMap::from([
			("backend.http.method".to_string(), "POST".to_string()),
			("backend.http.path".to_string(), "/users".to_string()),
			(
				"backend.http.request_entity".to_string(),
				"User".to_string(),
			),
		]);
		builder
			.add_node(IRNode {
				id: krunt_ir::IrNodeId(1),
				op: IROp::ENTRY {
					name: OperationId::new("backend.http_route").unwrap(),
				},
				input_types: Vec::new(),
				output_types: vec![TypeId::new("HttpRequest").unwrap()],
				async_kind: AsyncKind::ASYNC,
				effects: Vec::new(),
				attributes,
				returns: false,
				source: SourceMap::node(GraphNodeId(10)),
			})
			.unwrap();
		builder
			.add_node(IRNode {
				id: krunt_ir::IrNodeId(2),
				op: IROp::OPERATION {
					name: OperationId::new("backend.query").unwrap(),
				},
				input_types: Vec::new(),
				output_types: vec![TypeId::new("Array<User>").unwrap()],
				async_kind: AsyncKind::ASYNC,
				effects: Vec::new(),
				attributes: BTreeMap::from([
					("backend.persistence.kind".to_string(), "QUERY".to_string()),
					("backend.query.name".to_string(), "ListUsers".to_string()),
					(
						"backend.query.repository".to_string(),
						"UserRepository".to_string(),
					),
				]),
				returns: false,
				source: SourceMap::node(GraphNodeId(20)),
			})
			.unwrap();
		builder
			.add_node(IRNode {
				id: krunt_ir::IrNodeId(3),
				op: IROp::RETURN,
				input_types: vec![TypeId::new("Array<User>").unwrap()],
				output_types: Vec::new(),
				async_kind: AsyncKind::ASYNC,
				effects: Vec::new(),
				attributes: BTreeMap::from([(
					"response.body_type".to_string(),
					"Array<User>".to_string(),
				)]),
				returns: true,
				source: SourceMap::node(GraphNodeId(30)),
			})
			.unwrap();
		for (from, to) in [(1, 2), (2, 3)] {
			builder
				.add_control_edge(ControlEdge {
					from: PortRef {
						node: krunt_ir::IrNodeId(from),
						index: 0,
					},
					to: PortRef {
						node: krunt_ir::IrNodeId(to),
						index: 0,
					},
				})
				.unwrap();
		}
		builder
			.add_data_edge(DataEdge {
				from: PortRef {
					node: krunt_ir::IrNodeId(2),
					index: 0,
				},
				to: PortRef {
					node: krunt_ir::IrNodeId(3),
					index: 0,
				},
			})
			.unwrap();

		let program = TypeScriptBackend::lower_graph(&builder.build().unwrap()).unwrap();
		let source = TypeScriptSourceRenderer::render_program(&program)
			.unwrap()
			.remove("generated.ts")
			.unwrap();

		assert!(
			source.contains(
				"export async function execute_1(request: User, runtime: KruntRuntime): Promise<Array<User>>",
			)
		);
	}

	#[test]
	fn renders_runtime_backed_operations_with_traceable_calls() {
		let mut builder = IRBuilder::new();
		let mut http_attributes = BTreeMap::new();
		http_attributes.insert("backend.http.method".to_string(), "GET".to_string());
		http_attributes.insert("backend.http.path".to_string(), "/effects".to_string());
		let nodes = vec![
			IRNode {
				id: krunt_ir::IrNodeId(1),
				op: IROp::ENTRY {
					name: OperationId::new("backend.http_route").unwrap(),
				},
				input_types: Vec::new(),
				output_types: vec![TypeId::new("Request").unwrap()],
				async_kind: AsyncKind::ASYNC,
				effects: Vec::new(),
				attributes: http_attributes,
				returns: false,
				source: SourceMap::node(GraphNodeId(10)),
			},
			IRNode {
				id: krunt_ir::IrNodeId(2),
				op: IROp::OPERATION {
					name: OperationId::new("backend.http_request").unwrap(),
				},
				input_types: vec![TypeId::new("Request").unwrap()],
				output_types: vec![TypeId::new("Response").unwrap()],
				async_kind: AsyncKind::ASYNC,
				effects: Vec::new(),
				attributes: BTreeMap::from([
					(
						"backend.effect.kind".to_string(),
						"HTTP_REQUEST".to_string(),
					),
					(
						"backend.http_request.method".to_string(),
						"POST".to_string(),
					),
					(
						"backend.http_request.url".to_string(),
						"https://example.test/items".to_string(),
					),
				]),
				returns: false,
				source: SourceMap::node(GraphNodeId(20)),
			},
			IRNode {
				id: krunt_ir::IrNodeId(3),
				op: IROp::OPERATION {
					name: OperationId::new("backend.logging").unwrap(),
				},
				input_types: vec![TypeId::new("Response").unwrap()],
				output_types: Vec::new(),
				async_kind: AsyncKind::SYNC,
				effects: Vec::new(),
				attributes: BTreeMap::from([
					("backend.effect.kind".to_string(), "LOGGING".to_string()),
					("backend.logging.level".to_string(), "INFO".to_string()),
				]),
				returns: false,
				source: SourceMap::node(GraphNodeId(30)),
			},
			IRNode {
				id: krunt_ir::IrNodeId(4),
				op: IROp::OPERATION {
					name: OperationId::new("backend.storage").unwrap(),
				},
				input_types: Vec::new(),
				output_types: vec![TypeId::new("Response").unwrap()],
				async_kind: AsyncKind::ASYNC,
				effects: Vec::new(),
				attributes: BTreeMap::from([
					("backend.effect.kind".to_string(), "STORAGE".to_string()),
					("backend.storage.operation".to_string(), "READ".to_string()),
					(
						"backend.storage.key".to_string(),
						"last-response".to_string(),
					),
				]),
				returns: false,
				source: SourceMap::node(GraphNodeId(40)),
			},
			IRNode {
				id: krunt_ir::IrNodeId(5),
				op: IROp::RETURN,
				input_types: vec![TypeId::new("Response").unwrap()],
				output_types: Vec::new(),
				async_kind: AsyncKind::ASYNC,
				effects: Vec::new(),
				attributes: BTreeMap::new(),
				returns: true,
				source: SourceMap::node(GraphNodeId(50)),
			},
		];
		for node in nodes {
			builder.add_node(node).unwrap();
		}
		for (from, to) in [(1, 2), (2, 3), (3, 4), (4, 5)] {
			builder
				.add_control_edge(ControlEdge {
					from: PortRef {
						node: krunt_ir::IrNodeId(from),
						index: 0,
					},
					to: PortRef {
						node: krunt_ir::IrNodeId(to),
						index: 0,
					},
				})
				.unwrap();
		}
		for (from, to) in [(1, 2), (2, 3), (4, 5)] {
			builder
				.add_data_edge(DataEdge {
					from: PortRef {
						node: krunt_ir::IrNodeId(from),
						index: 0,
					},
					to: PortRef {
						node: krunt_ir::IrNodeId(to),
						index: 0,
					},
				})
				.unwrap();
		}

		let program = TypeScriptBackend::lower_graph(&builder.build().unwrap()).unwrap();
		let source = TypeScriptSourceRenderer::render_program(&program)
			.unwrap()
			.remove("generated.ts")
			.unwrap();

		assert!(source.contains("export interface KruntRuntime"));
		assert!(source.contains("runtime: KruntRuntime"));
		assert!(source.contains(
			"runtime.httpRequest(\"POST\", \"https://example.test/items\", operation_1)"
		));
		assert!(source.contains("runtime.log(\"INFO\", operation_2);"));
		assert!(source.contains("runtime.storageRead(\"last-response\")"));
		assert!(!source.contains("declare function krunt_backend_http_request"));
		assert!(source.contains("// krunt:source=node:20"));
	}

	#[test]
	fn renders_typed_structured_control_flow_from_execution_ports() {
		let mut builder = IRBuilder::new();
		let nodes = vec![
			IRNode {
				id: krunt_ir::IrNodeId(1),
				op: IROp::ENTRY {
					name: OperationId::new("start").unwrap(),
				},
				input_types: Vec::new(),
				output_types: Vec::new(),
				async_kind: AsyncKind::SYNC,
				effects: Vec::new(),
				attributes: BTreeMap::new(),
				returns: false,
				source: SourceMap::node(GraphNodeId(10)),
			},
			IRNode {
				id: krunt_ir::IrNodeId(2),
				op: IROp::OPERATION {
					name: OperationId::new("predicate").unwrap(),
				},
				input_types: Vec::new(),
				output_types: vec![TypeId::new("Boolean").unwrap()],
				async_kind: AsyncKind::SYNC,
				effects: Vec::new(),
				attributes: BTreeMap::new(),
				returns: false,
				source: SourceMap::node(GraphNodeId(20)),
			},
			IRNode {
				id: krunt_ir::IrNodeId(3),
				op: IROp::BRANCH,
				input_types: vec![TypeId::new("Boolean").unwrap()],
				output_types: Vec::new(),
				async_kind: AsyncKind::SYNC,
				effects: Vec::new(),
				attributes: BTreeMap::new(),
				returns: false,
				source: SourceMap::node(GraphNodeId(30)),
			},
			IRNode {
				id: krunt_ir::IrNodeId(4),
				op: IROp::OPERATION {
					name: OperationId::new("when_true").unwrap(),
				},
				input_types: Vec::new(),
				output_types: vec![TypeId::new("String").unwrap()],
				async_kind: AsyncKind::SYNC,
				effects: Vec::new(),
				attributes: BTreeMap::new(),
				returns: false,
				source: SourceMap::node(GraphNodeId(40)),
			},
			IRNode {
				id: krunt_ir::IrNodeId(5),
				op: IROp::OPERATION {
					name: OperationId::new("when_false").unwrap(),
				},
				input_types: Vec::new(),
				output_types: vec![TypeId::new("String").unwrap()],
				async_kind: AsyncKind::SYNC,
				effects: Vec::new(),
				attributes: BTreeMap::new(),
				returns: false,
				source: SourceMap::node(GraphNodeId(50)),
			},
			IRNode {
				id: krunt_ir::IrNodeId(6),
				op: IROp::MERGE,
				input_types: Vec::new(),
				output_types: Vec::new(),
				async_kind: AsyncKind::SYNC,
				effects: Vec::new(),
				attributes: BTreeMap::new(),
				returns: false,
				source: SourceMap::node(GraphNodeId(60)),
			},
			IRNode {
				id: krunt_ir::IrNodeId(7),
				op: IROp::OPERATION {
					name: OperationId::new("after_merge").unwrap(),
				},
				input_types: Vec::new(),
				output_types: Vec::new(),
				async_kind: AsyncKind::SYNC,
				effects: Vec::new(),
				attributes: BTreeMap::new(),
				returns: false,
				source: SourceMap::node(GraphNodeId(70)),
			},
			IRNode {
				id: krunt_ir::IrNodeId(8),
				op: IROp::LOOP,
				input_types: vec![TypeId::new("Boolean").unwrap()],
				output_types: Vec::new(),
				async_kind: AsyncKind::SYNC,
				effects: Vec::new(),
				attributes: BTreeMap::new(),
				returns: false,
				source: SourceMap::node(GraphNodeId(80)),
			},
			IRNode {
				id: krunt_ir::IrNodeId(9),
				op: IROp::OPERATION {
					name: OperationId::new("body").unwrap(),
				},
				input_types: Vec::new(),
				output_types: Vec::new(),
				async_kind: AsyncKind::SYNC,
				effects: Vec::new(),
				attributes: BTreeMap::new(),
				returns: false,
				source: SourceMap::node(GraphNodeId(90)),
			},
			IRNode {
				id: krunt_ir::IrNodeId(10),
				op: IROp::OPERATION {
					name: OperationId::new("done").unwrap(),
				},
				input_types: Vec::new(),
				output_types: Vec::new(),
				async_kind: AsyncKind::SYNC,
				effects: Vec::new(),
				attributes: BTreeMap::new(),
				returns: false,
				source: SourceMap::node(GraphNodeId(100)),
			},
		];
		for node in nodes {
			builder.add_node(node).unwrap();
		}
		for (from, from_index, to, to_index) in [
			(1, 0, 2, 0),
			(2, 0, 3, 0),
			(3, 0, 4, 0),
			(3, 1, 5, 0),
			(4, 0, 6, 0),
			(5, 0, 6, 1),
			(6, 0, 7, 0),
			(7, 0, 8, 0),
			(8, 0, 9, 0),
			(9, 0, 8, 1),
			(8, 1, 10, 0),
		] {
			builder
				.add_control_edge(ControlEdge {
					from: PortRef {
						node: krunt_ir::IrNodeId(from),
						index: from_index,
					},
					to: PortRef {
						node: krunt_ir::IrNodeId(to),
						index: to_index,
					},
				})
				.unwrap();
		}
		for (from, to) in [(2, 3), (2, 8)] {
			builder
				.add_data_edge(DataEdge {
					from: PortRef {
						node: krunt_ir::IrNodeId(from),
						index: 0,
					},
					to: PortRef {
						node: krunt_ir::IrNodeId(to),
						index: 0,
					},
				})
				.unwrap();
		}

		let source = TypeScriptSourceRenderer::render_program(
			&TypeScriptBackend::lower_graph(&builder.build().unwrap()).unwrap(),
		)
		.unwrap()
		.remove("generated.ts")
		.unwrap();

		assert!(source.contains("let operation_2: Boolean = undefined as unknown as Boolean;"));
		assert!(source.contains("control = operation_2 ? 4 : 5;"));
		assert!(source.contains("control = operation_2 ? 9 : 10;"));
		assert!(source.contains("control = 8;"));
		assert!(!source.contains(": unknown = undefined"));
	}

	#[test]
	fn renders_non_recursive_subgraph_operations_as_functions() {
		let mut builder = IRBuilder::new();
		for (id, operation) in [
			(
				1,
				IROp::ENTRY {
					name: OperationId::new("start").unwrap(),
				},
			),
			(
				2,
				IROp::SUBGRAPH {
					id: krunt_ir::GraphSubgraphId(9),
				},
			),
			(
				3,
				IROp::OPERATION {
					name: OperationId::new("body").unwrap(),
				},
			),
		] {
			builder
				.add_node(IRNode {
					id: krunt_ir::IrNodeId(id),
					op: operation,
					input_types: Vec::new(),
					output_types: Vec::new(),
					async_kind: AsyncKind::ASYNC,
					effects: Vec::new(),
					attributes: BTreeMap::new(),
					returns: false,
					source: SourceMap::node(GraphNodeId(id as u64 * 10)),
				})
				.unwrap();
		}
		builder
			.add_control_edge(ControlEdge {
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
		builder
			.add_subgraph(krunt_ir::IRSubgraph {
				id: krunt_ir::GraphSubgraphId(9),
				nodes: vec![krunt_ir::IrNodeId(3)],
				inputs: vec![krunt_ir::IRSubgraphBoundary {
					port: krunt_ir::GraphPortId(91),
					target: PortRef {
						node: krunt_ir::IrNodeId(3),
						index: 0,
					},
				}],
				outputs: vec![krunt_ir::IRSubgraphBoundary {
					port: krunt_ir::GraphPortId(92),
					target: PortRef {
						node: krunt_ir::IrNodeId(3),
						index: 0,
					},
				}],
			})
			.unwrap();

		let program = TypeScriptBackend::lower_graph(&builder.build().unwrap()).unwrap();
		let source = TypeScriptSourceRenderer::render_program(&program)
			.unwrap()
			.remove("generated.ts")
			.unwrap();
		assert!(source.contains("await subgraph_9();"));
		assert!(source.contains("export async function subgraph_9(): Promise<void>"));
	}
}
