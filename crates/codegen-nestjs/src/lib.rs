//! NestJS framework backend for Krunt.
//!
//! This backend consumes TypeScript language IR, lowers it into a versioned
//! NestJS framework semantic model, and produces a deterministic project.
//! Runtime-backed operations use a generated adapter so emitted code remains
//! executable while persistence-specific infrastructure stays explicit.

use krunt_codegen_typescript::{
	TypeScriptFunction, TypeScriptOperation, TypeScriptOperationKind, TypeScriptProgram,
	TypeScriptRenderError, TypeScriptSourceRenderer, TypeScriptType,
};
use krunt_framework_backend::{FrameworkBackend, GeneratedProject, ProjectError};
use krunt_ir::GraphElementId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// Version of the NestJS framework semantic IR.
pub const NESTJS_PROGRAM_VERSION: u32 = 4;

/// Framework-level semantic model consumed by NestJS project rendering.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct NestJsProgram {
	version: u32,
	fallback_execution_name: String,
	requires_runtime: bool,
	entities: Vec<NestJsEntity>,
	repositories: Vec<NestJsRepository>,
	routes: Vec<NestJsRoute>,
}

/// Response semantics preserved by the NestJS framework lowering boundary.
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum NestJsResponseKind {
	SUCCESS,
	ERROR,
}

impl NestJsProgram {
	pub const CURRENT_VERSION: u32 = NESTJS_PROGRAM_VERSION;

	pub fn version(&self) -> u32 {
		self.version
	}

	pub fn fallback_execution_name(&self) -> &str {
		&self.fallback_execution_name
	}

	pub fn requires_runtime(&self) -> bool {
		self.requires_runtime
	}

	pub fn entities(&self) -> &[NestJsEntity] {
		&self.entities
	}

	pub fn repositories(&self) -> &[NestJsRepository] {
		&self.repositories
	}

	pub fn routes(&self) -> &[NestJsRoute] {
		&self.routes
	}
}

/// One Backend DSL entity declaration preserved by the language backend.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct NestJsEntity {
	id: u32,
	name: String,
	source: String,
	fields: Vec<NestJsEntityField>,
}

impl NestJsEntity {
	pub fn id(&self) -> u32 {
		self.id
	}

	pub fn name(&self) -> &str {
		&self.name
	}

	pub fn source(&self) -> &str {
		&self.source
	}

	pub fn fields(&self) -> &[NestJsEntityField] {
		&self.fields
	}
}

/// One scalar field in a generated NestJS entity model.
#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum NestJsEntityType {
	SCALAR { type_name: String },
	ARRAY { items: Box<NestJsEntityType> },
	RECORD { fields: Vec<NestJsEntityField> },
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct NestJsEntityField {
	name: String,
	type_name: String,
	type_schema: NestJsEntityType,
	required: bool,
}

impl NestJsEntityField {
	pub fn name(&self) -> &str {
		&self.name
	}

	pub fn type_name(&self) -> &str {
		&self.type_name
	}

	pub fn type_schema(&self) -> &NestJsEntityType {
		&self.type_schema
	}

	pub fn required(&self) -> bool {
		self.required
	}
}

/// The kind of persistence method generated for a NestJS repository.
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum NestJsRepositoryMethodKind {
	MUTATION,
	QUERY,
}

/// One query or mutation method exposed by a generated repository provider.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct NestJsRepositoryMethod {
	operation_id: u32,
	name: String,
	kind: NestJsRepositoryMethodKind,
	input_type: String,
	output_type: String,
	source: String,
}

impl NestJsRepositoryMethod {
	pub fn operation_id(&self) -> u32 {
		self.operation_id
	}

	pub fn name(&self) -> &str {
		&self.name
	}

	pub fn kind(&self) -> NestJsRepositoryMethodKind {
		self.kind
	}

	pub fn input_type(&self) -> &str {
		&self.input_type
	}

	pub fn output_type(&self) -> &str {
		&self.output_type
	}

	pub fn source(&self) -> &str {
		&self.source
	}
}

/// One repository provider bound to a declared Backend DSL entity.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct NestJsRepository {
	id: u32,
	name: String,
	entity: String,
	source: String,
	methods: Vec<NestJsRepositoryMethod>,
}

impl NestJsRepository {
	pub fn id(&self) -> u32 {
		self.id
	}

	pub fn name(&self) -> &str {
		&self.name
	}

	pub fn entity(&self) -> &str {
		&self.entity
	}

	pub fn source(&self) -> &str {
		&self.source
	}

	pub fn methods(&self) -> &[NestJsRepositoryMethod] {
		&self.methods
	}
}

/// One framework-level HTTP route binding.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct NestJsRoute {
	operation_id: u32,
	method: String,
	path: String,
	source: String,
	execution_name: String,
	status_code: u16,
	response_kind: NestJsResponseKind,
	error_message: Option<String>,
	request_entity: Option<String>,
	requires_runtime: bool,
}

impl NestJsRoute {
	pub fn operation_id(&self) -> u32 {
		self.operation_id
	}

	pub fn method(&self) -> &str {
		&self.method
	}

	pub fn path(&self) -> &str {
		&self.path
	}

	pub fn source(&self) -> &str {
		&self.source
	}

	pub fn execution_name(&self) -> &str {
		&self.execution_name
	}

	pub fn status_code(&self) -> u16 {
		self.status_code
	}

	pub fn response_kind(&self) -> NestJsResponseKind {
		self.response_kind
	}

	pub fn error_message(&self) -> Option<&str> {
		self.error_message.as_deref()
	}

	pub fn request_entity(&self) -> Option<&str> {
		self.request_entity.as_deref()
	}

	pub fn requires_runtime(&self) -> bool {
		self.requires_runtime
	}
}

/// The first NestJS backend implementation for Krunt.
#[derive(Clone, Copy, Debug, Default)]
pub struct NestJsBackend;

impl NestJsBackend {
	pub const BACKEND_ID: &'static str = "nestjs";
	pub const LANGUAGE_BACKEND_ID: &'static str = "typescript";

	pub fn generate_project(
		program: &TypeScriptProgram,
	) -> Result<GeneratedProject, NestJsGenerationError> {
		<Self as FrameworkBackend>::generate(&Self, program)
	}

	/// Lower TypeScript language IR into the NestJS framework semantic IR.
	pub fn lower_program(
		program: &TypeScriptProgram,
	) -> Result<NestJsProgram, NestJsGenerationError> {
		lower_nestjs_program(program)
	}
}

impl FrameworkBackend for NestJsBackend {
	type Program = TypeScriptProgram;
	type Error = NestJsGenerationError;

	fn backend_id(&self) -> &'static str {
		Self::BACKEND_ID
	}

	fn generate(&self, program: &Self::Program) -> Result<GeneratedProject, Self::Error> {
		let framework_program = lower_nestjs_program(program)?;

		let mut files = Vec::new();
		for (path, content) in TypeScriptSourceRenderer::render_program(program)
			.map_err(NestJsGenerationError::TYPESCRIPT_RENDER)?
		{
			let content = if path == "generated.ts" {
				prepend_entity_type_imports(content, &framework_program)
			} else {
				content
			};
			files.push((format!("src/{path}"), content));
		}
		files.extend(static_project_files(&framework_program));
		GeneratedProject::try_new(files).map_err(NestJsGenerationError::PROJECT)
	}
}

/// Errors raised while generating a NestJS project.
#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NestJsGenerationError {
	MISSING_EXECUTE_FUNCTION,
	INVALID_ENTITY_FIELDS {
		entity: String,
	},
	INVALID_ENTITY_FIELD_NAME {
		entity: String,
		field: String,
	},
	UNSUPPORTED_ENTITY_FIELD_TYPE {
		entity: String,
		field: String,
		type_name: String,
	},
	REQUEST_ENTITY_NOT_FOUND {
		entity: String,
	},
	UNSUPPORTED_RESPONSE_KIND {
		kind: String,
	},
	TYPESCRIPT_RENDER(TypeScriptRenderError),
	PROJECT(ProjectError),
}

impl fmt::Display for NestJsGenerationError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::MISSING_EXECUTE_FUNCTION => {
				write!(f, "TypeScript program does not expose an execute function")
			}
			Self::INVALID_ENTITY_FIELDS { entity } => {
				write!(f, "entity '{entity}' has invalid field metadata")
			}
			Self::INVALID_ENTITY_FIELD_NAME { entity, field } => {
				write!(f, "entity '{entity}' has invalid field name '{field}'")
			}
			Self::UNSUPPORTED_ENTITY_FIELD_TYPE {
				entity,
				field,
				type_name,
			} => write!(
				f,
				"entity '{entity}' field '{field}' has unsupported type '{type_name}'"
			),
			Self::REQUEST_ENTITY_NOT_FOUND { entity } => {
				write!(f, "request entity '{entity}' is not declared")
			}
			Self::UNSUPPORTED_RESPONSE_KIND { kind } => {
				write!(
					f,
					"unsupported response kind '{kind}' in TypeScript program"
				)
			}
			Self::TYPESCRIPT_RENDER(error) => write!(f, "TypeScript rendering failed: {error}"),
			Self::PROJECT(error) => write!(f, "generated project is invalid: {error}"),
		}
	}
}

impl std::error::Error for NestJsGenerationError {}

fn execution_function_name(program: &TypeScriptProgram) -> Option<String> {
	let mut names = program
		.modules()
		.iter()
		.flat_map(|module| module.functions())
		.map(|function| function.name().to_string())
		.filter(|name| name == "execute" || name.starts_with("execute_"))
		.collect::<Vec<_>>();
	names.sort();
	names.into_iter().next()
}

fn lower_nestjs_program(
	program: &TypeScriptProgram,
) -> Result<NestJsProgram, NestJsGenerationError> {
	let Some(fallback_execution_name) = execution_function_name(program) else {
		return Err(NestJsGenerationError::MISSING_EXECUTE_FUNCTION);
	};
	let entities = lower_entities(program)?;
	let repositories = lower_repositories(program);
	let routes = lower_routes(program, &entities)?;
	Ok(NestJsProgram {
		version: NestJsProgram::CURRENT_VERSION,
		fallback_execution_name,
		requires_runtime: program
			.modules()
			.iter()
			.flat_map(|module| module.functions())
			.any(|function| {
				function
					.operations()
					.iter()
					.any(|operation| operation.requires_runtime())
			}),
		entities,
		repositories,
		routes,
	})
}

fn lower_entities(program: &TypeScriptProgram) -> Result<Vec<NestJsEntity>, NestJsGenerationError> {
	let mut entities = program
		.modules()
		.iter()
		.flat_map(|module| module.declarations())
		.map(|declaration| {
			let name = declaration.attributes().get("backend.entity.name").cloned();
			let Some(name) = name else {
				return Ok(None);
			};
			let fields =
				entity_fields(&name, declaration.attributes().get("backend.entity.fields"))?;
			Ok(Some(NestJsEntity {
				id: declaration.id().as_u32(),
				name,
				source: source_marker(declaration.source()),
				fields,
			}))
		})
		.collect::<Result<Vec<_>, NestJsGenerationError>>()?
		.into_iter()
		.flatten()
		.collect::<Vec<_>>();
	entities.sort();
	Ok(entities)
}

fn entity_fields(
	entity: &str,
	serialized: Option<&String>,
) -> Result<Vec<NestJsEntityField>, NestJsGenerationError> {
	let Some(serialized) = serialized else {
		return Ok(Vec::new());
	};
	let fields = serde_json::from_str::<BTreeMap<String, SerializedEntityField>>(serialized)
		.map_err(|_| NestJsGenerationError::INVALID_ENTITY_FIELDS {
			entity: entity.to_string(),
		})?;
	let mut result = Vec::with_capacity(fields.len());
	for (name, field) in fields {
		if !is_typescript_property_identifier(&name) {
			return Err(NestJsGenerationError::INVALID_ENTITY_FIELD_NAME {
				entity: entity.to_string(),
				field: name,
			});
		}
		let (serialized_type, required) = field.into_parts();
		let type_schema = lower_entity_type(entity, &name, serialized_type)?;
		let type_name = entity_type_name(&type_schema);
		result.push(NestJsEntityField {
			name,
			type_name,
			type_schema,
			required,
		});
	}
	Ok(result)
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum SerializedEntityField {
	Legacy(String),
	Schema {
		#[serde(rename = "type")]
		type_schema: SerializedEntityType,
		#[serde(default = "default_required")]
		required: bool,
	},
}

impl SerializedEntityField {
	fn into_parts(self) -> (SerializedEntityType, bool) {
		match self {
			Self::Legacy(type_name) => (SerializedEntityType::SCALAR(type_name), true),
			Self::Schema {
				type_schema,
				required,
			} => (type_schema, required),
		}
	}
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum SerializedEntityType {
	SCALAR(String),
	ARRAY {
		kind: String,
		items: Box<SerializedEntityType>,
	},
	RECORD {
		kind: String,
		fields: BTreeMap<String, SerializedEntityField>,
	},
}

fn lower_entity_type(
	entity: &str,
	field: &str,
	type_schema: SerializedEntityType,
) -> Result<NestJsEntityType, NestJsGenerationError> {
	match type_schema {
		SerializedEntityType::SCALAR(type_name) => {
			if typescript_scalar_type(&type_name).is_none() {
				return Err(NestJsGenerationError::UNSUPPORTED_ENTITY_FIELD_TYPE {
					entity: entity.to_string(),
					field: field.to_string(),
					type_name,
				});
			}
			Ok(NestJsEntityType::SCALAR { type_name })
		}
		SerializedEntityType::ARRAY { kind, items } => {
			if kind != "array" {
				return Err(NestJsGenerationError::INVALID_ENTITY_FIELDS {
					entity: entity.to_string(),
				});
			}
			Ok(NestJsEntityType::ARRAY {
				items: Box::new(lower_entity_type(entity, field, *items)?),
			})
		}
		SerializedEntityType::RECORD { kind, fields } => {
			if kind != "record" {
				return Err(NestJsGenerationError::INVALID_ENTITY_FIELDS {
					entity: entity.to_string(),
				});
			}
			let mut lowered = Vec::with_capacity(fields.len());
			for (name, field_schema) in fields {
				if !is_typescript_property_identifier(&name) {
					return Err(NestJsGenerationError::INVALID_ENTITY_FIELD_NAME {
						entity: entity.to_string(),
						field: format!("{field}.{name}"),
					});
				}
				let (serialized_type, required) = field_schema.into_parts();
				let type_schema =
					lower_entity_type(entity, &format!("{field}.{name}"), serialized_type)?;
				let type_name = entity_type_name(&type_schema);
				lowered.push(NestJsEntityField {
					name,
					type_name,
					type_schema,
					required,
				});
			}
			Ok(NestJsEntityType::RECORD { fields: lowered })
		}
	}
}

fn entity_type_name(type_schema: &NestJsEntityType) -> String {
	match type_schema {
		NestJsEntityType::SCALAR { type_name } => type_name.clone(),
		NestJsEntityType::ARRAY { .. } => "Array".to_string(),
		NestJsEntityType::RECORD { .. } => "Record".to_string(),
	}
}

fn default_required() -> bool {
	true
}

fn lower_repositories(program: &TypeScriptProgram) -> Vec<NestJsRepository> {
	let mut repositories = BTreeMap::<String, NestJsRepository>::new();
	for declaration in program
		.modules()
		.iter()
		.flat_map(|module| module.declarations())
	{
		let Some(name) = declaration.attributes().get("backend.repository.name") else {
			continue;
		};
		let Some(entity) = declaration.attributes().get("backend.repository.entity") else {
			continue;
		};
		repositories.insert(
			name.clone(),
			NestJsRepository {
				id: declaration.id().as_u32(),
				name: name.clone(),
				entity: entity.clone(),
				source: source_marker(declaration.source()),
				methods: Vec::new(),
			},
		);
	}

	for operation in program
		.modules()
		.iter()
		.flat_map(|module| module.functions())
		.flat_map(TypeScriptFunction::operations)
	{
		let Some((kind, name_key, repository_key)) = persistence_operation(operation) else {
			continue;
		};
		let Some(repository_name) = operation.attributes().get(repository_key) else {
			continue;
		};
		let Some(name) = operation.attributes().get(name_key) else {
			continue;
		};
		let Some(repository) = repositories.get_mut(repository_name) else {
			continue;
		};
		repository.methods.push(NestJsRepositoryMethod {
			operation_id: operation.id().as_u32(),
			name: name.clone(),
			kind,
			input_type: operation
				.input_types()
				.first()
				.map(render_typescript_type)
				.unwrap_or_else(|| "unknown".to_string()),
			output_type: operation
				.output_types()
				.first()
				.map(render_typescript_type)
				.unwrap_or_else(|| "unknown".to_string()),
			source: source_marker(operation.source()),
		});
	}

	let mut repositories = repositories.into_values().collect::<Vec<_>>();
	for repository in &mut repositories {
		repository.methods.sort();
		repository.methods.dedup();
	}
	repositories
}

fn render_typescript_type(type_value: &TypeScriptType) -> String {
	match type_value {
		TypeScriptType::ANY => "unknown".to_string(),
		TypeScriptType::NAMED { name } => name.clone(),
	}
}

fn persistence_operation(
	operation: &TypeScriptOperation,
) -> Option<(NestJsRepositoryMethodKind, &'static str, &'static str)> {
	match operation
		.attributes()
		.get("backend.persistence.kind")
		.map(String::as_str)
	{
		Some("QUERY") => Some((
			NestJsRepositoryMethodKind::QUERY,
			"backend.query.name",
			"backend.query.repository",
		)),
		Some("MUTATION") => Some((
			NestJsRepositoryMethodKind::MUTATION,
			"backend.mutation.name",
			"backend.mutation.repository",
		)),
		_ => None,
	}
}

fn lower_routes(
	program: &TypeScriptProgram,
	entities: &[NestJsEntity],
) -> Result<Vec<NestJsRoute>, NestJsGenerationError> {
	let mut routes = Vec::new();
	for module in program.modules() {
		for function in module.functions() {
			let Some(operation) = function.operations().iter().find(|operation| {
				matches!(operation.kind(), TypeScriptOperationKind::ENTRY { .. })
					&& operation.attributes().contains_key("backend.http.method")
					&& operation.attributes().contains_key("backend.http.path")
			}) else {
				continue;
			};
			let return_operation = function
				.operations()
				.iter()
				.find(|operation| matches!(operation.kind(), TypeScriptOperationKind::RETURN));
			let status_code = return_operation
				.and_then(|operation| operation.attributes().get("response.status_code"))
				.and_then(|value| value.parse().ok())
				.unwrap_or(200);
			let response_kind = response_kind(function)?;
			let request_entity = operation
				.attributes()
				.get("backend.http.request_entity")
				.cloned();
			if let Some(entity) = request_entity.as_deref() {
				if !entities.iter().any(|candidate| candidate.name() == entity) {
					return Err(NestJsGenerationError::REQUEST_ENTITY_NOT_FOUND {
						entity: entity.to_string(),
					});
				}
			}
			routes.push(NestJsRoute {
				operation_id: operation.id().as_u32(),
				method: operation
					.attributes()
					.get("backend.http.method")
					.cloned()
					.unwrap_or_default(),
				path: operation
					.attributes()
					.get("backend.http.path")
					.cloned()
					.unwrap_or_default(),
				source: source_marker(operation.source()),
				execution_name: function.name().to_string(),
				status_code,
				error_message: (response_kind == NestJsResponseKind::ERROR)
					.then(|| return_operation)
					.flatten()
					.and_then(|operation| operation.attributes().get("error.message").cloned()),
				response_kind,
				request_entity,
				requires_runtime: function
					.operations()
					.iter()
					.any(|operation| operation.requires_runtime()),
			});
		}
	}
	routes.sort();
	Ok(routes)
}

fn return_operation(function: &TypeScriptFunction) -> Option<&TypeScriptOperation> {
	function
		.operations()
		.iter()
		.find(|operation| matches!(operation.kind(), TypeScriptOperationKind::RETURN))
}

fn response_kind(
	function: &TypeScriptFunction,
) -> Result<NestJsResponseKind, NestJsGenerationError> {
	let value = return_operation(function)
		.and_then(|operation| operation.attributes().get("response.kind"))
		.map(String::as_str);
	match value {
		None | Some("SUCCESS") => Ok(NestJsResponseKind::SUCCESS),
		Some("ERROR") => Ok(NestJsResponseKind::ERROR),
		Some(kind) => Err(NestJsGenerationError::UNSUPPORTED_RESPONSE_KIND {
			kind: kind.to_string(),
		}),
	}
}

fn static_project_files(program: &NestJsProgram) -> Vec<(String, String)> {
	let mut files = vec![
		("src/app.controller.ts".to_string(), app_controller(program)),
		("src/app.module.ts".to_string(), app_module(program)),
		("src/main.ts".to_string(), main_module()),
		("package.json".to_string(), package_json()),
		("README.md".to_string(), project_readme(program)),
		("tsconfig.json".to_string(), tsconfig()),
		("tsconfig.build.json".to_string(), tsconfig_build()),
	];
	for entity in program.entities() {
		files.push((
			format!(
				"src/models/{}.model.ts",
				type_identifier(entity.name(), "Entity")
			),
			entity_model(entity),
		));
	}
	for repository in program.repositories() {
		files.push((
			format!(
				"src/repositories/{}.repository.ts",
				repository_class_name(repository)
			),
			repository_provider(repository),
		));
	}
	if needs_runtime_file(program) {
		files.push(("src/krunt-runtime.ts".to_string(), krunt_runtime(program)));
	}
	files
}

fn prepend_entity_type_imports(mut source: String, program: &NestJsProgram) -> String {
	let imports = program
		.entities()
		.iter()
		.map(|entity| {
			let name = type_identifier(entity.name(), "Entity");
			format!("import type {{ {name} }} from './models/{name}.model';\n",)
		})
		.collect::<String>();
	if imports.is_empty() {
		return source;
	}
	source.insert_str(0, &format!("{imports}\n"));
	source
}

fn needs_runtime_file(program: &NestJsProgram) -> bool {
	program.requires_runtime() || !program.repositories().is_empty()
}

fn app_controller(program: &NestJsProgram) -> String {
	let has_error_routes = program
		.routes()
		.iter()
		.any(|route| route.response_kind() == NestJsResponseKind::ERROR);
	let has_request_validation = program
		.routes()
		.iter()
		.any(|route| route.request_entity().is_some());
	let mut decorators = vec![
		"Controller",
		"Delete",
		"Get",
		"Head",
		"HttpCode",
		"Options",
		"Patch",
		"Post",
		"Put",
	];
	if has_request_validation {
		decorators.push("BadRequestException");
		decorators.push("Body");
	}
	decorators.push("Req");
	if has_error_routes {
		decorators.push("HttpException");
	}
	let execution_imports = if program.routes().is_empty() {
		program.fallback_execution_name().to_string()
	} else {
		program
			.routes()
			.iter()
			.map(NestJsRoute::execution_name)
			.collect::<Vec<_>>()
			.join(", ")
	};
	let runtime_import = if program.requires_runtime() {
		"import { KruntRuntimeService } from './krunt-runtime';\n"
	} else {
		""
	};
	let mut request_validator_imports = program
		.routes()
		.iter()
		.filter_map(|route| route.request_entity())
		.map(|entity| format!("validate{}", type_identifier(entity, "Entity")))
		.collect::<Vec<_>>();
	request_validator_imports.sort();
	request_validator_imports.dedup();
	let request_validator_imports = request_validator_imports
		.iter()
		.map(|function| {
			let entity = function.strip_prefix("validate").unwrap_or("Entity");
			format!("import {{ {function} }} from './models/{entity}.model';\n")
		})
		.collect::<String>();
	let mut source = format!(
		r#"import {{ {} }} from '@nestjs/common';
import {{ {execution_imports} }} from './generated';
{runtime_import}
{request_validator_imports}

@Controller()
export class AppController {{
"#,
		decorators.join(", "),
	);
	if program.requires_runtime() {
		source.push_str("  constructor(private readonly runtime: KruntRuntimeService) {}\n\n");
	}

	if program.routes().is_empty() {
		source.push_str(
			"  @Get()\n  async execute(@Req() request: unknown): Promise<unknown> {\n    return ",
		);
		source.push_str(program.fallback_execution_name());
		if program.requires_runtime() {
			source.push_str("(request, this.runtime);\n  }\n");
		} else {
			source.push_str("(request);\n  }\n");
		}
	} else {
		for route in program.routes() {
			source.push_str("  // krunt:source=");
			source.push_str(route.source());
			source.push('\n');
			source.push_str("  @HttpCode(");
			source.push_str(&route.status_code().to_string());
			source.push_str(")\n  @");
			source.push_str(method_decorator(route.method()));
			source.push_str("('");
			source.push_str(&escape_single_quoted(route.path()));
			source.push_str("')\n");
			source.push_str("  async handle_");
			source.push_str(&route.operation_id().to_string());
			if route.request_entity().is_some() {
				source.push_str("(@Body() request: unknown): Promise<unknown> {\n");
			} else {
				source.push_str("(@Req() request: unknown): Promise<unknown> {\n");
			}
			let request_expression = if let Some(entity) = route.request_entity() {
				source.push_str("    let validatedRequest: unknown;\n");
				source.push_str("    try {\n      validatedRequest = validate");
				source.push_str(&type_identifier(entity, "Entity"));
				source.push_str("(request);\n    } catch (error) {\n      throw new BadRequestException(error instanceof Error ? error.message : 'Invalid request body');\n    }\n");
				"validatedRequest"
			} else {
				"request"
			};
			let mut invocation = route.execution_name().to_string();
			if route.requires_runtime() {
				invocation.push('(');
				invocation.push_str(request_expression);
				invocation.push_str(", this.runtime)");
			} else {
				invocation.push('(');
				invocation.push_str(request_expression);
				invocation.push(')');
			}
			if route.response_kind() == NestJsResponseKind::ERROR {
				source.push_str("    const result = await ");
				source.push_str(&invocation);
				source.push_str(";\n    throw new HttpException(result ?? '");
				source.push_str(&escape_single_quoted(
					route.error_message().unwrap_or("Request failed"),
				));
				source.push_str("', ");
				source.push_str(&route.status_code().to_string());
				source.push_str(");\n  }\n");
			} else {
				source.push_str("    return ");
				source.push_str(&invocation);
				source.push_str(";\n  }\n");
			}
		}
	}
	source.push_str("}\n");
	source
}

fn method_decorator(method: &str) -> &'static str {
	match method {
		"DELETE" => "Delete",
		"GET" => "Get",
		"HEAD" => "Head",
		"OPTIONS" => "Options",
		"PATCH" => "Patch",
		"POST" => "Post",
		"PUT" => "Put",
		_ => "Get",
	}
}

fn escape_single_quoted(value: &str) -> String {
	value.replace('\\', "\\\\").replace('\'', "\\'")
}

fn source_marker(source: &krunt_ir::SourceMap) -> String {
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

fn entity_model(entity: &NestJsEntity) -> String {
	let name = type_identifier(entity.name(), "Entity");
	let mut source = format!(
		"// krunt:source={}\nexport interface {} {{\n",
		entity.source(),
		name,
	);
	if entity.fields().is_empty() {
		source.push_str("  [key: string]: unknown;\n");
	} else {
		for field in entity.fields() {
			source.push_str("  ");
			source.push_str(field.name());
			if !field.required() {
				source.push('?');
			}
			source.push_str(": ");
			source.push_str(&typescript_entity_type(field.type_schema(), 2));
			source.push_str(";\n");
		}
	}
	source.push_str("}\n\n");
	if !entity.fields().is_empty() {
		source.push_str("export function validate");
		source.push_str(&name);
		source.push_str("(value: unknown): ");
		source.push_str(&name);
		source.push_str(" {\n");
		source.push_str(
		"  if (typeof value !== 'object' || value === null || Array.isArray(value)) {\n    throw new Error('",
	);
		source.push_str(&name);
		source.push_str(" must be an object.');\n  }\n");
		source.push_str("  const record = value as Record<string, unknown>;\n");
		for field in entity.fields() {
			source.push_str("  if (");
			source.push_str(&entity_field_validation(field));
			source.push_str(") {\n    throw new Error('");
			source.push_str(&name);
			source.push('.');
			source.push_str(field.name());
			source.push_str(" must be a ");
			source.push_str(entity_field_error_type(field.type_schema()));
			source.push_str(".');\n  }\n");
		}
		source.push_str("  return {\n");
		for field in entity.fields() {
			source.push_str("    ");
			source.push_str(field.name());
			source.push_str(": record.");
			source.push_str(field.name());
			source.push_str(" as ");
			source.push_str(&typescript_entity_type(field.type_schema(), 2));
			if !field.required() {
				source.push_str(" | undefined");
			}
			source.push_str(",\n");
		}
		source.push_str("  };\n}\n");
	}
	source
}

fn typescript_scalar_type(type_name: &str) -> Option<&'static str> {
	match type_name.to_ascii_lowercase().as_str() {
		"string" => Some("string"),
		"boolean" => Some("boolean"),
		"integer" | "float" | "number" => Some("number"),
		"null" => Some("null"),
		"any" => Some("unknown"),
		_ => None,
	}
}

fn typescript_entity_type(type_schema: &NestJsEntityType, indent: usize) -> String {
	match type_schema {
		NestJsEntityType::SCALAR { type_name } => typescript_scalar_type(type_name)
			.unwrap_or("unknown")
			.to_string(),
		NestJsEntityType::ARRAY { items } => {
			format!("Array<{}>", typescript_entity_type(items, indent))
		}
		NestJsEntityType::RECORD { fields } => {
			let padding = "  ".repeat(indent);
			let field_padding = "  ".repeat(indent + 1);
			let mut source = String::from("{\n");
			for field in fields {
				source.push_str(&field_padding);
				source.push_str(field.name());
				if !field.required() {
					source.push('?');
				}
				source.push_str(": ");
				source.push_str(&typescript_entity_type(field.type_schema(), indent + 1));
				source.push_str(";\n");
			}
			source.push_str(&padding);
			source.push('}');
			source
		}
	}
}

fn entity_field_error_type(type_schema: &NestJsEntityType) -> &'static str {
	match type_schema {
		NestJsEntityType::SCALAR { type_name } => match type_name.to_ascii_lowercase().as_str() {
			"string" => "string",
			"boolean" => "boolean",
			"integer" => "integer",
			"float" | "number" => "number",
			"null" => "null",
			"any" => "value",
			_ => "supported value",
		},
		NestJsEntityType::ARRAY { .. } => "array",
		NestJsEntityType::RECORD { .. } => "object",
	}
}

fn entity_field_validation(field: &NestJsEntityField) -> String {
	let access = format!("record.{}", field.name());
	let presence = format!(
		"Object.prototype.hasOwnProperty.call(record, '{}')",
		field.name()
	);
	let type_check = entity_type_validation(&access, field.type_schema());

	match field.required() {
		true => format!("!{presence} || {type_check}"),
		false => format!("{presence} && ({type_check})"),
	}
}

fn entity_type_validation(access: &str, type_schema: &NestJsEntityType) -> String {
	match type_schema {
		NestJsEntityType::SCALAR { type_name } => match type_name.to_ascii_lowercase().as_str() {
			"string" => format!("typeof {access} !== 'string'"),
			"boolean" => format!("typeof {access} !== 'boolean'"),
			"integer" => format!("typeof {access} !== 'number' || !Number.isInteger({access})"),
			"float" | "number" => {
				format!("typeof {access} !== 'number' || !Number.isFinite({access})")
			}
			"null" => format!("{access} !== null"),
			"any" => "false".to_string(),
			_ => "true".to_string(),
		},
		NestJsEntityType::ARRAY { items } => format!(
			"!Array.isArray({access}) || !{access}.every((item) => !({}))",
			entity_type_validation("item", items)
		),
		NestJsEntityType::RECORD { fields } => {
			let mut checks = vec![format!(
				"typeof {access} !== 'object' || {access} === null || Array.isArray({access})"
			)];
			for field in fields {
				let field_access = entity_property_access(access, field.name());
				let field_presence = format!(
					"Object.prototype.hasOwnProperty.call({access}, '{}')",
					field.name()
				);
				let field_validation = entity_type_validation(&field_access, field.type_schema());
				if field.required() {
					checks.push(format!("!{field_presence} || {field_validation}"));
				} else {
					checks.push(format!("{field_presence} && ({field_validation})"));
				}
			}
			checks.join(" || ")
		}
	}
}

fn entity_property_access(value: &str, property: &str) -> String {
	format!("({value} as Record<string, unknown>).{property}")
}

fn is_typescript_property_identifier(value: &str) -> bool {
	let mut characters = value.chars();
	let Some(first) = characters.next() else {
		return false;
	};
	(first.is_ascii_alphabetic() || first == '_' || first == '$')
		&& characters.all(|character| {
			character.is_ascii_alphanumeric() || character == '_' || character == '$'
		})
}

fn repository_provider(repository: &NestJsRepository) -> String {
	let class_name = repository_class_name(repository);
	let entity_name = type_identifier(repository.entity(), "Entity");
	let mut source = String::new();
	source.push_str("// krunt:source=");
	source.push_str(repository.source());
	source.push('\n');
	source.push_str("import type { ");
	source.push_str(&entity_name);
	source.push_str(" } from '../models/");
	source.push_str(&entity_name);
	source.push_str(".model';\n");
	source.push_str(
		"import type { KruntRepository, KruntRepositoryOperation } from '../krunt-runtime';\n\n",
	);
	source.push_str("export class ");
	source.push_str(&class_name);
	source.push_str(" implements KruntRepository {\n");
	source.push_str(
		"  async execute(kind: KruntRepositoryOperation, name: string, value: unknown): Promise<unknown> {\n",
	);
	source.push_str("    switch (`${kind}:${name}`) {\n");
	for method in repository.methods() {
		source.push_str("      case '");
		source.push_str(match method.kind() {
			NestJsRepositoryMethodKind::QUERY => "QUERY",
			NestJsRepositoryMethodKind::MUTATION => "MUTATION",
		});
		source.push(':');
		source.push_str(&escape_single_quoted(method.name()));
		source.push_str("': return this.");
		source.push_str(&method_identifier(method.name(), "operation"));
		if method.kind() == NestJsRepositoryMethodKind::MUTATION {
			source.push_str("(value);");
		} else {
			source.push_str("();");
		}
		source.push('\n');
	}
	source.push_str("      default: throw new Error(`Unknown repository operation ${name}`);\n");
	source.push_str("    }\n  }\n\n");
	for method in repository.methods() {
		source.push_str("  // krunt:source=");
		source.push_str(method.source());
		source.push('\n');
		source.push_str("  async ");
		source.push_str(&method_identifier(method.name(), "operation"));
		if method.kind() == NestJsRepositoryMethodKind::MUTATION {
			source.push_str("(value: ");
			source.push_str(method.input_type());
			source.push_str("): Promise<");
			source.push_str(method.output_type());
			source.push_str("> {\n");
		} else {
			source.push_str("(): Promise<");
			source.push_str(method.output_type());
			source.push_str("> {\n");
		}
		source.push_str("    throw new Error('Repository method ");
		source.push_str(&escape_single_quoted(repository.name()));
		source.push('.');
		source.push_str(&escape_single_quoted(method.name()));
		source.push_str(" requires a persistence adapter.');\n  }\n\n");
	}
	source.push_str("}\n");
	source
}

fn repository_class_name(repository: &NestJsRepository) -> String {
	format!(
		"{}Repository",
		type_identifier(repository.name(), "Repository")
	)
}

fn type_identifier(value: &str, fallback: &str) -> String {
	let identifier = sanitize_identifier(value, fallback);
	let mut characters = identifier.chars();
	let Some(first) = characters.next() else {
		return fallback.to_string();
	};
	let mut result = first.to_ascii_uppercase().to_string();
	result.extend(characters);
	if is_typescript_keyword(&result) {
		result.insert(0, '_');
	}
	result
}

fn method_identifier(value: &str, fallback: &str) -> String {
	let identifier = sanitize_identifier(value, fallback);
	let mut characters = identifier.chars();
	let Some(first) = characters.next() else {
		return fallback.to_string();
	};
	let mut identifier = first.to_ascii_lowercase().to_string();
	identifier.extend(characters);
	if is_typescript_keyword(&identifier) {
		format!("_{identifier}")
	} else {
		identifier
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

fn is_typescript_keyword(value: &str) -> bool {
	matches!(
		value,
		"break"
			| "case" | "catch"
			| "class" | "const"
			| "continue"
			| "debugger"
			| "default"
			| "delete"
			| "do" | "else"
			| "enum" | "export"
			| "extends"
			| "false" | "finally"
			| "for" | "function"
			| "if" | "import"
			| "in" | "instanceof"
			| "new" | "null"
			| "return"
			| "super" | "switch"
			| "this" | "throw"
			| "true" | "try"
			| "typeof"
			| "var" | "void"
			| "while" | "with"
			| "yield" | "implements"
			| "interface"
			| "let" | "package"
			| "private"
			| "protected"
			| "public"
			| "static"
			| "await" | "abstract"
			| "as" | "asserts"
			| "any" | "boolean"
			| "constructor"
			| "declare"
			| "get" | "infer"
			| "intrinsic"
			| "is" | "keyof"
			| "module"
			| "namespace"
			| "never" | "number"
			| "object"
			| "readonly"
			| "require"
			| "set" | "string"
			| "symbol"
			| "type" | "undefined"
			| "unique"
			| "unknown"
			| "from" | "of"
	)
}

fn app_module(program: &NestJsProgram) -> String {
	let mut source = "import { Module } from '@nestjs/common';\nimport { AppController } from './app.controller';\n".to_string();
	if program.requires_runtime() {
		source.push_str("import { KruntRuntimeService } from './krunt-runtime';\n");
	}
	for repository in program.repositories() {
		source.push_str("import { ");
		source.push_str(&repository_class_name(repository));
		source.push_str(" } from './repositories/");
		source.push_str(&repository_class_name(repository));
		source.push_str(".repository';\n");
	}
	source.push_str("\n@Module({\n  controllers: [AppController],\n");
	let mut providers = Vec::new();
	if program.requires_runtime() {
		providers.push("KruntRuntimeService".to_string());
	}
	providers.extend(program.repositories().iter().map(repository_class_name));
	if !providers.is_empty() {
		source.push_str("  providers: [");
		source.push_str(&providers.join(", "));
		source.push_str("],\n");
	}
	source.push_str("})\nexport class AppModule {}\n");
	source
}

fn main_module() -> String {
	r#"import { NestFactory } from '@nestjs/core';
import { AppModule } from './app.module';

async function bootstrap(): Promise<void> {
  const app = await NestFactory.create(AppModule);
  const port = Number(process.env.PORT ?? 3000);
  await app.listen(port);
}

void bootstrap();
"#
	.to_string()
}

fn package_json() -> String {
	r#"{
  "name": "krunt-generated-backend",
  "version": "0.1.0",
  "private": true,
  "scripts": {
    "build": "tsc -p tsconfig.build.json",
    "start": "node dist/main.js",
    "start:dev": "nest start --watch"
  },
  "dependencies": {
    "@nestjs/common": "11.0.11",
    "@nestjs/core": "11.0.11",
    "reflect-metadata": "0.2.2",
    "rxjs": "7.8.2"
  },
  "devDependencies": {
    "@nestjs/cli": "11.0.0",
    "@types/node": "22.13.4",
    "typescript": "5.7.3"
  }
}
"#
	.to_string()
}

fn project_readme(program: &NestJsProgram) -> String {
	let runtime_note = if needs_runtime_file(program) {
		"\n`src/krunt-runtime.ts` is the generated runtime adapter. Repository providers are generated as readable persistence seams; replace their stub methods with the application's database integrations before production deployment.\n"
	} else {
		""
	};
	let mut readme = r#"# Generated Krunt NestJS backend

This project was generated by Krunt from validated TypeScript language IR.
Generated files are user-owned artifacts and can be built independently of the
Krunt editor.

## Commands

```sh
npm install
npm run build
npm start
```

`src/generated.ts` contains source-linked operation markers and one execution
function per HTTP route. Runtime-backed operations receive a deterministic
`KruntRuntime` adapter and retain source-node traceability in the generated
comments. Route handlers preserve the declared success or error HTTP status.
"#
	.to_string();
	readme.push_str(runtime_note);
	readme
}

fn krunt_runtime(program: &NestJsProgram) -> String {
	let mut source = String::new();
	source.push_str("import type { KruntRuntime } from './generated';\n");
	for repository in program.repositories() {
		source.push_str("import { ");
		source.push_str(&repository_class_name(repository));
		source.push_str(" } from './repositories/");
		source.push_str(&repository_class_name(repository));
		source.push_str(".repository';\n");
	}
	source.push_str(
		"\nexport type KruntRepositoryOperation = 'QUERY' | 'MUTATION';\n\nexport interface KruntRepository {\n  execute(kind: KruntRepositoryOperation, name: string, value: unknown): Promise<unknown>;\n}\n\n",
	);
	source.push_str("export class KruntRuntimeService implements KruntRuntime {\n");
	source.push_str("  private readonly storage = new Map<string, unknown>();\n");
	if !program.repositories().is_empty() {
		source.push_str("\n  constructor(\n");
		for repository in program.repositories() {
			source.push_str("    private readonly ");
			source.push_str(&repository_variable_name(repository));
			source.push_str(": ");
			source.push_str(&repository_class_name(repository));
			source.push_str(",\n");
		}
		source.push_str("  ) {}\n");
		source.push_str("\n  private repository(name: string): KruntRepository | undefined {\n    switch (name) {\n");
		for repository in program.repositories() {
			source.push_str("      case '");
			source.push_str(&escape_single_quoted(repository.name()));
			source.push_str("': return this.");
			source.push_str(&repository_variable_name(repository));
			source.push_str(";\n");
		}
		source.push_str("      default: return undefined;\n    }\n  }\n");
	}
	source.push_str(
		r#"
  async httpRequest(method: string, url: string, request: unknown): Promise<unknown> {
    const response = await fetch(url, {
      method,
      body: request === undefined ? undefined : JSON.stringify(request),
      headers: request === undefined ? undefined : { 'content-type': 'application/json' },
    });
    const text = await response.text();
    if (text.length === 0) {
      return undefined;
    }
    try {
      return JSON.parse(text) as unknown;
    } catch {
      return text;
    }
  }

  async query<TOutput>(name: string, repository: string): Promise<TOutput> {
    const adapter = this.repository(repository);
    if (adapter === undefined) {
      throw new Error(`No repository provider configured for query ${repository}.${name}`);
    }
    return adapter.execute('QUERY', name, undefined) as Promise<TOutput>;
  }

  async mutation<TInput, TOutput>(name: string, repository: string, value: TInput): Promise<TOutput> {
    const adapter = this.repository(repository);
    if (adapter === undefined) {
      throw new Error(`No repository provider configured for mutation ${repository}.${name}`);
    }
    return adapter.execute('MUTATION', name, value) as Promise<TOutput>;
  }

  log(level: string, message: unknown): void {
    switch (level) {
      case 'ERROR':
        console.error(message);
        break;
      case 'WARN':
        console.warn(message);
        break;
      case 'DEBUG':
        console.debug(message);
        break;
      default:
        console.info(message);
    }
  }

  async storageRead(key: string): Promise<unknown> {
    return this.storage.get(key);
  }

  async storageWrite(key: string, value: unknown): Promise<unknown> {
    this.storage.set(key, value);
    return value;
  }
}
"#,
	);
	source
}

fn repository_variable_name(repository: &NestJsRepository) -> String {
	let class_name = repository_class_name(repository);
	let mut characters = class_name.chars();
	let Some(first) = characters.next() else {
		return "repository".to_string();
	};
	let mut result = first.to_ascii_lowercase().to_string();
	result.extend(characters);
	result
}

fn tsconfig() -> String {
	r#"{
  "compilerOptions": {
    "module": "commonjs",
    "declaration": true,
    "removeComments": false,
    "emitDecoratorMetadata": true,
    "experimentalDecorators": true,
    "allowSyntheticDefaultImports": true,
    "target": "ES2021",
    "sourceMap": true,
    "outDir": "./dist",
    "baseUrl": "./",
    "incremental": true,
    "skipLibCheck": true,
    "strict": true,
    "esModuleInterop": true,
    "moduleResolution": "node",
    "types": ["node"]
  }
}
"#
	.to_string()
}

fn tsconfig_build() -> String {
	r#"{
  "extends": "./tsconfig.json",
  "exclude": ["node_modules", "test", "dist", "**/*spec.ts"]
}
"#
	.to_string()
}

#[cfg(test)]
mod tests {
	use super::*;
	use krunt_backend_dsl::{
		BackendDslPlugin, ERROR_RESPONSE_NODE_TYPE, HTTP_REQUEST_TYPE, HTTP_ROUTE_NODE_TYPE,
		SUCCESS_RESPONSE_NODE_TYPE, TRANSFORM_NODE_TYPE, VALIDATION_ERROR_NODE_TYPE,
	};
	use krunt_codegen_typescript::TypeScriptBackend;
	use krunt_framework_backend::GeneratedProject;
	use krunt_graph_model::{
		Edge, EdgeId, EdgeKind, GraphBuilder, GraphId, GraphMetadata, Node, NodeId, NodeTypeId,
		Port, PortDirection, PortId, PortKind, TypeReference,
	};
	use krunt_ir::{
		AsyncKind, ControlEdge, DataEdge, GraphNodeId, IRBuilder, IRNode, IROp, OperationId,
		PortRef, SourceMap, TypeId,
	};
	use krunt_plugin_api::PluginRegistry;
	use serde_json::Value;
	use std::fs;
	use std::process::Command;
	use std::sync::atomic::{AtomicUsize, Ordering};

	static TYPECHECK_DIRECTORY_COUNTER: AtomicUsize = AtomicUsize::new(0);

	fn program() -> TypeScriptProgram {
		let mut builder = IRBuilder::new();
		builder
			.add_node(IRNode {
				id: krunt_ir::IrNodeId(1),
				op: IROp::ENTRY {
					name: OperationId::new("route").unwrap(),
				},
				input_types: Vec::new(),
				output_types: vec![TypeId::new("Request").unwrap()],
				async_kind: krunt_ir::AsyncKind::ASYNC,
				effects: Vec::new(),
				attributes: std::collections::BTreeMap::new(),
				returns: false,
				source: SourceMap::node(GraphNodeId(10)),
			})
			.unwrap();
		let graph = builder.build().unwrap();
		TypeScriptBackend::lower_graph(&graph).unwrap()
	}

	fn runtime_program() -> TypeScriptProgram {
		let mut builder = IRBuilder::new();
		let mut route_attributes = std::collections::BTreeMap::new();
		route_attributes.insert("backend.http.method".to_string(), "GET".to_string());
		route_attributes.insert("backend.http.path".to_string(), "/effects".to_string());
		builder
			.add_node(IRNode {
				id: krunt_ir::IrNodeId(1),
				op: IROp::ENTRY {
					name: OperationId::new("backend.http_route").unwrap(),
				},
				input_types: Vec::new(),
				output_types: vec![TypeId::new("Request").unwrap()],
				async_kind: AsyncKind::ASYNC,
				effects: Vec::new(),
				attributes: route_attributes,
				returns: false,
				source: SourceMap::node(GraphNodeId(10)),
			})
			.unwrap();
		builder
			.add_node(IRNode {
				id: krunt_ir::IrNodeId(2),
				op: IROp::OPERATION {
					name: OperationId::new("backend.http_request").unwrap(),
				},
				input_types: vec![TypeId::new("Request").unwrap()],
				output_types: vec![TypeId::new("Response").unwrap()],
				async_kind: AsyncKind::ASYNC,
				effects: Vec::new(),
				attributes: std::collections::BTreeMap::from([
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
			})
			.unwrap();
		builder
			.add_node(IRNode {
				id: krunt_ir::IrNodeId(3),
				op: IROp::RETURN,
				input_types: vec![TypeId::new("Response").unwrap()],
				output_types: Vec::new(),
				async_kind: AsyncKind::ASYNC,
				effects: Vec::new(),
				attributes: std::collections::BTreeMap::from([
					("response.kind".to_string(), "SUCCESS".to_string()),
					("response.status_code".to_string(), "200".to_string()),
				]),
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
		TypeScriptBackend::lower_graph(&builder.build().unwrap()).unwrap()
	}

	fn persistence_program() -> TypeScriptProgram {
		let mut builder = IRBuilder::new();
		let mut route_attributes = std::collections::BTreeMap::new();
		route_attributes.insert("backend.http.method".to_string(), "POST".to_string());
		route_attributes.insert("backend.http.path".to_string(), "/users".to_string());
		route_attributes.insert(
			"backend.http.request_entity".to_string(),
			"User".to_string(),
		);
		builder
			.add_node(IRNode {
				id: krunt_ir::IrNodeId(1),
				op: IROp::ENTRY {
					name: OperationId::new("backend.http_route").unwrap(),
				},
				input_types: Vec::new(),
				output_types: vec![TypeId::new("Request").unwrap()],
				async_kind: AsyncKind::ASYNC,
				effects: Vec::new(),
				attributes: route_attributes,
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
				attributes: std::collections::BTreeMap::from([
					("backend.persistence.kind".to_string(), "QUERY".to_string()),
					("backend.query.name".to_string(), "ListUsers".to_string()),
					("backend.query.cardinality".to_string(), "many".to_string()),
					("backend.persistence.entity".to_string(), "User".to_string()),
					(
						"backend.persistence.output_type".to_string(),
						"Array<User>".to_string(),
					),
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
				attributes: std::collections::BTreeMap::from([
					("response.kind".to_string(), "SUCCESS".to_string()),
					("response.status_code".to_string(), "200".to_string()),
				]),
				returns: true,
				source: SourceMap::node(GraphNodeId(30)),
			})
			.unwrap();
		builder
			.add_node(IRNode {
				id: krunt_ir::IrNodeId(4),
				op: IROp::DECLARATION {
					name: OperationId::new("backend.entity").unwrap(),
				},
				input_types: Vec::new(),
				output_types: Vec::new(),
				async_kind: AsyncKind::SYNC,
				effects: Vec::new(),
				attributes: std::collections::BTreeMap::from([
					("backend.entity.name".to_string(), "User".to_string()),
					(
						"backend.entity.fields".to_string(),
						"{\"email\":{\"type\":\"String\",\"required\":false},\"id\":{\"type\":\"Integer\",\"required\":true},\"profile\":{\"type\":{\"kind\":\"record\",\"fields\":{\"bio\":{\"type\":\"String\",\"required\":false},\"displayName\":{\"type\":\"String\",\"required\":true}}},\"required\":false},\"tags\":{\"type\":{\"kind\":\"array\",\"items\":\"String\"},\"required\":true}}".to_string(),
					),
				]),
				returns: false,
				source: SourceMap::node(GraphNodeId(40)),
			})
			.unwrap();
		builder
			.add_node(IRNode {
				id: krunt_ir::IrNodeId(5),
				op: IROp::DECLARATION {
					name: OperationId::new("backend.repository").unwrap(),
				},
				input_types: Vec::new(),
				output_types: Vec::new(),
				async_kind: AsyncKind::SYNC,
				effects: Vec::new(),
				attributes: std::collections::BTreeMap::from([
					(
						"backend.repository.name".to_string(),
						"UserRepository".to_string(),
					),
					("backend.repository.entity".to_string(), "User".to_string()),
				]),
				returns: false,
				source: SourceMap::node(GraphNodeId(50)),
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
		TypeScriptBackend::lower_graph(&builder.build().unwrap()).unwrap()
	}

	fn backend_dsl_program() -> TypeScriptProgram {
		backend_dsl_program_with_response(SUCCESS_RESPONSE_NODE_TYPE, 201)
	}

	fn backend_dsl_error_program() -> TypeScriptProgram {
		backend_dsl_program_with_response(ERROR_RESPONSE_NODE_TYPE, 422)
	}

	fn backend_dsl_validation_error_program() -> TypeScriptProgram {
		backend_dsl_program_with_response(VALIDATION_ERROR_NODE_TYPE, 422)
	}

	fn backend_dsl_program_with_response(
		response_node_type: &str,
		status_code: u16,
	) -> TypeScriptProgram {
		let metadata =
			GraphMetadata::new(GraphId::new(1).unwrap(), "backend-v1", "krunt-test").unwrap();
		let mut builder = GraphBuilder::new(metadata);

		let mut route = Node::new(
			NodeId::new(1).unwrap(),
			NodeTypeId::new(HTTP_ROUTE_NODE_TYPE).unwrap(),
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
				.with_type_reference(TypeReference::new(HTTP_REQUEST_TYPE).unwrap()),
			)
			.unwrap();
		route = route
			.with_property("method", Value::String("GET".to_string()))
			.unwrap();
		route = route
			.with_property("path", Value::String("/users".to_string()))
			.unwrap();

		let mut transform = Node::new(
			NodeId::new(2).unwrap(),
			NodeTypeId::new(TRANSFORM_NODE_TYPE).unwrap(),
		);
		transform
			.add_port(Port::new(
				PortId::new(21).unwrap(),
				PortDirection::INPUT,
				PortKind::EXECUTION,
			))
			.unwrap();
		transform
			.add_port(
				Port::new(
					PortId::new(22).unwrap(),
					PortDirection::INPUT,
					PortKind::DATA,
				)
				.with_type_reference(TypeReference::new(HTTP_REQUEST_TYPE).unwrap()),
			)
			.unwrap();
		transform
			.add_port(Port::new(
				PortId::new(23).unwrap(),
				PortDirection::OUTPUT,
				PortKind::EXECUTION,
			))
			.unwrap();
		transform
			.add_port(
				Port::new(
					PortId::new(24).unwrap(),
					PortDirection::OUTPUT,
					PortKind::DATA,
				)
				.with_type_reference(TypeReference::new(HTTP_REQUEST_TYPE).unwrap()),
			)
			.unwrap();

		let mut response = Node::new(
			NodeId::new(3).unwrap(),
			NodeTypeId::new(response_node_type).unwrap(),
		);
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
		if response_node_type != VALIDATION_ERROR_NODE_TYPE {
			response = response
				.with_property(
					"status_code",
					Value::Number(serde_json::Number::from(status_code)),
				)
				.unwrap();
		}

		builder.add_node(route).unwrap();
		builder.add_node(transform).unwrap();
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
		builder
			.add_edge(Edge::new(
				EdgeId::new(33).unwrap(),
				PortId::new(23).unwrap(),
				PortId::new(31).unwrap(),
				EdgeKind::EXECUTION,
			))
			.unwrap();
		builder
			.add_edge(Edge::new(
				EdgeId::new(34).unwrap(),
				PortId::new(24).unwrap(),
				PortId::new(32).unwrap(),
				EdgeKind::DATA,
			))
			.unwrap();
		let document = builder.build().unwrap();

		let mut registry = PluginRegistry::new();
		BackendDslPlugin::register(&mut registry).unwrap();
		let result = krunt_compiler::compile(&document, &registry);
		assert!(
			!result.has_errors(),
			"compiler diagnostics: {:?}",
			result.diagnostics()
		);
		TypeScriptBackend::lower_graph(result.ir().expect("valid graph should lower")).unwrap()
	}

	fn backend_dsl_route_without_response_document() -> krunt_graph_model::GraphDocument {
		let metadata =
			GraphMetadata::new(GraphId::new(2).unwrap(), "backend-v1", "krunt-test").unwrap();
		let mut builder = GraphBuilder::new(metadata);
		let mut route = Node::new(
			NodeId::new(1).unwrap(),
			NodeTypeId::new(HTTP_ROUTE_NODE_TYPE).unwrap(),
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
				.with_type_reference(TypeReference::new(HTTP_REQUEST_TYPE).unwrap()),
			)
			.unwrap();
		route = route
			.with_property("method", Value::String("GET".to_string()))
			.unwrap();
		route = route
			.with_property("path", Value::String("/users".to_string()))
			.unwrap();
		builder.add_node(route).unwrap();
		builder.build().unwrap()
	}

	fn assert_generated_project_typechecks(project: &GeneratedProject, label: &str) {
		let directory = std::env::temp_dir().join(format!(
			"krunt-generated-{label}-{}-{}",
			std::process::id(),
			TYPECHECK_DIRECTORY_COUNTER.fetch_add(1, Ordering::Relaxed)
		));
		fs::create_dir_all(&directory).expect("create generated project typecheck directory");

		for (path, content) in project.files() {
			let file_path = directory.join(path);
			if let Some(parent) = file_path.parent() {
				fs::create_dir_all(parent).expect("create generated project file directory");
			}
			fs::write(file_path, content).expect("write generated project file");
		}
		fs::write(directory.join("typecheck-stubs.d.ts"), TYPECHECK_STUBS)
			.expect("write generated project typecheck stubs");
		fs::write(
			directory.join("tsconfig.typecheck.json"),
			TYPECHECK_TSCONFIG,
		)
		.expect("write generated project typecheck configuration");

		let tsc = std::env::var("KRUNT_TSC").unwrap_or_else(|_| "tsc".to_string());
		let output = Command::new(&tsc)
			.args(["-p", "tsconfig.typecheck.json", "--pretty", "false"])
			.current_dir(&directory)
			.output()
			.unwrap_or_else(|error| {
				panic!(
					"generated project typecheck could not start for {label} using {tsc}: {error}. Set KRUNT_TSC to a TypeScript compiler path."
				)
			});

		if !output.status.success() {
			let stdout = String::from_utf8_lossy(&output.stdout);
			let stderr = String::from_utf8_lossy(&output.stderr);
			panic!(
				"generated project typecheck failed for {label} in {}\nstdout:\n{stdout}\nstderr:\n{stderr}",
				directory.display()
			);
		}

		fs::remove_dir_all(&directory).expect("remove generated project typecheck directory");
	}

	const TYPECHECK_TSCONFIG: &str = r#"{
  "extends": "./tsconfig.json",
  "compilerOptions": {
    "noEmit": true,
    "incremental": false,
    "types": [],
    "lib": ["ES2021", "DOM"]
  },
  "include": ["src/**/*.ts", "typecheck-stubs.d.ts"]
}
"#;

	const TYPECHECK_STUBS: &str = r#"declare module '@nestjs/common' {
  export function Controller(prefix?: string): ClassDecorator;
  export function Body(): ParameterDecorator;
  export function Delete(path?: string): MethodDecorator;
  export function Get(path?: string): MethodDecorator;
  export function Head(path?: string): MethodDecorator;
  export function HttpCode(status: number): MethodDecorator;
  export function Options(path?: string): MethodDecorator;
  export function Patch(path?: string): MethodDecorator;
  export function Post(path?: string): MethodDecorator;
  export function Put(path?: string): MethodDecorator;
  export function Req(): ParameterDecorator;
  export function Module(metadata: unknown): ClassDecorator;
  export class BadRequestException extends Error {
    constructor(response: unknown);
  }
  export class HttpException extends Error {
    constructor(response: unknown, status: number);
  }
}

declare module '@nestjs/core' {
  export const NestFactory: {
    create(module: unknown): Promise<{ listen(port: number): Promise<void> }>;
  };
}

declare const process: {
  env: Record<string, string | undefined>;
};
"#;

	#[test]
	fn generates_a_deterministic_nestjs_project() {
		let first = NestJsBackend::generate_project(&program()).unwrap();
		let second = NestJsBackend::generate_project(&program()).unwrap();

		assert_eq!(first, second);
		assert_eq!(
			first.files().keys().cloned().collect::<Vec<_>>(),
			vec![
				"README.md",
				"package.json",
				"src/app.controller.ts",
				"src/app.module.ts",
				"src/generated.ts",
				"src/main.ts",
				"tsconfig.build.json",
				"tsconfig.json",
			]
		);
		assert!(first
			.file("src/app.controller.ts")
			.unwrap()
			.contains("@Controller()"));
		assert!(first
			.file("src/app.controller.ts")
			.unwrap()
			.contains("async execute(@Req() request: unknown): Promise<unknown>"));
		assert!(first
			.file("src/app.controller.ts")
			.unwrap()
			.contains("return execute(request);"));
		assert!(first
			.file("src/generated.ts")
			.unwrap()
			.contains("krunt:source=node:10"));
		assert!(first.file("package.json").unwrap().contains("@nestjs/core"));
	}

	#[test]
	fn lowers_typescript_into_a_deterministic_nestjs_program() {
		let first = NestJsBackend::lower_program(&backend_dsl_program()).unwrap();
		let second = NestJsBackend::lower_program(&backend_dsl_program()).unwrap();

		assert_eq!(first, second);
		assert_eq!(first.version(), NestJsProgram::CURRENT_VERSION);
		assert_eq!(first.routes().len(), 1);
		assert_eq!(first.fallback_execution_name(), "execute_1");
		assert_eq!(first.routes()[0].operation_id(), 1);
		assert_eq!(first.routes()[0].method(), "GET");
		assert_eq!(first.routes()[0].path(), "/users");
		assert_eq!(first.routes()[0].execution_name(), "execute_1");
		assert_eq!(first.routes()[0].status_code(), 201);
		assert_eq!(
			first.routes()[0].response_kind(),
			NestJsResponseKind::SUCCESS
		);
		assert_eq!(
			serde_json::to_string(&first).unwrap(),
			serde_json::to_string(&second).unwrap()
		);
	}

	#[test]
	fn renders_backend_dsl_http_routes_in_the_controller() {
		let project = NestJsBackend::generate_project(&backend_dsl_program()).unwrap();
		let controller = project.file("src/app.controller.ts").unwrap();

		assert!(controller.contains("// krunt:source=node:1"));
		assert!(controller.contains("@HttpCode(201)"));
		assert!(controller.contains("@Get('/users')"));
		assert!(controller.contains("@Req() request: unknown"));
		assert!(controller.contains("return execute_1(request);"));
		assert!(project
			.file("src/generated.ts")
			.unwrap()
			.contains("export function execute_1(request: unknown = undefined): unknown"));
		assert!(project
			.file("src/generated.ts")
			.unwrap()
			.contains("return operation_2;"));
		assert!(project
			.file("src/generated.ts")
			.unwrap()
			.contains("const operation_2 = operation_1;"));
		assert!(!project
			.file("src/generated.ts")
			.unwrap()
			.contains("declare function krunt_backend_transform"));
		assert!(!project
			.file("src/generated.ts")
			.unwrap()
			.contains("declare function krunt_backend_http_route"));
	}

	#[test]
	fn preserves_backend_dsl_error_responses_in_the_nestjs_program() {
		let program = backend_dsl_error_program();
		let framework_program = NestJsBackend::lower_program(&program).unwrap();

		assert_eq!(framework_program.routes().len(), 1);
		assert_eq!(framework_program.routes()[0].status_code(), 422);
		assert_eq!(
			framework_program.routes()[0].response_kind(),
			NestJsResponseKind::ERROR
		);

		let project = NestJsBackend::generate_project(&program).unwrap();
		assert!(project
			.file("src/app.controller.ts")
			.unwrap()
			.contains("@HttpCode(422)"));
	}

	#[test]
	fn preserves_validation_errors_as_bad_request_responses() {
		let program = backend_dsl_validation_error_program();
		let framework_program = NestJsBackend::lower_program(&program).unwrap();

		assert_eq!(framework_program.routes().len(), 1);
		assert_eq!(framework_program.routes()[0].status_code(), 400);
		assert_eq!(
			framework_program.routes()[0].response_kind(),
			NestJsResponseKind::ERROR
		);
		assert_eq!(
			framework_program.routes()[0].error_message(),
			Some("Validation failed")
		);
		assert!(NestJsBackend::generate_project(&program)
			.unwrap()
			.file("src/app.controller.ts")
			.unwrap()
			.contains("throw new HttpException(result ?? 'Validation failed', 400);"));
	}

	#[test]
	fn generates_a_runtime_adapter_for_effect_backed_routes() {
		let first = NestJsBackend::generate_project(&runtime_program()).unwrap();
		let second = NestJsBackend::generate_project(&runtime_program()).unwrap();

		assert_eq!(first, second);
		assert!(first.file("src/krunt-runtime.ts").is_some());
		assert!(first
			.file("src/generated.ts")
			.unwrap()
			.contains("export interface KruntRuntime"));
		assert!(first.file("src/generated.ts").unwrap().contains(
			"runtime.httpRequest(\"POST\", \"https://example.test/items\", operation_1)"
		));
		assert!(first
			.file("src/app.controller.ts")
			.unwrap()
			.contains("import { KruntRuntimeService } from './krunt-runtime';"));
		assert!(first
			.file("src/app.controller.ts")
			.unwrap()
			.contains("constructor(private readonly runtime: KruntRuntimeService) {}"));
		assert!(first
			.file("src/app.controller.ts")
			.unwrap()
			.contains("return execute_1(request, this.runtime);"));
		assert!(first
			.file("src/app.module.ts")
			.unwrap()
			.contains("providers: [KruntRuntimeService]"));
		assert!(first
			.file("src/krunt-runtime.ts")
			.unwrap()
			.contains("No repository provider configured"));
	}

	#[test]
	fn generates_entity_and_repository_providers_for_persistence_operations() {
		let program = persistence_program();
		let framework_program = NestJsBackend::lower_program(&program).unwrap();
		assert_eq!(framework_program.entities().len(), 1);
		assert_eq!(framework_program.entities()[0].name(), "User");
		assert_eq!(framework_program.entities()[0].fields().len(), 4);
		assert_eq!(framework_program.entities()[0].fields()[0].name(), "email");
		assert_eq!(
			framework_program.entities()[0].fields()[0].type_name(),
			"String"
		);
		assert!(!framework_program.entities()[0].fields()[0].required());
		assert!(framework_program.entities()[0].fields()[1].required());
		assert!(matches!(
			framework_program.entities()[0].fields()[2].type_schema(),
			NestJsEntityType::RECORD { .. }
		));
		assert!(matches!(
			framework_program.entities()[0].fields()[3].type_schema(),
			NestJsEntityType::ARRAY { .. }
		));
		assert_eq!(framework_program.routes()[0].request_entity(), Some("User"));
		assert_eq!(framework_program.repositories().len(), 1);
		assert_eq!(framework_program.repositories()[0].name(), "UserRepository");
		assert_eq!(framework_program.repositories()[0].methods().len(), 1);
		assert_eq!(
			framework_program.repositories()[0].methods()[0].kind(),
			NestJsRepositoryMethodKind::QUERY
		);
		assert_eq!(
			framework_program.repositories()[0].methods()[0].input_type(),
			"unknown"
		);
		assert_eq!(
			framework_program.repositories()[0].methods()[0].output_type(),
			"Array<User>"
		);

		let project = NestJsBackend::generate_project(&program).unwrap();
		assert!(project.file("src/models/User.model.ts").is_some());
		let model = project.file("src/models/User.model.ts").unwrap();
		assert!(model.contains("email?: string;"));
		assert!(model.contains("id: number;"));
		assert!(
			model.contains("profile?: {\n      bio?: string;\n      displayName: string;\n    };")
		);
		assert!(model.contains("tags: Array<string>;"));
		assert!(model.contains("export function validateUser(value: unknown): User"));
		assert!(model.contains("Object.prototype.hasOwnProperty.call(record, 'email') && (typeof record.email !== 'string')"));
		assert!(model.contains("!Array.isArray(record.tags) || !record.tags.every((item) => !(typeof item !== 'string'))"));
		assert!(project
			.file("src/repositories/UserRepositoryRepository.repository.ts")
			.unwrap()
			.contains("async listUsers(): Promise<Array<User>>"));
		assert!(project
			.file("src/app.module.ts")
			.unwrap()
			.contains("providers: [KruntRuntimeService, UserRepositoryRepository]"));
		assert!(project
			.file("src/krunt-runtime.ts")
			.unwrap()
			.contains("case 'UserRepository': return this.userRepositoryRepository;"));
		assert!(project
			.file("src/generated.ts")
			.unwrap()
			.contains("runtime.query<Array<User>>(\"ListUsers\", \"UserRepository\")"));
		let controller = project.file("src/app.controller.ts").unwrap();
		assert!(controller.contains("@Body() request: unknown"));
		assert!(controller.contains("validateUser(request)"));
		assert!(controller.contains("BadRequestException"));
	}

	#[test]
	fn generated_success_and_error_projects_pass_the_quality_gate() {
		let cases = [
			(
				"success",
				backend_dsl_program(),
				"return execute_1(request);",
				"@HttpCode(201)",
			),
			(
				"error",
				backend_dsl_error_program(),
				"throw new HttpException(result ?? 'Request failed', 422);",
				"@HttpCode(422)",
			),
		];

		for (label, program, route_semantics, status_decorator) in cases {
			let project = NestJsBackend::generate_project(&program).unwrap();
			let regenerated = NestJsBackend::generate_project(&program).unwrap();
			assert_eq!(
				project, regenerated,
				"generation must be deterministic: {label}"
			);
			assert_eq!(
				project
					.files()
					.keys()
					.map(String::as_str)
					.collect::<Vec<_>>(),
				vec![
					"README.md",
					"package.json",
					"src/app.controller.ts",
					"src/app.module.ts",
					"src/generated.ts",
					"src/main.ts",
					"tsconfig.build.json",
					"tsconfig.json",
				],
				"golden file tree changed: {label}"
			);
			let controller = project.file("src/app.controller.ts").unwrap();
			assert!(controller.contains("// krunt:source=node:1"));
			assert!(controller.contains(route_semantics));
			assert!(controller.contains(status_decorator));
			assert_generated_project_typechecks(&project, label);
		}

		let persistence = NestJsBackend::generate_project(&persistence_program()).unwrap();
		assert_generated_project_typechecks(&persistence, "typed-request");
	}

	#[test]
	fn compiler_rejects_a_route_without_a_terminal_response() {
		let mut registry = PluginRegistry::new();
		BackendDslPlugin::register(&mut registry).unwrap();
		let result =
			krunt_compiler::compile(&backend_dsl_route_without_response_document(), &registry);

		assert!(result.has_errors());
		assert_eq!(
			result.diagnostics().as_slice()[0].code().as_str(),
			"BACKEND_DSL.HTTP_ROUTE_MISSING_RESPONSE"
		);
		assert!(result.ir().is_none());
	}
}
