//! Domain queries — delegates to cwa-redis.
pub use cwa_redis::queries::domains::{
    BoundedContextRow, DomainObjectRow,
    create_context, get_context, get_context_in_project,
    list_contexts, create_domain_object, list_domain_objects,
    list_domain_objects_by_context,
};
pub use cwa_redis::queries::glossary::{
    GlossaryTermRow, create_glossary_term, list_glossary,
};
