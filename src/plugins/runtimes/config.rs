use crate::catalog::{PackageManager, Runtime};
use anyhow::Result;
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use std::path::Path;

pub fn parse_runtimes(path: &Path) -> Result<Vec<Runtime>> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return Ok(vec![]);
    };
    let first_line = content
        .lines()
        .find(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
        .unwrap_or("");
    let runtimes = first_line
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| {
            let mut parts = s.splitn(2, '@');
            let name = parts.next().unwrap_or("").to_string();
            let version = parts.next().unwrap_or("latest").to_string();
            Runtime { name, version }
        })
        .collect();
    Ok(runtimes)
}

pub fn parse_package_managers(path: &Path) -> Result<Vec<PackageManager>> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return Ok(vec![]);
    };
    let mut pms = Vec::new();
    let mut current_name: Option<String> = None;
    let mut in_code = false;
    let mut code_buf = String::new();
    let mut capture_name = false;
    let mut name_buf = String::new();
    let mut desc_buf = String::new();

    for event in Parser::new(&content) {
        match event {
            Event::Start(Tag::Heading {
                level: HeadingLevel::H1,
                ..
            }) => {
                capture_name = true;
                name_buf.clear();
                desc_buf.clear();
            }
            Event::End(TagEnd::Heading(HeadingLevel::H1)) => {
                capture_name = false;
                current_name = Some(name_buf.trim().to_string());
                code_buf.clear();
            }
            Event::Start(Tag::CodeBlock(_)) => {
                in_code = true;
                code_buf.clear();
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code = false;
                if let Some(name) = current_name.take() {
                    pms.push(PackageManager {
                        name,
                        description: desc_buf.trim().to_string(),
                        install_script: code_buf.trim().to_string(),
                    });
                }
            }
            Event::Text(t) if in_code => code_buf.push_str(&t),
            Event::Text(t) if capture_name => name_buf.push_str(&t),
            Event::Text(t) if current_name.is_some() => desc_buf.push_str(&t),
            _ => {}
        }
    }
    Ok(pms)
}
