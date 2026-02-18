//! Design system queries — stub (design systems stored in project info for now).
use cwa_redis::RedisPool as DbPool;
use cwa_redis::RedisError as DbError;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DesignSystemRow {
    pub id: String,
    pub project_id: String,
    pub source_url: String,
    pub colors_json: Option<String>,
    pub typography_json: Option<String>,
    pub spacing_json: Option<String>,
    pub border_radius_json: Option<String>,
    pub shadows_json: Option<String>,
    pub breakpoints_json: Option<String>,
    pub components_json: Option<String>,
    pub raw_analysis: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub async fn create_design_system(
    _pool: &DbPool,
    _id: &str,
    _project_id: &str,
    _source_url: &str,
    _colors_json: Option<&str>,
    _typography_json: Option<&str>,
    _spacing_json: Option<&str>,
    _border_radius_json: Option<&str>,
    _shadows_json: Option<&str>,
    _breakpoints_json: Option<&str>,
    _components_json: Option<&str>,
    _raw_analysis: Option<&str>,
) -> Result<(), DbError> {
    // TODO: store design system in Redis
    Ok(())
}

pub async fn get_latest_design_system(
    _pool: &DbPool,
    _project_id: &str,
) -> Result<Option<DesignSystemRow>, DbError> {
    Ok(None)
}

pub async fn list_design_systems(
    _pool: &DbPool,
    _project_id: &str,
) -> Result<Vec<DesignSystemRow>, DbError> {
    Ok(vec![])
}
