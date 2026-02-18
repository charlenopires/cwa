//! Design system artifact generation.
//!
//! Generates `.claude/design-system.md` from the stored design system,
//! providing a complete design token reference for Claude Code agents.

use anyhow::Result;
use std::path::Path;

use cwa_db::DbPool;
use cwa_db::queries::design_systems::DesignSystemRow;

/// Generated design system markdown file.
#[derive(Debug, Clone)]
pub struct GeneratedDesignSystem {
    pub content: String,
    pub filename: String,
}

/// Generate the design-system.md content from the latest stored design system.
pub async fn generate_design_system_md(db: &DbPool, project_id: &str) -> Result<Option<GeneratedDesignSystem>> {
    let row = cwa_db::queries::design_systems::get_latest_design_system(db, project_id)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to query design system: {}", e))?;

    let row = match row {
        Some(r) => r,
        None => return Ok(None),
    };

    let content = render_design_system_md(&row);

    Ok(Some(GeneratedDesignSystem {
        content,
        filename: "design-system.md".to_string(),
    }))
}

/// Write the design-system.md file to the .claude/ directory.
pub fn write_design_system_md(generated: &GeneratedDesignSystem, project_dir: &Path) -> Result<String> {
    let dir = project_dir.join(".claude");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(&generated.filename);
    std::fs::write(&path, &generated.content)?;
    Ok(path.display().to_string())
}

/// Render the design system into a comprehensive markdown document.
fn render_design_system_md(row: &DesignSystemRow) -> String {
    let mut md = String::new();

    md.push_str("# Design System\n\n");
    md.push_str(&format!("> Extracted from: {}\n", row.source_url));
    md.push_str(&format!("> Generated: {}\n\n", row.created_at));
    md.push_str("All UI implementation MUST follow the design tokens defined below.\n\n");

    // CSS Custom Properties
    md.push_str("## CSS Custom Properties\n\n");
    md.push_str("```css\n:root {\n");

    // Colors
    if let Some(ref json) = row.colors_json {
        if let Ok(palette) = serde_json::from_str::<serde_json::Value>(json) {
            render_css_colors(&mut md, &palette);
        }
    }

    // Spacing
    if let Some(ref json) = row.spacing_json {
        if let Ok(tokens) = serde_json::from_str::<Vec<serde_json::Value>>(json) {
            md.push_str("\n  /* Spacing */\n");
            for token in &tokens {
                if let (Some(name), Some(val)) = (token["name"].as_str(), token["value_px"].as_f64()) {
                    md.push_str(&format!("  --{}: {}px;\n", name, val));
                }
            }
        }
    }

    // Border Radius
    if let Some(ref json) = row.border_radius_json {
        if let Ok(tokens) = serde_json::from_str::<Vec<serde_json::Value>>(json) {
            if !tokens.is_empty() {
                md.push_str("\n  /* Border Radius */\n");
                for token in &tokens {
                    if let (Some(name), Some(val)) = (token["name"].as_str(), token["value_px"].as_f64()) {
                        md.push_str(&format!("  --{}: {}px;\n", name, val));
                    }
                }
            }
        }
    }

    // Shadows
    if let Some(ref json) = row.shadows_json {
        if let Ok(tokens) = serde_json::from_str::<Vec<serde_json::Value>>(json) {
            if !tokens.is_empty() {
                md.push_str("\n  /* Shadows */\n");
                for token in &tokens {
                    if let (Some(name), Some(val)) = (token["name"].as_str(), token["value"].as_str()) {
                        md.push_str(&format!("  --{}: {};\n", name, val));
                    }
                }
            }
        }
    }

    // Breakpoints
    if let Some(ref json) = row.breakpoints_json {
        if let Ok(tokens) = serde_json::from_str::<Vec<serde_json::Value>>(json) {
            if !tokens.is_empty() {
                md.push_str("\n  /* Breakpoints */\n");
                for token in &tokens {
                    if let (Some(name), Some(val)) = (token["name"].as_str(), token["min_width_px"].as_u64()) {
                        md.push_str(&format!("  --breakpoint-{}: {}px;\n", name, val));
                    }
                }
            }
        }
    }

    md.push_str("}\n```\n\n");

    // Color Palette Table
    if let Some(ref json) = row.colors_json {
        if let Ok(palette) = serde_json::from_str::<serde_json::Value>(json) {
            render_color_palette_section(&mut md, &palette);
        }
    }

    // Typography
    if let Some(ref json) = row.typography_json {
        if let Ok(typo) = serde_json::from_str::<serde_json::Value>(json) {
            render_typography_section(&mut md, &typo);
        }
    }

    // Spacing Scale
    if let Some(ref json) = row.spacing_json {
        if let Ok(tokens) = serde_json::from_str::<Vec<serde_json::Value>>(json) {
            if !tokens.is_empty() {
                md.push_str("## Spacing Scale\n\n");
                md.push_str("| Token | Value |\n");
                md.push_str("|-------|-------|\n");
                for token in &tokens {
                    if let (Some(name), Some(val)) = (token["name"].as_str(), token["value_px"].as_f64()) {
                        md.push_str(&format!("| `--{}` | {}px |\n", name, val));
                    }
                }
                md.push('\n');
            }
        }
    }

    // Border Radius
    if let Some(ref json) = row.border_radius_json {
        if let Ok(tokens) = serde_json::from_str::<Vec<serde_json::Value>>(json) {
            if !tokens.is_empty() {
                md.push_str("## Border Radius\n\n");
                md.push_str("| Token | Value |\n");
                md.push_str("|-------|-------|\n");
                for token in &tokens {
                    if let (Some(name), Some(val)) = (token["name"].as_str(), token["value_px"].as_f64()) {
                        md.push_str(&format!("| `--{}` | {}px |\n", name, val));
                    }
                }
                md.push('\n');
            }
        }
    }

    // Shadows
    if let Some(ref json) = row.shadows_json {
        if let Ok(tokens) = serde_json::from_str::<Vec<serde_json::Value>>(json) {
            if !tokens.is_empty() {
                md.push_str("## Shadows\n\n");
                md.push_str("| Token | Value |\n");
                md.push_str("|-------|-------|\n");
                for token in &tokens {
                    if let (Some(name), Some(val)) = (token["name"].as_str(), token["value"].as_str()) {
                        md.push_str(&format!("| `--{}` | `{}` |\n", name, val));
                    }
                }
                md.push('\n');
            }
        }
    }

    // Breakpoints
    if let Some(ref json) = row.breakpoints_json {
        if let Ok(tokens) = serde_json::from_str::<Vec<serde_json::Value>>(json) {
            if !tokens.is_empty() {
                md.push_str("## Breakpoints\n\n");
                md.push_str("| Name | Min Width |\n");
                md.push_str("|------|-----------|\n");
                for token in &tokens {
                    if let (Some(name), Some(val)) = (token["name"].as_str(), token["min_width_px"].as_u64()) {
                        md.push_str(&format!("| {} | {}px |\n", name, val));
                    }
                }
                md.push('\n');
            }
        }
    }

    // Components
    if let Some(ref json) = row.components_json {
        if let Ok(components) = serde_json::from_str::<Vec<serde_json::Value>>(json) {
            if !components.is_empty() {
                md.push_str("## Components\n\n");
                for comp in &components {
                    if let Some(name) = comp["name"].as_str() {
                        md.push_str(&format!("### {}\n\n", name));
                        if let Some(desc) = comp["description"].as_str() {
                            md.push_str(&format!("{}\n\n", desc));
                        }
                        if let Some(variants) = comp["variants"].as_array() {
                            if !variants.is_empty() {
                                md.push_str("**Variants:** ");
                                let vs: Vec<&str> = variants.iter()
                                    .filter_map(|v| v.as_str())
                                    .collect();
                                md.push_str(&vs.join(", "));
                                md.push_str("\n\n");
                            }
                        }
                        if let Some(states) = comp["states"].as_array() {
                            if !states.is_empty() {
                                md.push_str("**States:** ");
                                let ss: Vec<&str> = states.iter()
                                    .filter_map(|v| v.as_str())
                                    .collect();
                                md.push_str(&ss.join(", "));
                                md.push_str("\n\n");
                            }
                        }
                    }
                }
            }
        }
    }

    md
}

/// Render CSS color custom properties.
fn render_css_colors(md: &mut String, palette: &serde_json::Value) {
    let sections = [
        ("primary", "Primary Colors"),
        ("secondary", "Secondary Colors"),
        ("neutral", "Neutral Colors"),
    ];

    for (key, comment) in &sections {
        if let Some(colors) = palette[key].as_array() {
            if !colors.is_empty() {
                md.push_str(&format!("\n  /* {} */\n", comment));
                for color in colors {
                    if let (Some(name), Some(hex)) = (color["name"].as_str(), color["hex"].as_str()) {
                        md.push_str(&format!("  --color-{}: {};\n", name, hex));
                    }
                }
            }
        }
    }

    // Semantic colors
    if let Some(semantic) = palette.get("semantic") {
        let has_any = ["success", "warning", "error", "info"]
            .iter()
            .any(|k| semantic[*k].as_str().is_some());

        if has_any {
            md.push_str("\n  /* Semantic Colors */\n");
            for key in &["success", "warning", "error", "info"] {
                if let Some(hex) = semantic[*key].as_str() {
                    md.push_str(&format!("  --color-{}: {};\n", key, hex));
                }
            }
        }
    }
}

/// Render color palette as a markdown table.
fn render_color_palette_section(md: &mut String, palette: &serde_json::Value) {
    md.push_str("## Color Palette\n\n");

    let sections = [
        ("primary", "Primary"),
        ("secondary", "Secondary"),
        ("neutral", "Neutral"),
    ];

    for (key, title) in &sections {
        if let Some(colors) = palette[key].as_array() {
            if !colors.is_empty() {
                md.push_str(&format!("### {} Colors\n\n", title));
                md.push_str("| Token | Hex | RGB | Usage |\n");
                md.push_str("|-------|-----|-----|-------|\n");
                for color in colors {
                    let name = color["name"].as_str().unwrap_or("-");
                    let hex = color["hex"].as_str().unwrap_or("-");
                    let rgb = color["rgb"].as_str().unwrap_or("-");
                    let usage = color["usage"].as_str().unwrap_or("-");
                    md.push_str(&format!("| `--color-{}` | `{}` | {} | {} |\n", name, hex, rgb, usage));
                }
                md.push('\n');
            }
        }
    }

    // Semantic
    if let Some(semantic) = palette.get("semantic") {
        let has_any = ["success", "warning", "error", "info"]
            .iter()
            .any(|k| semantic[*k].as_str().is_some());

        if has_any {
            md.push_str("### Semantic Colors\n\n");
            md.push_str("| Role | Hex |\n");
            md.push_str("|------|-----|\n");
            for key in &["success", "warning", "error", "info"] {
                if let Some(hex) = semantic[*key].as_str() {
                    md.push_str(&format!("| {} | `{}` |\n", key, hex));
                }
            }
            md.push('\n');
        }
    }
}

/// Render typography section.
fn render_typography_section(md: &mut String, typo: &serde_json::Value) {
    md.push_str("## Typography\n\n");

    // Font families
    if let Some(families) = typo["font_families"].as_array() {
        if !families.is_empty() {
            md.push_str("### Font Families\n\n");
            md.push_str("| Family | Category | Weights | Usage |\n");
            md.push_str("|--------|----------|---------|-------|\n");
            for f in families {
                let name = f["name"].as_str().unwrap_or("-");
                let cat = f["category"].as_str().unwrap_or("-");
                let weights = f["weights"].as_array()
                    .map(|w| w.iter().filter_map(|v| v.as_u64()).map(|v| v.to_string()).collect::<Vec<_>>().join(", "))
                    .unwrap_or_else(|| "-".to_string());
                let usage = f["usage"].as_str().unwrap_or("-");
                md.push_str(&format!("| {} | {} | {} | {} |\n", name, cat, weights, usage));
            }
            md.push('\n');
        }
    }

    // Type scale
    if let Some(scale) = typo["scale"].as_array() {
        if !scale.is_empty() {
            md.push_str("### Type Scale\n\n");
            md.push_str("| Token | Size | Weight | Line Height |\n");
            md.push_str("|-------|------|--------|-------------|\n");
            for step in scale {
                let name = step["name"].as_str().unwrap_or("-");
                let size = step["size_px"].as_f64().map(|v| format!("{}px", v)).unwrap_or_else(|| "-".to_string());
                let weight = step["weight"].as_u64().map(|v| v.to_string()).unwrap_or_else(|| "-".to_string());
                let lh = step["line_height"].as_f64().map(|v| format!("{}", v)).unwrap_or_else(|| "-".to_string());
                md.push_str(&format!("| `{}` | {} | {} | {} |\n", name, size, weight, lh));
            }
            md.push('\n');
        }
    }
}
