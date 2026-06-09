use super::{ApmStack, ClaudeSettings};
use anyhow::Result;
use gray_matter::{engine::YAML, Matter};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize, Default)]
struct ClaudeFrontmatter {
    name: Option<String>,
    description: Option<String>,
    #[serde(rename = "settings-path")]
    settings_path: Option<String>,
    #[serde(rename = "required-variables")]
    required_variables: Option<String>,
}

pub fn parse_claude_settings(path: &Path) -> Result<ClaudeSettings> {
    let content = std::fs::read_to_string(path)?;
    let matter = Matter::<YAML>::new();
    let result = matter.parse(&content);

    let fm: ClaudeFrontmatter = result
        .data
        .and_then(|d| d.deserialize().ok())
        .unwrap_or_default();

    // Extract first JSON code block from the body
    let json_body = extract_first_code_block(&result.content);

    Ok(ClaudeSettings {
        name: fm.name.unwrap_or_default(),
        description: fm.description.unwrap_or_default(),
        settings_path: fm.settings_path.unwrap_or_default(),
        required_variables: fm
            .required_variables
            .map(|s| s.split('|').map(str::to_string).collect())
            .unwrap_or_default(),
        json_body,
    })
}

pub fn parse_apm_stack(name: &str, stack_dir: &Path) -> Result<Option<ApmStack>> {
    let apm_path = stack_dir.join("apm.yml");
    if !apm_path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&apm_path)?;
    Ok(Some(ApmStack {
        name: name.to_string(),
        apm_yml_content: content,
    }))
}

fn extract_first_code_block(markdown: &str) -> String {
    let mut in_code = false;
    let mut buf = String::new();
    for event in Parser::new(markdown) {
        match event {
            Event::Start(Tag::CodeBlock(_)) => { in_code = true; buf.clear(); }
            Event::End(TagEnd::CodeBlock) if in_code => break,
            Event::Text(t) if in_code => buf.push_str(&t),
            _ => {}
        }
    }
    buf.trim().to_string()
}
