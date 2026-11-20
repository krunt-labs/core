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
		if let Some(open) = value.find('<') {
			if !value[..open].eq_ignore_ascii_case("Array") {
				return Self::named(value);
			}
			let inner = &value[open + 1..];
			let Some(inner) = inner.strip_suffix('>') else {
				return Err(TypeError::INVALID_REFERENCE);
			};
			if inner.trim().is_empty() {
				return Err(TypeError::INVALID_REFERENCE);
			}
			return Ok(Self::ARRAY(Box::new(Self::from_reference(inner.trim())?)));
		}
		match value.to_ascii_lowercase().as_str() {
			"string" => Ok(Self::STRING),
			"boolean" => Ok(Self::BOOLEAN),
			"integer" => Ok(Self::INTEGER),
			"float" => Ok(Self::FLOAT),
			"number" => Ok(Self::NUMBER),
			"null" => Ok(Self::NULL),
			"void" => Ok(Self::VOID),
			"any" => Ok(Self::ANY),
			_ => Self::named(value),
		}
	}

	pub fn array(element: Type) -> Self {
		Self::ARRAY(Box::new(element))
	}

	pub fn record(fields: BTreeMap<String, RecordField>) -> Self {
		Self::RECORD(fields)
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
			| Self::VARIABLE(_)
			| Self::ANY => None,
		}
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
		(Type::INTEGER, Type::NUMBER) | (Type::FLOAT, Type::NUMBER) => Ok(()),
		(Type::ARRAY(left), Type::ARRAY(right)) => constrain_assignable(substitution, left, right),
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

fn collect_variables(type_value: &Type, variables: &mut BTreeSet<TypeVariable>) {
	match type_value {
		Type::VARIABLE(variable) => {
			variables.insert(*variable);
		}
		Type::ARRAY(element) => collect_variables(element, variables),
		Type::RECORD(fields) => {
			for field in fields.values() {
				collect_variables(field.type_value(), variables);
			}
		}
		_ => {}
	}
}

fn contains_variable(type_value: &Type, variable: TypeVariable) -> bool {
	match type_value {
		Type::VARIABLE(candidate) => *candidate == variable,
		Type::ARRAY(element) => contains_variable(element, variable),
		Type::RECORD(fields) => fields
			.values()
			.any(|field| contains_variable(field.type_value(), variable)),
		_ => false,
	}
}

/// Check whether a produced value can be connected to a consumed value.
pub fn is_assignable(from: &Type, to: &Type) -> bool {
	match (from, to) {
		(_, Type::ANY) => true,
		(Type::INTEGER, Type::NUMBER) | (Type::FLOAT, Type::NUMBER) => true,
		(Type::ARRAY(left), Type::ARRAY(right)) => is_assignable(left, right),
		(Type::RECORD(left), Type::RECORD(right)) => right.iter().all(|(name, target)| {
			let Some(source) = left.get(name) else {
				return !target.required();
			};
			(!target.required() || source.required())
				&& is_assignable(source.type_value(), target.type_value())
		}),
		(left, right) if left == right => true,
		(Type::NAMED(left), Type::NAMED(right)) => left == right,
		(Type::ANY, Type::NAMED(_)) => false,
		(Type::ANY, _) => false,
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
