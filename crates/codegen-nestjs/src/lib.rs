//! NestJS framework backend for Krunt.
//!
//! This backend consumes TypeScript language IR, lowers it into a versioned
//! NestJS framework semantic model, and produces a deterministic project
//! scaffold. Semantic operation implementations are deliberately not invented
//! here; the Backend DSL and future framework contracts will map those
//! operations to concrete controllers, providers, and repositories.

use krunt_codegen_typescript::{
	TypeScriptOperationKind, TypeScriptProgram, TypeScriptRenderError, TypeScriptSourceRenderer,
};
use krunt_framework_backend::{FrameworkBackend, GeneratedProject, ProjectError};
use krunt_ir::GraphElementId;
use serde::Serialize;
use std::fmt;

/// Version of the NestJS framework semantic IR.
pub const NESTJS_PROGRAM_VERSION: u32 = 1;

/// Framework-level semantic model consumed by NestJS project rendering.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct NestJsProgram {
	version: u32,
	fallback_execution_name: String,
	routes: Vec<NestJsRoute>,
}

impl NestJsProgram {
	pub const CURRENT_VERSION: u32 = NESTJS_PROGRAM_VERSION;

	pub fn version(&self) -> u32 {
		self.version
	}

	pub fn fallback_execution_name(&self) -> &str {
		&self.fallback_execution_name
	}

	pub fn routes(&self) -> &[NestJsRoute] {
		&self.routes
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
	TYPESCRIPT_RENDER(TypeScriptRenderError),
	PROJECT(ProjectError),
}

impl fmt::Display for NestJsGenerationError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::MISSING_EXECUTE_FUNCTION => {
				write!(f, "TypeScript program does not expose an execute function")
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
	Ok(NestJsProgram {
		version: NestJsProgram::CURRENT_VERSION,
		fallback_execution_name,
		routes: lower_routes(program),
	})
}

fn lower_routes(program: &TypeScriptProgram) -> Vec<NestJsRoute> {
	let mut routes = program
		.modules()
		.iter()
		.flat_map(|module| module.functions())
		.filter_map(|function| {
			let operation = function.operations().iter().find(|operation| {
				matches!(operation.kind(), TypeScriptOperationKind::ENTRY { .. })
					&& operation.attributes().contains_key("backend.http.method")
					&& operation.attributes().contains_key("backend.http.path")
			})?;
			let status_code = function
				.operations()
				.iter()
				.find(|operation| matches!(operation.kind(), TypeScriptOperationKind::RETURN))
				.and_then(|operation| operation.attributes().get("response.status_code"))
				.and_then(|value| value.parse().ok())
				.unwrap_or(200);
			Some(NestJsRoute {
				operation_id: operation.id().as_u32(),
				method: operation.attributes().get("backend.http.method")?.clone(),
				path: operation.attributes().get("backend.http.path")?.clone(),
				source: source_marker(operation.source()),
				execution_name: function.name().to_string(),
				status_code,
			})
		})
		.collect::<Vec<_>>();
	routes.sort();
	routes
}

fn static_project_files(program: &NestJsProgram) -> Vec<(String, String)> {
	vec![
		("src/app.controller.ts".to_string(), app_controller(program)),
		("src/app.module.ts".to_string(), app_module()),
		("src/main.ts".to_string(), main_module()),
		("package.json".to_string(), package_json()),
		("README.md".to_string(), project_readme()),
		("tsconfig.json".to_string(), tsconfig()),
		("tsconfig.build.json".to_string(), tsconfig_build()),
	]
}

fn app_controller(program: &NestJsProgram) -> String {
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
	let mut source = format!(
		r#"import {{ Controller, Delete, Get, Head, HttpCode, Options, Patch, Post, Put, Req }} from '@nestjs/common';
import {{ {execution_imports} }} from './generated';

@Controller()
export class AppController {{
"#
	);

	if program.routes().is_empty() {
		source.push_str(
			"  @Get()\n  async execute(@Req() request: unknown): Promise<unknown> {\n    return ",
		);
		source.push_str(program.fallback_execution_name());
		source.push_str("(request);\n  }\n");
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
			source.push_str("(@Req() request: unknown): Promise<unknown> {\n    return ");
			source.push_str(route.execution_name());
			source.push_str("(request);\n  }\n");
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

fn app_module() -> String {
	r#"import { Module } from '@nestjs/common';
import { AppController } from './app.controller';

@Module({
  controllers: [AppController],
})
export class AppModule {}
"#
	.to_string()
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

fn project_readme() -> String {
	r#"# Generated Krunt NestJS backend

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
function per HTTP route. The initial Backend DSL slice generates route-specific
handlers and successful HTTP 200 responses; additional semantic operation
implementations will be added as more Backend DSL nodes are introduced.
"#
	.to_string()
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
		BackendDslPlugin, HTTP_REQUEST_TYPE, HTTP_ROUTE_NODE_TYPE, SUCCESS_RESPONSE_NODE_TYPE,
		TRANSFORM_NODE_TYPE,
	};
	use krunt_codegen_typescript::TypeScriptBackend;
	use krunt_graph_model::{
		Edge, EdgeId, EdgeKind, GraphBuilder, GraphId, GraphMetadata, Node, NodeId, NodeTypeId,
		Port, PortDirection, PortId, PortKind, TypeReference,
	};
	use krunt_ir::{GraphNodeId, IRBuilder, IRNode, IROp, OperationId, SourceMap, TypeId};
	use krunt_plugin_api::PluginRegistry;
	use serde_json::Value;

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

	fn backend_dsl_program() -> TypeScriptProgram {
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
			NodeTypeId::new(SUCCESS_RESPONSE_NODE_TYPE).unwrap(),
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
		response = response
			.with_property("status_code", Value::Number(serde_json::Number::from(201)))
			.unwrap();

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
	fn compiler_rejects_a_route_without_a_success_response() {
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
