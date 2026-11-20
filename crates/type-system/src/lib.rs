//! DSL-independent compile-time types and assignability for Krunt.
//!
//! The type system owns reusable primitive and named type semantics. Backend
//! plugins can compose these types into domain schemas without making the
//! graph model or compiler orchestration depend on a particular DSL.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// A validated type name used by the compiler's initial type relation.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct TypeName(String);

impl TypeName {
	pub fn new(value: impl Into<String>) -> Result<Self, TypeError> {
		let value = value.into();
		if value.trim().is_empty() {
			return Err(TypeError::EMPTY_NAME);
		}
		Ok(Self(value))
	}

	pub fn as_str(&self) -> &str {
		&self.0
	}
}

/// A stable identity for a type variable introduced during inference.
///
/// The compiler derives graph-port variables from stable port identities. The
/// type system intentionally keeps the identity opaque so it remains usable by
/// other frontends and DSLs.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct TypeVariable(u64);

impl TypeVariable {
	pub const fn new(value: u64) -> Self {
		Self(value)
	}

	pub const fn as_u64(self) -> u64 {
		self.0
	}
}

/// A compiler-facing type representation.
#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Type {
	STRING,
	BOOLEAN,
	INTEGER,
	FLOAT,
	NUMBER,
	NULL,
	VOID,
	ARRAY(Box<Type>),
	RECORD(BTreeMap<String, RecordField>),
	SET(Box<Type>),
	MAP(Box<Type>, Box<Type>),
	OPTION(Box<Type>),
	RESULT(Box<Type>, Box<Type>),
	FUTURE(Box<Type>),
	STREAM(Box<Type>),
	UNION(Vec<Type>),
	INTERSECTION(Vec<Type>),
	NAMED(TypeName),
	VARIABLE(TypeVariable),
	ANY,
}

/// One field in a structural record type.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RecordField {
	type_value: Type,
	required: bool,
}

impl RecordField {
	pub fn new(type_value: Type, required: bool) -> Self {
		Self {
			type_value,
			required,
		}
	}

	pub fn type_value(&self) -> &Type {
		&self.type_value
	}

	pub fn required(&self) -> bool {
		self.required
	}
}

impl Type {
	pub fn named(value: impl Into<String>) -> Result<Self, TypeError> {
		Ok(Self::NAMED(TypeName::new(value)?))
	}

	pub const fn variable(variable: TypeVariable) -> Self {
		Self::VARIABLE(variable)
	}

	/// Parse a graph-facing type reference, including the open response-body
	/// type used by nodes whose payload is supplied by the graph.
	pub fn from_reference(value: impl Into<String>) -> Result<Self, TypeError> {
		let value = value.into();
		parse_reference(&value)
	}

	pub fn array(element: Type) -> Self {
		Self::ARRAY(Box::new(element))
	}

	pub fn record(fields: BTreeMap<String, RecordField>) -> Self {
		Self::RECORD(fields)
	}

	pub fn set(element: Type) -> Self {
		Self::SET(Box::new(element))
	}

	pub fn map(key: Type, value: Type) -> Self {
		Self::MAP(Box::new(key), Box::new(value))
	}

	pub fn option(value: Type) -> Self {
		Self::OPTION(Box::new(value))
	}

	pub fn result(value: Type, error: Type) -> Self {
		Self::RESULT(Box::new(value), Box::new(error))
	}

	pub fn future(value: Type) -> Self {
		Self::FUTURE(Box::new(value))
	}

	pub fn stream(value: Type) -> Self {
		Self::STREAM(Box::new(value))
	}

	pub fn union(types: impl IntoIterator<Item = Type>) -> Result<Self, TypeError> {
		normalize_composite(types, false)
	}

	pub fn intersection(types: impl IntoIterator<Item = Type>) -> Result<Self, TypeError> {
		normalize_composite(types, true)
	}

	pub fn variable_id(&self) -> Option<TypeVariable> {
		match self {
			Self::VARIABLE(variable) => Some(*variable),
			_ => None,
		}
	}

	/// Return the canonical graph-facing spelling used by IR type identities.
	pub fn to_reference(&self) -> String {
		match self {
			Self::STRING => "String".to_string(),
			Self::BOOLEAN => "Boolean".to_string(),
			Self::INTEGER => "Integer".to_string(),
			Self::FLOAT => "Float".to_string(),
			Self::NUMBER => "Number".to_string(),
			Self::NULL => "Null".to_string(),
			Self::VOID => "Void".to_string(),
			Self::ARRAY(element) => format!("Array<{}>", element.to_reference()),
			Self::RECORD(_) => "Record".to_string(),
			Self::SET(element) => format!("Set<{}>", element.to_reference()),
			Self::MAP(key, value) => {
				format!("Map<{},{}>", key.to_reference(), value.to_reference())
			}
			Self::OPTION(value) => format!("Option<{}>", value.to_reference()),
			Self::RESULT(value, error) => {
				format!("Result<{},{}>", value.to_reference(), error.to_reference())
			}
			Self::FUTURE(value) => format!("Future<{}>", value.to_reference()),
			Self::STREAM(value) => format!("Stream<{}>", value.to_reference()),
			Self::UNION(members) => members
				.iter()
				.map(Type::to_reference)
				.collect::<Vec<_>>()
				.join(" | "),
			Self::INTERSECTION(members) => members
				.iter()
				.map(Type::to_reference)
				.collect::<Vec<_>>()
				.join(" & "),
			Self::NAMED(name) => name.as_str().to_string(),
			Self::VARIABLE(_) | Self::ANY => "ANY".to_string(),
		}
	}

	pub fn array_element(&self) -> Option<&Type> {
		match self {
			Self::ARRAY(element) => Some(element),
			_ => None,
		}
	}

	pub fn set_element(&self) -> Option<&Type> {
		match self {
			Self::SET(element) => Some(element),
			_ => None,
		}
	}

	pub fn map_types(&self) -> Option<(&Type, &Type)> {
		match self {
			Self::MAP(key, value) => Some((key, value)),
			_ => None,
		}
	}

	/// Whether this type is a concrete scalar value suitable for a flat schema.
	pub fn is_scalar(&self) -> bool {
		matches!(
			self,
			Self::STRING | Self::BOOLEAN | Self::INTEGER | Self::FLOAT | Self::NUMBER | Self::NULL
		)
	}

	pub fn as_name(&self) -> Option<&str> {
		match self {
			Self::NAMED(name) => Some(name.as_str()),
			Self::STRING
			| Self::BOOLEAN
			| Self::INTEGER
			| Self::FLOAT
			| Self::NUMBER
			| Self::NULL
			| Self::VOID
			| Self::ARRAY(_)
			| Self::RECORD(_)
			| Self::SET(_)
			| Self::MAP(_, _)
			| Self::OPTION(_)
			| Self::RESULT(_, _)
			| Self::FUTURE(_)
			| Self::STREAM(_)
			| Self::UNION(_)
			| Self::INTERSECTION(_)
			| Self::VARIABLE(_)
			| Self::ANY => None,
		}
	}
}

fn parse_reference(value: &str) -> Result<Type, TypeError> {
	let value = value.trim();
	if value.is_empty() {
		return Err(TypeError::INVALID_REFERENCE);
	}
	if let Some(parts) = split_top_level(value, '|')? {
		return Type::union(
			parts
				.into_iter()
				.map(parse_reference)
				.collect::<Result<Vec<_>, _>>()?,
		);
	}
	if let Some(parts) = split_top_level(value, '&')? {
		return Type::intersection(
			parts
				.into_iter()
				.map(parse_reference)
				.collect::<Result<Vec<_>, _>>()?,
		);
	}

	if let Some(open) = value.find('<') {
		let base = value[..open].trim();
		let inner = &value[open + 1..];
		if !has_matching_outer_angle(inner) {
			return Err(TypeError::INVALID_REFERENCE);
		}
		let inner = &inner[..inner.len() - 1];
		let arguments = split_top_level(inner, ',')?
			.unwrap_or_else(|| vec![inner])
			.into_iter()
			.map(parse_reference)
			.collect::<Result<Vec<_>, _>>()?;
		return match base.to_ascii_lowercase().as_str() {
			"array" if arguments.len() == 1 => Ok(Type::array(arguments[0].clone())),
			"set" if arguments.len() == 1 => Ok(Type::set(arguments[0].clone())),
			"option" if arguments.len() == 1 => Ok(Type::option(arguments[0].clone())),
			"future" if arguments.len() == 1 => Ok(Type::future(arguments[0].clone())),
			"stream" if arguments.len() == 1 => Ok(Type::stream(arguments[0].clone())),
			"map" if arguments.len() == 2 => {
				Ok(Type::map(arguments[0].clone(), arguments[1].clone()))
			}
			"result" if arguments.len() == 2 => {
				Ok(Type::result(arguments[0].clone(), arguments[1].clone()))
			}
			_ => Err(TypeError::INVALID_REFERENCE),
		};
	}

	match value.to_ascii_lowercase().as_str() {
		"string" => Ok(Type::STRING),
		"boolean" => Ok(Type::BOOLEAN),
		"integer" => Ok(Type::INTEGER),
		"float" => Ok(Type::FLOAT),
		"number" => Ok(Type::NUMBER),
		"null" => Ok(Type::NULL),
		"void" => Ok(Type::VOID),
		"any" => Ok(Type::ANY),
		_ => Type::named(value),
	}
}

fn split_top_level(value: &str, delimiter: char) -> Result<Option<Vec<&str>>, TypeError> {
	let mut depth = 0_u32;
	let mut start = 0;
	let mut parts = Vec::new();
	for (index, character) in value.char_indices() {
		match character {
			'<' => depth = depth.saturating_add(1),
			'>' => {
				if depth == 0 {
					return Err(TypeError::INVALID_REFERENCE);
				}
				depth -= 1;
			}
			character if character == delimiter && depth == 0 => {
				let part = value[start..index].trim();
				if part.is_empty() {
					return Err(TypeError::INVALID_REFERENCE);
				}
				parts.push(part);
				start = index + character.len_utf8();
			}
			_ => {}
		}
	}
	if depth != 0 {
		return Err(TypeError::INVALID_REFERENCE);
	}
	if parts.is_empty() {
		Ok(None)
	} else {
		let part = value[start..].trim();
		if part.is_empty() {
			return Err(TypeError::INVALID_REFERENCE);
		}
		parts.push(part);
		Ok(Some(parts))
	}
}

fn has_matching_outer_angle(value: &str) -> bool {
	let mut depth = 1_u32;
	for (index, character) in value.char_indices() {
		match character {
			'<' => depth = depth.saturating_add(1),
			'>' => {
				if depth == 0 {
					return false;
				}
				depth -= 1;
				if depth == 0 {
					return index == value.len() - 1;
				}
			}
			_ => {}
		}
	}
	false
}

fn normalize_composite(
	types: impl IntoIterator<Item = Type>,
	intersection: bool,
) -> Result<Type, TypeError> {
	let mut members = Vec::new();
	for type_value in types {
		match (intersection, type_value) {
			(false, Type::UNION(nested)) | (true, Type::INTERSECTION(nested)) => {
				members.extend(nested)
			}
			(_, other) => members.push(other),
		}
	}
	members.sort();
	members.dedup();
	match members.as_slice() {
		[] if intersection => Err(TypeError::EMPTY_INTERSECTION),
		[] => Err(TypeError::EMPTY_UNION),
		[type_value] => Ok(type_value.clone()),
		_ if intersection => Ok(Type::INTERSECTION(members)),
		_ => Ok(Type::UNION(members)),
	}
}

/// A directional constraint stating that one produced type must be assignable
/// to one consumed type.
#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum TypeConstraint {
	ASSIGNABLE { from: Type, to: Type },
}

impl TypeConstraint {
	pub fn assignable(from: Type, to: Type) -> Self {
		Self::ASSIGNABLE { from, to }
	}

	pub fn from(&self) -> &Type {
		match self {
			Self::ASSIGNABLE { from, .. } => from,
		}
	}

	pub fn to(&self) -> &Type {
		match self {
			Self::ASSIGNABLE { to, .. } => to,
		}
	}
}

/// The solved bindings for type variables.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TypeSubstitution {
	bindings: BTreeMap<TypeVariable, Type>,
}

impl TypeSubstitution {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn get(&self, variable: TypeVariable) -> Option<&Type> {
		self.bindings.get(&variable)
	}

	pub fn resolve(&self, type_value: &Type) -> Type {
		match type_value {
			Type::VARIABLE(variable) => self
				.get(*variable)
				.map_or_else(|| type_value.clone(), |value| self.resolve(value)),
			Type::ARRAY(element) => Type::array(self.resolve(element)),
			Type::RECORD(fields) => Type::record(
				fields
					.iter()
					.map(|(name, field)| {
						(
							name.clone(),
							RecordField::new(self.resolve(field.type_value()), field.required()),
						)
					})
					.collect(),
			),
			Type::SET(element) => Type::set(self.resolve(element)),
			Type::MAP(key, value) => Type::map(self.resolve(key), self.resolve(value)),
			Type::OPTION(value) => Type::option(self.resolve(value)),
			Type::RESULT(value, error) => Type::result(self.resolve(value), self.resolve(error)),
			Type::FUTURE(value) => Type::future(self.resolve(value)),
			Type::STREAM(value) => Type::stream(self.resolve(value)),
			Type::UNION(members) => Type::union(members.iter().map(|member| self.resolve(member)))
				.expect("a resolved union cannot be empty"),
			Type::INTERSECTION(members) => {
				Type::intersection(members.iter().map(|member| self.resolve(member)))
					.expect("a resolved intersection cannot be empty")
			}
			_ => type_value.clone(),
		}
	}

	pub fn unresolved_variables(&self, type_value: &Type) -> Vec<TypeVariable> {
		let mut variables = BTreeSet::new();
		collect_variables(&self.resolve(type_value), &mut variables);
		variables.into_iter().collect()
	}

	fn bind(&mut self, variable: TypeVariable, type_value: Type) -> Result<(), TypeInferenceError> {
		let type_value = self.resolve(&type_value);
		if type_value == Type::VARIABLE(variable) {
			return Ok(());
		}
		if contains_variable(&type_value, variable) {
			return Err(TypeInferenceError::OCCURS_CHECK {
				variable,
				type_value,
			});
		}
		self.bindings.insert(variable, type_value);
		Ok(())
	}
}

/// A deterministic failure while solving type constraints.
#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TypeInferenceError {
	CONFLICT {
		from: Type,
		to: Type,
	},
	OCCURS_CHECK {
		variable: TypeVariable,
		type_value: Type,
	},
}

impl TypeInferenceError {
	pub fn from(&self) -> Option<&Type> {
		match self {
			Self::CONFLICT { from, .. } => Some(from),
			Self::OCCURS_CHECK { .. } => None,
		}
	}

	pub fn to(&self) -> Option<&Type> {
		match self {
			Self::CONFLICT { to, .. } => Some(to),
			Self::OCCURS_CHECK { .. } => None,
		}
	}
}

/// Solve directional assignability constraints in canonical order.
pub fn solve_constraints(
	constraints: impl IntoIterator<Item = TypeConstraint>,
) -> Result<TypeSubstitution, TypeInferenceError> {
	let mut constraints: Vec<_> = constraints.into_iter().collect();
	constraints.sort();
	let mut substitution = TypeSubstitution::new();
	for constraint in constraints {
		match constraint {
			TypeConstraint::ASSIGNABLE { from, to } => {
				constrain_assignable(&mut substitution, &from, &to)?;
			}
		}
	}
	Ok(substitution)
}

fn constrain_assignable(
	substitution: &mut TypeSubstitution,
	from: &Type,
	to: &Type,
) -> Result<(), TypeInferenceError> {
	let from = substitution.resolve(from);
	let to = substitution.resolve(to);
	match (&from, &to) {
		(_, Type::ANY) => Ok(()),
		(Type::VARIABLE(left), Type::VARIABLE(right)) => match left.cmp(right) {
			std::cmp::Ordering::Equal => Ok(()),
			std::cmp::Ordering::Less => substitution.bind(*right, from),
			std::cmp::Ordering::Greater => substitution.bind(*left, to),
		},
		(Type::VARIABLE(variable), _) => substitution.bind(*variable, to),
		(_, Type::VARIABLE(variable)) => substitution.bind(*variable, from),
		(Type::UNION(sources), _) => {
			for source in sources {
				constrain_assignable(substitution, source, &to)?;
			}
			Ok(())
		}
		(_, Type::UNION(_)) if is_assignable(&from, &to) => Ok(()),
		(Type::INTERSECTION(_), _) if is_assignable(&from, &to) => Ok(()),
		(_, Type::INTERSECTION(targets)) => {
			for target in targets {
				constrain_assignable(substitution, &from, target)?;
			}
			Ok(())
		}
		(Type::INTEGER, Type::NUMBER) | (Type::FLOAT, Type::NUMBER) => Ok(()),
		(Type::ARRAY(left), Type::ARRAY(right)) => constrain_assignable(substitution, left, right),
		(Type::SET(left), Type::SET(right)) => constrain_assignable(substitution, left, right),
		(Type::MAP(left_key, left_value), Type::MAP(right_key, right_value)) => {
			constrain_equal(substitution, left_key, right_key)?;
			constrain_assignable(substitution, left_value, right_value)
		}
		(Type::OPTION(left), Type::OPTION(right))
		| (Type::FUTURE(left), Type::FUTURE(right))
		| (Type::STREAM(left), Type::STREAM(right)) => constrain_assignable(substitution, left, right),
		(Type::RESULT(left_value, left_error), Type::RESULT(right_value, right_error)) => {
			constrain_assignable(substitution, left_value, right_value)?;
			constrain_assignable(substitution, left_error, right_error)
		}
		(Type::RECORD(left), Type::RECORD(right)) => {
			for (name, target) in right {
				let Some(source) = left.get(name) else {
					if target.required() {
						return Err(TypeInferenceError::CONFLICT { from, to });
					}
					continue;
				};
				if target.required() && !source.required() {
					return Err(TypeInferenceError::CONFLICT { from, to });
				}
				constrain_assignable(substitution, source.type_value(), target.type_value())?;
			}
			Ok(())
		}
		(left, right) if left == right => Ok(()),
		_ => Err(TypeInferenceError::CONFLICT { from, to }),
	}
}

fn constrain_equal(
	substitution: &mut TypeSubstitution,
	left: &Type,
	right: &Type,
) -> Result<(), TypeInferenceError> {
	let left = substitution.resolve(left);
	let right = substitution.resolve(right);
	match (&left, &right) {
		(Type::VARIABLE(variable), _) => substitution.bind(*variable, right),
		(_, Type::VARIABLE(variable)) => substitution.bind(*variable, left),
		(Type::ARRAY(left), Type::ARRAY(right))
		| (Type::SET(left), Type::SET(right))
		| (Type::OPTION(left), Type::OPTION(right))
		| (Type::FUTURE(left), Type::FUTURE(right))
		| (Type::STREAM(left), Type::STREAM(right)) => constrain_equal(substitution, left, right),
		(Type::MAP(left_key, left_value), Type::MAP(right_key, right_value)) => {
			constrain_equal(substitution, left_key, right_key)?;
			constrain_equal(substitution, left_value, right_value)
		}
		(Type::RESULT(left_value, left_error), Type::RESULT(right_value, right_error)) => {
			constrain_equal(substitution, left_value, right_value)?;
			constrain_equal(substitution, left_error, right_error)
		}
		(left, right) if left == right => Ok(()),
		_ => Err(TypeInferenceError::CONFLICT {
			from: left,
			to: right,
		}),
	}
}

fn collect_variables(type_value: &Type, variables: &mut BTreeSet<TypeVariable>) {
	match type_value {
		Type::VARIABLE(variable) => {
			variables.insert(*variable);
		}
		Type::ARRAY(element) => collect_variables(element, variables),
		Type::SET(element)
		| Type::OPTION(element)
		| Type::FUTURE(element)
		| Type::STREAM(element) => collect_variables(element, variables),
		Type::MAP(key, value) | Type::RESULT(key, value) => {
			collect_variables(key, variables);
			collect_variables(value, variables);
		}
		Type::RECORD(fields) => {
			for field in fields.values() {
				collect_variables(field.type_value(), variables);
			}
		}
		Type::UNION(members) | Type::INTERSECTION(members) => {
			for member in members {
				collect_variables(member, variables);
			}
		}
		_ => {}
	}
}

fn contains_variable(type_value: &Type, variable: TypeVariable) -> bool {
	match type_value {
		Type::VARIABLE(candidate) => *candidate == variable,
		Type::ARRAY(element) => contains_variable(element, variable),
		Type::SET(element)
		| Type::OPTION(element)
		| Type::FUTURE(element)
		| Type::STREAM(element) => contains_variable(element, variable),
		Type::MAP(key, value) | Type::RESULT(key, value) => {
			contains_variable(key, variable) || contains_variable(value, variable)
		}
		Type::RECORD(fields) => fields
			.values()
			.any(|field| contains_variable(field.type_value(), variable)),
		Type::UNION(members) | Type::INTERSECTION(members) => members
			.iter()
			.any(|member| contains_variable(member, variable)),
		_ => false,
	}
}

/// Check whether a produced value can be connected to a consumed value.
pub fn is_assignable(from: &Type, to: &Type) -> bool {
	match (from, to) {
		(_, Type::ANY) => true,
		(Type::ANY, _) => false,
		(_, Type::UNION(targets)) => targets.iter().any(|target| is_assignable(from, target)),
		(Type::UNION(sources), _) => sources.iter().all(|source| is_assignable(source, to)),
		(_, Type::INTERSECTION(targets)) => {
			targets.iter().all(|target| is_assignable(from, target))
		}
		(Type::INTERSECTION(sources), _) => sources.iter().any(|source| is_assignable(source, to)),
		(Type::INTEGER, Type::NUMBER) | (Type::FLOAT, Type::NUMBER) => true,
		(Type::ARRAY(left), Type::ARRAY(right)) => is_assignable(left, right),
		(Type::SET(left), Type::SET(right)) => is_assignable(left, right),
		(Type::MAP(left_key, left_value), Type::MAP(right_key, right_value)) => {
			left_key == right_key && is_assignable(left_value, right_value)
		}
		(Type::OPTION(left), Type::OPTION(right)) => is_assignable(left, right),
		(Type::RESULT(left_value, left_error), Type::RESULT(right_value, right_error)) => {
			is_assignable(left_value, right_value) && is_assignable(left_error, right_error)
		}
		(Type::FUTURE(left), Type::FUTURE(right)) => is_assignable(left, right),
		(Type::STREAM(left), Type::STREAM(right)) => is_assignable(left, right),
		(Type::RECORD(left), Type::RECORD(right)) => right.iter().all(|(name, target)| {
			let Some(source) = left.get(name) else {
				return !target.required();
			};
			(!target.required() || source.required())
				&& is_assignable(source.type_value(), target.type_value())
		}),
		(left, right) if left == right => true,
		(Type::NAMED(left), Type::NAMED(right)) => left == right,
		_ => false,
	}
}

/// Explain a failed assignability check.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssignabilityError {
	from: Type,
	to: Type,
}

impl AssignabilityError {
	pub fn from(&self) -> &Type {
		&self.from
	}

	pub fn to(&self) -> &Type {
		&self.to
	}
}

pub fn check_assignable(from: &Type, to: &Type) -> Result<(), AssignabilityError> {
	if is_assignable(from, to) {
		Ok(())
	} else {
		Err(AssignabilityError {
			from: from.clone(),
			to: to.clone(),
		})
	}
}

/// Errors raised while constructing the initial type representation.
#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TypeError {
	EMPTY_NAME,
	INVALID_REFERENCE,
	EMPTY_UNION,
	EMPTY_INTERSECTION,
}

impl fmt::Display for TypeError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "{self:?}")
	}
}

impl std::error::Error for TypeError {}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn named_types_are_assignable_only_when_their_names_match() {
		let request = Type::named("Request").unwrap();
		let same_request = Type::named("Request").unwrap();
		let response = Type::named("Response").unwrap();

		assert!(is_assignable(&request, &same_request));
		assert!(!is_assignable(&request, &response));
		assert!(check_assignable(&request, &response).is_err());
	}

	#[test]
	fn any_accepts_every_produced_type_but_is_not_a_concrete_output() {
		let request = Type::named("Request").unwrap();

		assert!(is_assignable(&request, &Type::ANY));
		assert!(!is_assignable(&Type::ANY, &request));
	}

	#[test]
	fn empty_type_names_are_rejected() {
		assert_eq!(TypeName::new(" "), Err(TypeError::EMPTY_NAME));
	}

	#[test]
	fn graph_references_parse_any_as_an_open_type() {
		assert_eq!(Type::from_reference("ANY").unwrap(), Type::ANY);
		assert_eq!(
			Type::from_reference("User").unwrap(),
			Type::named("User").unwrap()
		);
	}

	#[test]
	fn graph_references_parse_primitive_types_case_insensitively() {
		assert_eq!(Type::from_reference("String").unwrap(), Type::STRING);
		assert_eq!(Type::from_reference("boolean").unwrap(), Type::BOOLEAN);
		assert_eq!(Type::from_reference("INTEGER").unwrap(), Type::INTEGER);
		assert_eq!(Type::from_reference("Float").unwrap(), Type::FLOAT);
		assert_eq!(Type::from_reference("Number").unwrap(), Type::NUMBER);
		assert_eq!(Type::from_reference("Null").unwrap(), Type::NULL);
		assert_eq!(Type::from_reference("Void").unwrap(), Type::VOID);
	}

	#[test]
	fn numeric_primitives_widen_to_number() {
		assert!(is_assignable(&Type::INTEGER, &Type::NUMBER));
		assert!(is_assignable(&Type::FLOAT, &Type::NUMBER));
		assert!(!is_assignable(&Type::NUMBER, &Type::INTEGER));
		assert!(!is_assignable(&Type::ANY, &Type::NUMBER));
	}

	#[test]
	fn array_references_and_assignability_are_structural() {
		let strings = Type::from_reference("Array<String>").unwrap();
		let values = Type::array(Type::ANY);

		assert_eq!(strings, Type::array(Type::STRING));
		assert_eq!(
			Type::from_reference("ARRAY<Boolean>").unwrap(),
			Type::array(Type::BOOLEAN)
		);
		assert!(is_assignable(&strings, &values));
		assert!(!is_assignable(&values, &strings));
		assert_eq!(
			Type::from_reference("Array<>"),
			Err(TypeError::INVALID_REFERENCE)
		);
	}

	#[test]
	fn graph_references_parse_nested_generics_and_normalize_composites() {
		let parsed = Type::from_reference("Result<Option<User>, Error>").unwrap();
		assert_eq!(parsed.to_reference(), "Result<Option<User>,Error>");

		let union = Type::from_reference("String | Boolean | String").unwrap();
		assert_eq!(union.to_reference(), "String | Boolean");
		assert_eq!(
			Type::from_reference("Array<String | Null>")
				.unwrap()
				.to_reference(),
			"Array<String | Null>"
		);
		assert_eq!(
			Type::from_reference("Map<String>").unwrap_err(),
			TypeError::INVALID_REFERENCE
		);
	}

	#[test]
	fn unions_and_intersections_have_structural_assignability() {
		let string_or_boolean = Type::union([Type::STRING, Type::BOOLEAN]).unwrap();
		assert!(is_assignable(&Type::STRING, &string_or_boolean));
		assert!(!is_assignable(&Type::INTEGER, &string_or_boolean));

		let numeric_union = Type::union([Type::INTEGER, Type::FLOAT]).unwrap();
		assert!(is_assignable(&numeric_union, &Type::NUMBER));

		let request = Type::named("Request").unwrap();
		let serializable = Type::named("Serializable").unwrap();
		let intersection = Type::intersection([request.clone(), serializable]).unwrap();
		assert!(is_assignable(&intersection, &request));
		assert!(!is_assignable(&request, &intersection));
	}

	#[test]
	fn generic_types_follow_declared_variance_rules() {
		assert!(is_assignable(
			&Type::option(Type::INTEGER),
			&Type::option(Type::NUMBER)
		));
		assert!(is_assignable(
			&Type::result(Type::INTEGER, Type::STRING),
			&Type::result(Type::NUMBER, Type::STRING)
		));
		assert!(is_assignable(
			&Type::future(Type::INTEGER),
			&Type::future(Type::NUMBER)
		));
		assert!(is_assignable(
			&Type::stream(Type::INTEGER),
			&Type::stream(Type::NUMBER)
		));
		assert!(is_assignable(
			&Type::map(Type::STRING, Type::INTEGER),
			&Type::map(Type::STRING, Type::NUMBER)
		));
		assert!(!is_assignable(
			&Type::map(Type::INTEGER, Type::STRING),
			&Type::map(Type::NUMBER, Type::STRING)
		));
	}

	#[test]
	fn records_respect_required_fields_and_nested_assignability() {
		let mut source_fields = BTreeMap::new();
		source_fields.insert("id".to_string(), RecordField::new(Type::INTEGER, true));
		source_fields.insert(
			"nickname".to_string(),
			RecordField::new(Type::STRING, false),
		);
		let source = Type::record(source_fields);

		let mut target_fields = BTreeMap::new();
		target_fields.insert("id".to_string(), RecordField::new(Type::NUMBER, true));
		target_fields.insert("avatar".to_string(), RecordField::new(Type::STRING, false));
		let target = Type::record(target_fields);

		assert!(is_assignable(&source, &target));

		let mut required_target_fields = BTreeMap::new();
		required_target_fields.insert("nickname".to_string(), RecordField::new(Type::STRING, true));
		assert!(!is_assignable(
			&source,
			&Type::record(required_target_fields)
		));
	}

	#[test]
	fn constraints_infer_a_concrete_type_for_an_open_variable() {
		let variable = Type::variable(TypeVariable::new(7));
		let substitution =
			solve_constraints([TypeConstraint::assignable(Type::STRING, variable.clone())])
				.unwrap();

		assert_eq!(substitution.resolve(&variable), Type::STRING);
	}

	#[test]
	fn constraints_propagate_through_multiple_variables() {
		let first = Type::variable(TypeVariable::new(1));
		let second = Type::variable(TypeVariable::new(2));
		let substitution = solve_constraints([
			TypeConstraint::assignable(first.clone(), second.clone()),
			TypeConstraint::assignable(Type::array(Type::INTEGER), first.clone()),
		])
		.unwrap();

		assert_eq!(substitution.resolve(&first), Type::array(Type::INTEGER));
		assert_eq!(substitution.resolve(&second), Type::array(Type::INTEGER));
	}

	#[test]
	fn constraints_infer_nested_generic_arguments() {
		let value = Type::variable(TypeVariable::new(1));
		let error = Type::variable(TypeVariable::new(2));
		let substitution = solve_constraints([TypeConstraint::assignable(
			Type::result(Type::option(Type::STRING), Type::BOOLEAN),
			Type::result(Type::option(value.clone()), error.clone()),
		)])
		.unwrap();

		assert_eq!(substitution.resolve(&value), Type::STRING);
		assert_eq!(substitution.resolve(&error), Type::BOOLEAN);
	}

	#[test]
	fn conflicting_constraints_are_rejected_deterministically() {
		let variable = Type::variable(TypeVariable::new(1));
		let constraints = [
			TypeConstraint::assignable(Type::BOOLEAN, variable.clone()),
			TypeConstraint::assignable(Type::STRING, variable),
		];

		let first = solve_constraints(constraints.clone()).unwrap_err();
		let second = solve_constraints(constraints.into_iter().rev()).unwrap_err();

		assert_eq!(first, second);
	}

	#[test]
	fn recursive_constraints_fail_the_occurs_check() {
		let variable = Type::variable(TypeVariable::new(1));
		let error = solve_constraints([TypeConstraint::assignable(
			variable.clone(),
			Type::array(variable),
		)])
		.unwrap_err();

		assert!(matches!(error, TypeInferenceError::OCCURS_CHECK { .. }));
	}
}
