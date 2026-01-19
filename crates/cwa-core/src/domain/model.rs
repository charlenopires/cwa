//! Domain model types (DDD).

use serde::{Deserialize, Serialize};
use cwa_db::queries::domains::{BoundedContextRow, DomainObjectRow, GlossaryTermRow};

/// A bounded context (DDD).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundedContext {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub description: Option<String>,
    pub responsibilities: Vec<String>,
    pub upstream_contexts: Vec<String>,
    pub downstream_contexts: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl BoundedContext {
    /// Create from database row.
    pub fn from_row(row: BoundedContextRow) -> Self {
        let responsibilities: Vec<String> = row
            .responsibilities
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        let upstream_contexts: Vec<String> = row
            .upstream_contexts
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        let downstream_contexts: Vec<String> = row
            .downstream_contexts
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        Self {
            id: row.id,
            project_id: row.project_id,
            name: row.name,
            description: row.description,
            responsibilities,
            upstream_contexts,
            downstream_contexts,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// A domain object (entity, value object, aggregate, service, event).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainObject {
    pub id: String,
    pub context_id: String,
    pub name: String,
    pub object_type: ObjectType,
    pub description: Option<String>,
    pub properties: Vec<Property>,
    pub behaviors: Vec<Behavior>,
    pub invariants: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl DomainObject {
    /// Create from database row.
    pub fn from_row(row: DomainObjectRow) -> Self {
        let properties: Vec<Property> = row
            .properties
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        let behaviors: Vec<Behavior> = row
            .behaviors
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        let invariants: Vec<String> = row
            .invariants
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        Self {
            id: row.id,
            context_id: row.context_id,
            name: row.name,
            object_type: ObjectType::from_str(&row.object_type),
            description: row.description,
            properties,
            behaviors,
            invariants,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Type of domain object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectType {
    Entity,
    ValueObject,
    Aggregate,
    Service,
    Event,
}

impl ObjectType {
    /// Parse from string.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "entity" => Self::Entity,
            "value_object" => Self::ValueObject,
            "aggregate" => Self::Aggregate,
            "service" => Self::Service,
            "event" => Self::Event,
            _ => Self::Entity,
        }
    }

    /// Convert to string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Entity => "entity",
            Self::ValueObject => "value_object",
            Self::Aggregate => "aggregate",
            Self::Service => "service",
            Self::Event => "event",
        }
    }
}

/// A property of a domain object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Property {
    pub name: String,
    pub property_type: String,
    pub required: bool,
}

/// A behavior of a domain object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Behavior {
    pub name: String,
    pub description: String,
}

/// A glossary term (ubiquitous language).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlossaryTerm {
    pub id: String,
    pub project_id: String,
    pub context_id: Option<String>,
    pub term: String,
    pub definition: String,
    pub aliases: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl GlossaryTerm {
    /// Create from database row.
    pub fn from_row(row: GlossaryTermRow) -> Self {
        let aliases: Vec<String> = row
            .aliases
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        Self {
            id: row.id,
            project_id: row.project_id,
            context_id: row.context_id,
            term: row.term,
            definition: row.definition,
            aliases,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Complete domain model for a project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainModel {
    pub contexts: Vec<ContextWithObjects>,
    pub glossary: Vec<GlossaryTerm>,
}

/// A bounded context with its domain objects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextWithObjects {
    pub context: BoundedContext,
    pub objects: Vec<DomainObject>,
}

/// Context map showing relationships.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMap {
    pub contexts: Vec<String>,
    pub relationships: Vec<ContextRelationship>,
}

/// A relationship between contexts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextRelationship {
    pub upstream_id: String,
    pub downstream_id: String,
    pub relationship_type: String,
}
