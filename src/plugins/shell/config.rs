use crate::catalog::ShellFunction;
use anyhow::Result;
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use std::path::Path;

pub fn parse(path: &Path) -> Result<Vec<ShellFunction>> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return Ok(vec![]);
    };
    let mut functions = Vec::new();
    let mut current_name: Option<String> = None;
    let mut in_code = false;
    let mut code_buf = String::new();
    let mut desc_buf = String::new();
    let mut capture = false;
    let mut text_buf = String::new();

    for event in Parser::new(&content) {
        match event {
            Event::Start(Tag::Heading { level: HeadingLevel::H1, .. }) => {
                capture = true; text_buf.clear(); desc_buf.clear();
            }
            Event::End(TagEnd::Heading(HeadingLevel::H1)) => {
                capture = false;
                current_name = Some(text_buf.trim().to_string());
            }
            Event::Start(Tag::CodeBlock(_)) => { in_code = true; code_buf.clear(); }
            Event::End(TagEnd::CodeBlock) => {
                in_code = false;
                if let Some(name) = current_name.take() {
                    functions.push(ShellFunction {
                        name,
                        body: code_buf.trim().to_string(),
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
    Ok(functions)
}
