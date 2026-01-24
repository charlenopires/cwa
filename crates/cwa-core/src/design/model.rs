//! Design system domain model.
//!
//! Represents a complete design system extracted from a UI screenshot,
//! including color palette, typography, spacing, and component tokens.

use serde::{Deserialize, Serialize};

/// Complete design system extracted from an image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignSystem {
    pub id: String,
    pub project_id: String,
    pub source_url: String,
    pub colors: ColorPalette,
    pub typography: Typography,
    pub spacing: Vec<SpacingToken>,
    pub border_radius: Vec<RadiusToken>,
    pub shadows: Vec<ShadowToken>,
    pub breakpoints: Vec<BreakpointToken>,
    pub components: Vec<IdentifiedComponent>,
    #[serde(default)]
    pub raw_analysis: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

/// Color palette organized by semantic role.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ColorPalette {
    #[serde(default)]
    pub primary: Vec<ColorToken>,
    #[serde(default)]
    pub secondary: Vec<ColorToken>,
    #[serde(default)]
    pub neutral: Vec<ColorToken>,
    #[serde(default)]
    pub semantic: SemanticColors,
}

/// A single color token with naming and value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorToken {
    pub name: String,
    pub hex: String,
    #[serde(default)]
    pub rgb: Option<String>,
    #[serde(default)]
    pub usage: Option<String>,
}

/// Semantic colors for UI states.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SemanticColors {
    #[serde(default)]
    pub success: Option<String>,
    #[serde(default)]
    pub warning: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub info: Option<String>,
}

/// Typography system.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Typography {
    #[serde(default)]
    pub font_families: Vec<FontFamily>,
    #[serde(default)]
    pub scale: Vec<TypeScaleStep>,
    #[serde(default)]
    pub line_heights: Vec<f64>,
}

/// A font family definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontFamily {
    pub name: String,
    /// serif, sans-serif, monospace
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub weights: Vec<u32>,
    #[serde(default)]
    pub usage: Option<String>,
}

/// A step in the type scale.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeScaleStep {
    pub name: String,
    pub size_px: f64,
    #[serde(default)]
    pub weight: Option<u32>,
    #[serde(default)]
    pub line_height: Option<f64>,
}

/// Spacing token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpacingToken {
    pub name: String,
    pub value_px: f64,
}

/// Border radius token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadiusToken {
    pub name: String,
    pub value_px: f64,
}

/// Shadow token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowToken {
    pub name: String,
    /// CSS box-shadow syntax
    pub value: String,
}

/// Breakpoint token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakpointToken {
    pub name: String,
    pub min_width_px: u32,
}

/// An identified UI component from the screenshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentifiedComponent {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub variants: Vec<String>,
    #[serde(default)]
    pub states: Vec<String>,
}

impl DesignSystem {
    /// Build from a DB row, parsing JSON fields.
    pub fn from_row(row: cwa_db::queries::design_systems::DesignSystemRow) -> Self {
        let colors: ColorPalette = row.colors_json
            .as_deref()
            .and_then(|j| serde_json::from_str(j).ok())
            .unwrap_or_default();

        let typography: Typography = row.typography_json
            .as_deref()
            .and_then(|j| serde_json::from_str(j).ok())
            .unwrap_or_default();

        let spacing: Vec<SpacingToken> = row.spacing_json
            .as_deref()
            .and_then(|j| serde_json::from_str(j).ok())
            .unwrap_or_default();

        let border_radius: Vec<RadiusToken> = row.border_radius_json
            .as_deref()
            .and_then(|j| serde_json::from_str(j).ok())
            .unwrap_or_default();

        let shadows: Vec<ShadowToken> = row.shadows_json
            .as_deref()
            .and_then(|j| serde_json::from_str(j).ok())
            .unwrap_or_default();

        let breakpoints: Vec<BreakpointToken> = row.breakpoints_json
            .as_deref()
            .and_then(|j| serde_json::from_str(j).ok())
            .unwrap_or_default();

        let components: Vec<IdentifiedComponent> = row.components_json
            .as_deref()
            .and_then(|j| serde_json::from_str(j).ok())
            .unwrap_or_default();

        Self {
            id: row.id,
            project_id: row.project_id,
            source_url: row.source_url,
            colors,
            typography,
            spacing,
            border_radius,
            shadows,
            breakpoints,
            components,
            raw_analysis: row.raw_analysis.unwrap_or_default(),
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }

    /// Count total colors across all palettes.
    pub fn colors_count(&self) -> usize {
        self.colors.primary.len()
            + self.colors.secondary.len()
            + self.colors.neutral.len()
            + [&self.colors.semantic.success, &self.colors.semantic.warning,
               &self.colors.semantic.error, &self.colors.semantic.info]
                .iter().filter(|c| c.is_some()).count()
    }

    /// Get font family names as a comma-separated string.
    pub fn typography_families(&self) -> String {
        self.typography.font_families
            .iter()
            .map(|f| f.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }
}
