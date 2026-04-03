// render.rs — render BEHAVIOR dispatch
#![allow(dead_code)]
#![allow(unused_variables)]

use std::path::PathBuf;
use crate::interfaces::{HtmlRenderer, TexRenderer, DocBookRenderer, MarkdownRenderer, Renderer};
use crate::render_human::render_human;
use crate::render_json::render_json;
use crate::types::{Config, OutputFormat, SitarManifest};

/// render BEHAVIOR — render a SitarManifest to one or more output files
pub fn render(manifest: &SitarManifest, config: &Config) -> Vec<String> {
    let mut files_written = Vec::new();

    // Step 1a: determine active formats
    let active_formats: Vec<OutputFormat> = match &config.format {
        None => vec![
            OutputFormat::Html,
            OutputFormat::Tex,
            OutputFormat::Sdocbook,
            OutputFormat::Json,
            OutputFormat::Markdown,
        ],
        Some(OutputFormat::All) => vec![
            OutputFormat::Html,
            OutputFormat::Tex,
            OutputFormat::Sdocbook,
            OutputFormat::Json,
            OutputFormat::Markdown,
        ],
        Some(fmt) => vec![fmt.clone()],
    };

    let hostname = &manifest.meta.hostname;

    // Step 1b: determine outdir
    let outdir = if config.outdir.is_empty() && active_formats.len() > 1 {
        // Auto-derive: /tmp/sitar-{hostname}-{YYYYMMDDhh}
        let ts = crate::collect::format_utc_timestamp(
            std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        );
        // Extract date+hour: YYYYMMDDHH from ISO 8601
        let compact = ts.replace('-', "").replace('T', "").replace(':', "").replace('Z', "");
        let datehour = &compact[..10.min(compact.len())];
        format!("/tmp/sitar-{}-{}", hostname, datehour)
    } else if !config.outdir.is_empty() {
        config.outdir.clone()
    } else {
        ".".to_string()
    };

    // Create outdir
    if let Err(e) = std::fs::create_dir_all(&outdir) {
        eprintln!("sitar: render: cannot create output directory {}: {}", outdir, e);
        std::process::exit(1);
    }

    // Step 1c + 2: render each format
    for fmt in &active_formats {
        let outpath = if !config.outfile.is_empty() && active_formats.len() == 1 {
            // Single format with explicit outfile
            let p = PathBuf::from(&config.outfile);
            if p.components().count() == 1 && outdir != "." {
                PathBuf::from(&outdir).join(&config.outfile)
            } else {
                p
            }
        } else {
            let ext = fmt.extension();
            PathBuf::from(&outdir).join(format!("sitar-{}{}", hostname, ext))
        };

        // Create parent dir if needed
        if let Some(parent) = outpath.parent() {
            if !parent.as_os_str().is_empty() {
                let _ = std::fs::create_dir_all(parent);
            }
        }

        let outpath_str = outpath.to_string_lossy().to_string();
        eprintln!("Generating {}...", outpath_str);

        let result = match fmt {
            OutputFormat::Html     => {
                let r = HtmlRenderer;
                render_human(manifest, &r, &outpath_str)
            }
            OutputFormat::Tex      => {
                let r = TexRenderer;
                render_human(manifest, &r, &outpath_str)
            }
            OutputFormat::Sdocbook => {
                let r = DocBookRenderer;
                render_human(manifest, &r, &outpath_str)
            }
            OutputFormat::Markdown => {
                let r = MarkdownRenderer;
                render_human(manifest, &r, &outpath_str)
            }
            OutputFormat::Json     => render_json(manifest, &outpath_str),
            OutputFormat::All      => continue,
        };

        match result {
            Ok(bytes) if bytes > 0 => {
                files_written.push(outpath_str);
            }
            Ok(_) => {
                eprintln!("sitar: render: {} produced 0 bytes", outpath_str);
            }
            Err(e) => {
                eprintln!("sitar: render: failed to write {}: {}", outpath_str, e);
            }
        }
    }

    files_written
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    #[test]
    fn test_render_json_single_format() {
        let mut manifest = SitarManifest::default();
        manifest.meta.format_version = 1;
        manifest.meta.sitar_version  = "0.9.0".to_string();
        manifest.meta.hostname       = "testhost".to_string();
        manifest.general_info.elements.push(GeneralInfoRecord {
            key: "hostname".to_string(), value: "testhost".to_string(),
        });

        let config = Config {
            format:  Some(OutputFormat::Json),
            outfile: "/tmp/sitar-test-render.json".to_string(),
            outdir:  "/tmp".to_string(),
            ..Default::default()
        };

        let files = render(&manifest, &config);
        assert!(!files.is_empty());
        assert!(files[0].ends_with(".json"));
        // Cleanup
        let _ = std::fs::remove_file(&files[0]);
    }
}
