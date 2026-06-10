use anyhow::Result;
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use serde::Deserialize;
use std::path::Path;

use crate::catalog::{BrewEntry, BrewKind};

#[derive(Deserialize, Default)]
struct BrewMeta {
    description: Option<String>,
    #[serde(default)]
    examples: Vec<String>,
    arch: Option<String>,
    bin: Option<String>,
    zsh_plugin: Option<String>,
}

pub fn parse(path: &Path) -> Result<Vec<BrewEntry>> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return Ok(vec![]);
    };
    let mut entries = Vec::new();
    let mut current_kind = BrewKind::Formula;
    let mut current_name: Option<String> = None;
    let mut in_code_block = false;
    let mut code_buf = String::new();
    let mut capture_text = false;
    let mut text_buf = String::new();

    let parser = Parser::new(&content);
    for event in parser {
        match event {
            Event::Start(Tag::Heading {
                level: HeadingLevel::H1,
                ..
            }) => {
                capture_text = true;
                text_buf.clear();
            }
            Event::End(TagEnd::Heading(HeadingLevel::H1)) => {
                capture_text = false;
                current_kind = match text_buf.trim().to_lowercase().as_str() {
                    "formulae" => BrewKind::Formula,
                    "casks" => BrewKind::Cask,
                    "taps" => BrewKind::Tap,
                    _ => BrewKind::Formula,
                };
            }
            Event::Start(Tag::Heading {
                level: HeadingLevel::H2,
                ..
            }) => {
                capture_text = true;
                text_buf.clear();
            }
            Event::End(TagEnd::Heading(HeadingLevel::H2)) => {
                capture_text = false;
                current_name = Some(text_buf.trim().to_string());
                code_buf.clear();
            }
            Event::Start(Tag::CodeBlock(_)) => {
                in_code_block = true;
                code_buf.clear();
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                if let Some(name) = current_name.take() {
                    let meta: BrewMeta = serde_yaml::from_str(&code_buf).unwrap_or_default();
                    entries.push(BrewEntry {
                        name,
                        kind: current_kind.clone(),
                        description: meta.description.unwrap_or_default(),
                        examples: meta.examples,
                        arch: meta.arch,
                        bin: meta.bin,
                        zsh_plugin: meta.zsh_plugin,
                    });
                }
            }
            Event::Text(t) if in_code_block => code_buf.push_str(&t),
            Event::Text(t) if capture_text => text_buf.push_str(&t),
            _ => {}
        }
    }
    Ok(entries)
}
