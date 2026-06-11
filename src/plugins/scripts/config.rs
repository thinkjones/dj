use crate::catalog::ScriptInstall;
use anyhow::Result;
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use std::path::Path;

pub fn parse(path: &Path) -> Result<Vec<ScriptInstall>> {
    // The plugin catalog may be split into multiple markdown files by purpose.
    // Use the directory containing the configured file path so any *.md files
    // in the plugin's catalog directory are picked up.
    let dir = if path.is_dir() {
        path
    } else {
        path.parent().unwrap_or(path)
    };

    let mut installs = Vec::new();

    if !dir.exists() {
        return Ok(installs);
    }

    let mut md_files: Vec<_> = std::fs::read_dir(dir)?
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "md" {
                        return Some(path);
                    }
                }
            }
            None
        })
        .collect();
    md_files.sort();

    for file in &md_files {
        installs.extend(parse_file(file)?);
    }

    Ok(installs)
}

fn parse_file(path: &Path) -> Result<Vec<ScriptInstall>> {
    let content = std::fs::read_to_string(path)?;
    let mut installs = Vec::new();
    let mut current_name: Option<String> = None;
    let mut in_code = false;
    let mut code_buf = String::new();
    let mut desc_buf = String::new();
    let mut capture = false;
    let mut text_buf = String::new();

    for event in Parser::new(&content) {
        match event {
            Event::Start(Tag::Heading {
                level: HeadingLevel::H1,
                ..
            }) => {
                capture = true;
                text_buf.clear();
                desc_buf.clear();
            }
            Event::End(TagEnd::Heading(HeadingLevel::H1)) => {
                capture = false;
                current_name = Some(text_buf.trim().to_string());
            }
            Event::Start(Tag::CodeBlock(_)) => {
                in_code = true;
                code_buf.clear();
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code = false;
                if let Some(name) = current_name.take() {
                    installs.push(ScriptInstall {
                        name,
                        install_script: code_buf.trim().to_string(),
                        description: desc_buf.trim().to_string(),
                    });
                }
            }
            Event::Text(t) if in_code => code_buf.push_str(&t),
            Event::Text(t) if capture => text_buf.push_str(&t),
            Event::Text(t) if current_name.is_some() => desc_buf.push_str(&t),
            _ => {}
        }
    }
    Ok(installs)
}
