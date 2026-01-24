//! Claude Vision API client for design system extraction.
//!
//! Downloads an image, sends it to the Claude API with a structured prompt,
//! and parses the response into a `DesignSystem`.

use anyhow::{Context, Result};
use base64::Engine;
use serde::{Deserialize, Serialize};
use tracing::debug;

use super::model::DesignSystem;

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Prompt instructing Claude to extract a complete design system from a screenshot.
const DESIGN_SYSTEM_PROMPT: &str = r##"Analyze this UI screenshot and extract a complete design system. Return ONLY a valid JSON object (no markdown, no explanation) matching this exact structure:

{
  "colors": {
    "primary": [{"name": "primary-500", "hex": "#XXXXXX", "rgb": "R, G, B", "usage": "description"}],
    "secondary": [{"name": "secondary-500", "hex": "#XXXXXX", "rgb": "R, G, B", "usage": "description"}],
    "neutral": [{"name": "neutral-100", "hex": "#XXXXXX", "rgb": "R, G, B", "usage": "description"}],
    "semantic": {"success": "#XXXXXX", "warning": "#XXXXXX", "error": "#XXXXXX", "info": "#XXXXXX"}
  },
  "typography": {
    "font_families": [{"name": "Font Name", "category": "sans-serif", "weights": [400, 700], "usage": "description"}],
    "scale": [{"name": "heading-xl", "size_px": 32, "weight": 700, "line_height": 1.2}],
    "line_heights": [1.2, 1.5, 1.75]
  },
  "spacing": [{"name": "spacing-xs", "value_px": 4}],
  "border_radius": [{"name": "radius-sm", "value_px": 4}],
  "shadows": [{"name": "shadow-sm", "value": "0 1px 2px rgba(0,0,0,0.05)"}],
  "breakpoints": [{"name": "mobile", "min_width_px": 320}],
  "components": [{"name": "Button", "description": "Primary action button", "variants": ["primary", "secondary"], "states": ["default", "hover", "disabled"]}]
}

Instructions:
- Extract ALL visible colors, organized by role (primary brand colors, secondary/accent, neutrals/grays, semantic states)
- For each color, provide a descriptive name following the pattern: role-shade (e.g., primary-500, neutral-100)
- Identify font families and their usage (headings vs body)
- Infer the type scale from visible text sizes (approximate px values)
- Identify consistent spacing patterns (multiples of 4px or 8px)
- Note border radius values used
- Identify shadow styles
- Infer responsive breakpoints from the layout
- List all identifiable UI components with their variants and states
- If you cannot determine a value precisely, make your best estimate based on visual analysis
- Return ONLY the JSON object, nothing else"##;

/// Client for calling Claude API with vision capabilities.
pub struct ClaudeVisionClient {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

#[derive(Serialize)]
struct MessagesRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: Vec<ContentBlock>,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum ContentBlock {
    #[serde(rename = "image")]
    Image { source: ImageSource },
    #[serde(rename = "text")]
    Text { text: String },
}

#[derive(Serialize)]
struct ImageSource {
    #[serde(rename = "type")]
    source_type: String,
    media_type: String,
    data: String,
}

#[derive(Deserialize)]
struct MessagesResponse {
    content: Vec<ResponseContent>,
}

#[derive(Deserialize)]
struct ResponseContent {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

impl ClaudeVisionClient {
    /// Create a new client with the given API key and model.
    pub fn new(api_key: &str, model: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            model: model.to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Download an image from a URL and analyze it to extract a design system.
    pub async fn analyze_image(&self, image_url: &str) -> Result<DesignSystem> {
        // Download the image
        debug!(url = image_url, "Downloading image");
        let response = self.client.get(image_url)
            .send()
            .await
            .context("Failed to download image")?;

        let status = response.status();
        if !status.is_success() {
            anyhow::bail!("Failed to download image: HTTP {}", status);
        }

        // Detect media type
        let media_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.split(';').next().unwrap_or(s).trim().to_string())
            .unwrap_or_else(|| detect_media_type_from_url(image_url));

        let bytes = response.bytes().await
            .context("Failed to read image bytes")?;

        // Encode to base64
        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
        debug!(size = bytes.len(), media_type = %media_type, "Image downloaded");

        // Call Claude API
        let request = MessagesRequest {
            model: self.model.clone(),
            max_tokens: 4096,
            messages: vec![Message {
                role: "user".to_string(),
                content: vec![
                    ContentBlock::Image {
                        source: ImageSource {
                            source_type: "base64".to_string(),
                            media_type: media_type.clone(),
                            data: b64,
                        },
                    },
                    ContentBlock::Text {
                        text: DESIGN_SYSTEM_PROMPT.to_string(),
                    },
                ],
            }],
        };

        debug!(model = %self.model, "Calling Claude Vision API");
        let api_response = self.client
            .post(ANTHROPIC_API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to call Claude API")?;

        let api_status = api_response.status();
        if !api_status.is_success() {
            let error_text = api_response.text().await.unwrap_or_default();
            anyhow::bail!("Claude API error (HTTP {}): {}", api_status, error_text);
        }

        let response_body: MessagesResponse = api_response.json().await
            .context("Failed to parse Claude API response")?;

        // Extract text content
        let text = response_body.content
            .iter()
            .find(|c| c.content_type == "text")
            .and_then(|c| c.text.as_ref())
            .ok_or_else(|| anyhow::anyhow!("No text content in Claude API response"))?;

        // Parse JSON from response (handle possible markdown code blocks)
        let json_str = extract_json(text);

        let design: DesignSystem = serde_json::from_str(&json_str)
            .context("Failed to parse design system JSON from Claude response")?;

        Ok(design)
    }
}

/// Extract JSON from a string that might be wrapped in markdown code blocks.
fn extract_json(text: &str) -> String {
    let trimmed = text.trim();

    // Try to find JSON within ```json ... ``` blocks
    if let Some(start) = trimmed.find("```json") {
        let after_marker = &trimmed[start + 7..];
        if let Some(end) = after_marker.find("```") {
            return after_marker[..end].trim().to_string();
        }
    }

    // Try to find JSON within ``` ... ``` blocks
    if let Some(start) = trimmed.find("```") {
        let after_marker = &trimmed[start + 3..];
        if let Some(end) = after_marker.find("```") {
            return after_marker[..end].trim().to_string();
        }
    }

    // Try to find the first { and last } for a JSON object
    if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}')) {
        if end > start {
            return trimmed[start..=end].to_string();
        }
    }

    trimmed.to_string()
}

/// Detect media type from URL extension.
fn detect_media_type_from_url(url: &str) -> String {
    let lower = url.to_lowercase();
    if lower.contains(".png") {
        "image/png".to_string()
    } else if lower.contains(".jpg") || lower.contains(".jpeg") {
        "image/jpeg".to_string()
    } else if lower.contains(".gif") {
        "image/gif".to_string()
    } else if lower.contains(".webp") {
        "image/webp".to_string()
    } else if lower.contains(".svg") {
        "image/svg+xml".to_string()
    } else {
        "image/png".to_string()
    }
}
