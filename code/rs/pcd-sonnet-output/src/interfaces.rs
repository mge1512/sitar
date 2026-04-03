// interfaces.rs — Trait definitions + production and test-double implementations
#![allow(dead_code)]
#![allow(unused_variables)]

use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

use crate::types::{FileInfo, PackageRecord, ChangedConfigFileRecord, SitarManifest};

// ---------------------------------------------------------------------------
// Filesystem trait
// ---------------------------------------------------------------------------

pub trait Filesystem: Send + Sync {
    fn read_file(&self, path: &str) -> Result<String, std::io::Error>;
    fn read_file_limited(&self, path: &str, limit: usize) -> Result<String, std::io::Error>;
    fn glob(&self, pattern: &str) -> Result<Vec<String>, Box<dyn std::error::Error>>;
    fn exists(&self, path: &str) -> bool;
    fn is_executable(&self, path: &str) -> bool;
    fn stat(&self, path: &str) -> Result<FileInfo, std::io::Error>;
    fn read_dir(&self, path: &str) -> Result<Vec<String>, std::io::Error>;
    fn is_dir(&self, path: &str) -> bool;
}

// ---------------------------------------------------------------------------
// OSFilesystem — production implementation
// ---------------------------------------------------------------------------

pub struct OSFilesystem;

impl Filesystem for OSFilesystem {
    fn read_file(&self, path: &str) -> Result<String, std::io::Error> {
        fs::read_to_string(path)
    }

    fn read_file_limited(&self, path: &str, limit: usize) -> Result<String, std::io::Error> {
        use std::io::Read;
        let mut f = fs::File::open(path)?;
        let mut buf = vec![0u8; limit];
        let n = f.read(&mut buf)?;
        buf.truncate(n);
        Ok(String::from_utf8_lossy(&buf).into_owned())
    }

    fn glob(&self, pattern: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();
        // Simple glob: handle trailing /* by listing directory
        if let Some(dir) = pattern.strip_suffix("/*") {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_file() {
                        results.push(p.to_string_lossy().into_owned());
                    }
                }
            }
        } else if pattern.contains('*') {
            // Walk parent dir and match
            let path = std::path::Path::new(pattern);
            if let Some(parent) = path.parent() {
                if let Some(file_pat) = path.file_name() {
                    let pat_str = file_pat.to_string_lossy();
                    let prefix = pat_str.split('*').next().unwrap_or("");
                    let suffix = if let Some(s) = pat_str.split('*').last() { s } else { "" };
                    if let Ok(entries) = fs::read_dir(parent) {
                        for entry in entries.flatten() {
                            let name = entry.file_name();
                            let name_str = name.to_string_lossy();
                            if name_str.starts_with(prefix) && name_str.ends_with(suffix) {
                                results.push(entry.path().to_string_lossy().into_owned());
                            }
                        }
                    }
                }
            }
        } else {
            if std::path::Path::new(pattern).exists() {
                results.push(pattern.to_string());
            }
        }
        Ok(results)
    }

    fn exists(&self, path: &str) -> bool {
        std::path::Path::new(path).exists()
    }

    fn is_executable(&self, path: &str) -> bool {
        let p = std::path::Path::new(path);
        if !p.exists() { return false; }
        if let Ok(meta) = p.metadata() {
            let mode = meta.permissions().mode();
            return mode & 0o111 != 0;
        }
        false
    }

    fn stat(&self, path: &str) -> Result<FileInfo, std::io::Error> {
        use std::os::unix::fs::MetadataExt;
        let meta = fs::metadata(path)?;
        let mode = meta.permissions().mode();
        let mtime = {
            let t = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            let secs = t.duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap_or_default().as_secs();
            format!("{}", secs)
        };
        Ok(FileInfo {
            uid:  meta.uid(),
            gid:  meta.gid(),
            mode: format!("{:04o}", mode & 0o7777),
            size: meta.len(),
            mtime,
        })
    }

    fn read_dir(&self, path: &str) -> Result<Vec<String>, std::io::Error> {
        let mut entries = Vec::new();
        for e in fs::read_dir(path)? {
            let e = e?;
            entries.push(e.path().to_string_lossy().into_owned());
        }
        entries.sort();
        Ok(entries)
    }

    fn is_dir(&self, path: &str) -> bool {
        std::path::Path::new(path).is_dir()
    }
}

// ---------------------------------------------------------------------------
// CommandRunner trait
// ---------------------------------------------------------------------------

pub trait CommandRunner: Send + Sync {
    fn run(&self, cmd: &str, args: &[&str])
        -> Result<(String, String), Box<dyn std::error::Error>>;
}

// ---------------------------------------------------------------------------
// OSCommandRunner — production implementation
// ---------------------------------------------------------------------------

pub struct OSCommandRunner;

impl CommandRunner for OSCommandRunner {
    fn run(&self, cmd: &str, args: &[&str])
        -> Result<(String, String), Box<dyn std::error::Error>>
    {
        let output = Command::new(cmd)
            .args(args)
            .env("PATH", "/sbin:/bin:/usr/bin:/usr/sbin")
            .output()?;
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        if output.status.success() {
            Ok((stdout, stderr))
        } else {
            Err(format!("{} failed (exit {}): {}", cmd, output.status, stderr.trim()).into())
        }
    }
}

// ---------------------------------------------------------------------------
// Renderer trait
// ---------------------------------------------------------------------------

pub trait Renderer: Send + Sync {
    fn header(&self, manifest: &SitarManifest) -> String;
    fn toc(&self, sections: &[String]) -> String;
    fn section(&self, title: &str, level: u8, content: &str) -> String;
    fn footer(&self) -> String;
    fn escape(&self, raw: &str) -> String;
}

// ---------------------------------------------------------------------------
// HTMLRenderer
// ---------------------------------------------------------------------------

pub struct HtmlRenderer;

impl Renderer for HtmlRenderer {
    fn header(&self, manifest: &SitarManifest) -> String {
        let hostname = &manifest.meta.hostname;
        let collected_at = &manifest.meta.collected_at;
        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<title>SITAR — {hostname}</title>
<style>
body {{ font-family: monospace; margin: 2em; background: #fff; color: #000; }}
h1 {{ border-bottom: 2px solid #333; }}
h2 {{ border-bottom: 1px solid #aaa; }}
table {{ border-collapse: collapse; margin-bottom: 1em; width: 100%; }}
th {{ background: #ddd; text-align: left; padding: 4px 8px; }}
td {{ padding: 2px 8px; border-bottom: 1px solid #eee; vertical-align: top; }}
pre {{ background: #f4f4f4; padding: 1em; overflow-x: auto; }}
</style>
</head>
<body>
<h1>SITAR &#x2014; System InformaTion At Runtime</h1>
<p>Hostname: {hostname} &mdash; Date: {collected_at}</p>
"#,
            hostname = self.escape(hostname),
            collected_at = self.escape(collected_at),
        )
    }

    fn toc(&self, sections: &[String]) -> String {
        let mut s = String::from("<nav><ol>\n");
        for (i, sec) in sections.iter().enumerate() {
            let anchor = sec.to_lowercase().replace(' ', "-");
            s.push_str(&format!("<li><a href=\"#{anchor}\">{sec}</a></li>\n",
                anchor = anchor, sec = self.escape(sec)));
        }
        s.push_str("</ol></nav>\n");
        s
    }

    fn section(&self, title: &str, level: u8, content: &str) -> String {
        let anchor = title.to_lowercase().replace(' ', "-");
        let tag = if level <= 1 { "h2" } else { "h3" };
        if content.is_empty() {
            return String::new();
        }
        format!(
            "<{tag} id=\"{anchor}\">{title}</{tag}>\n{content}\n",
            tag = tag,
            anchor = anchor,
            title = self.escape(title),
            content = content,
        )
    }

    fn footer(&self) -> String {
        "</body>\n</html>\n".to_string()
    }

    fn escape(&self, raw: &str) -> String {
        raw.replace('&', "&amp;")
           .replace('<', "&lt;")
           .replace('>', "&gt;")
           .replace('"', "&quot;")
    }
}

// ---------------------------------------------------------------------------
// TeXRenderer
// ---------------------------------------------------------------------------

pub struct TexRenderer;

impl Renderer for TexRenderer {
    fn header(&self, manifest: &SitarManifest) -> String {
        let hostname = &manifest.meta.hostname;
        let collected_at = &manifest.meta.collected_at;
        format!(
            r#"\documentclass{{scrartcl}}
\usepackage{{longtable}}
\usepackage{{verbatim}}
\usepackage{{multicol}}
\usepackage[T1]{{fontenc}}
\usepackage[utf8]{{inputenc}}
\title{{SITAR --- System InformaTion At Runtime}}
\author{{{hostname}}}
\date{{{collected_at}}}
\begin{{document}}
\maketitle
\tableofcontents
\newpage
"#,
            hostname = self.escape(hostname),
            collected_at = self.escape(collected_at),
        )
    }

    fn toc(&self, _sections: &[String]) -> String {
        // \tableofcontents already in header
        String::new()
    }

    fn section(&self, title: &str, level: u8, content: &str) -> String {
        if content.is_empty() {
            return String::new();
        }
        let cmd = if level <= 1 { "\\section" } else { "\\subsection" };
        format!("{cmd}{{{title}}}\n{content}\n",
            cmd = cmd,
            title = self.escape(title),
            content = content,
        )
    }

    fn footer(&self) -> String {
        "\\end{document}\n".to_string()
    }

    fn escape(&self, raw: &str) -> String {
        raw.replace('\\', "\\textbackslash{}")
           .replace('{', "\\{")
           .replace('}', "\\}")
           .replace('_', "\\_")
           .replace('#', "\\#")
           .replace('%', "\\%")
           .replace('&', "\\&")
           .replace('<', "$<$")
           .replace('>', "$>$")
           .replace('~', "\\textasciitilde{}")
           .replace('^', "\\textasciicircum{}")
    }
}

// ---------------------------------------------------------------------------
// DocBookRenderer
// ---------------------------------------------------------------------------

pub struct DocBookRenderer;

impl Renderer for DocBookRenderer {
    fn header(&self, manifest: &SitarManifest) -> String {
        let hostname = &manifest.meta.hostname;
        let collected_at = &manifest.meta.collected_at;
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE article PUBLIC "-//OASIS//DTD DocBook XML V4.5//EN"
  "http://www.oasis-open.org/docbook/xml/4.5/docbookx.dtd">
<article>
<title>SITAR &#x2014; System InformaTion At Runtime</title>
<articleinfo>
  <subtitle>Hostname: {hostname}</subtitle>
  <date>{collected_at}</date>
</articleinfo>
"#,
            hostname = self.escape(hostname),
            collected_at = self.escape(collected_at),
        )
    }

    fn toc(&self, _sections: &[String]) -> String {
        String::new()
    }

    fn section(&self, title: &str, level: u8, content: &str) -> String {
        if content.is_empty() {
            return String::new();
        }
        format!(
            "<section>\n<title>{title}</title>\n{content}\n</section>\n",
            title = self.escape(title),
            content = content,
        )
    }

    fn footer(&self) -> String {
        "</article>\n".to_string()
    }

    fn escape(&self, raw: &str) -> String {
        raw.replace('&', "&amp;")
           .replace('<', "&lt;")
           .replace('>', "&gt;")
    }
}

// ---------------------------------------------------------------------------
// MarkdownRenderer
// ---------------------------------------------------------------------------

pub struct MarkdownRenderer;

impl Renderer for MarkdownRenderer {
    fn header(&self, manifest: &SitarManifest) -> String {
        let hostname = &manifest.meta.hostname;
        let collected_at = &manifest.meta.collected_at;
        format!(
            "# SITAR — System InformaTion At Runtime\n\nHostname: {hostname}  \nDate: {collected_at}\n\n",
            hostname = hostname,
            collected_at = collected_at,
        )
    }

    fn toc(&self, _sections: &[String]) -> String {
        String::new()
    }

    fn section(&self, title: &str, level: u8, content: &str) -> String {
        if content.is_empty() {
            return String::new();
        }
        let heading = if level <= 1 { "##" } else { "###" };
        format!("{heading} {title}\n\n{content}\n",
            heading = heading,
            title = title,
            content = content,
        )
    }

    fn footer(&self) -> String {
        String::new()
    }

    fn escape(&self, raw: &str) -> String {
        raw.replace('|', "\\|")
    }
}

// ---------------------------------------------------------------------------
// JSONRenderer — handled by render_json, not this interface
// (placeholder struct for completeness)
// ---------------------------------------------------------------------------

pub struct JsonRenderer;

impl Renderer for JsonRenderer {
    fn header(&self, _manifest: &SitarManifest) -> String { "{".to_string() }
    fn toc(&self, _sections: &[String]) -> String { String::new() }
    fn section(&self, _title: &str, _level: u8, _content: &str) -> String { String::new() }
    fn footer(&self) -> String { "}\n".to_string() }
    fn escape(&self, raw: &str) -> String {
        serde_json::to_string(raw).unwrap_or_else(|_| raw.to_string())
    }
}

// ---------------------------------------------------------------------------
// PackageBackend trait
// ---------------------------------------------------------------------------

pub trait PackageBackend: Send + Sync {
    fn list_installed(&self) -> Result<Vec<PackageRecord>, Box<dyn std::error::Error>>;
    fn query_file(&self, path: &str) -> Result<String, Box<dyn std::error::Error>>;
    fn verify_all(&self) -> Result<Vec<ChangedConfigFileRecord>, Box<dyn std::error::Error>>;
    fn verify_package(&self, name: &str) -> Result<Vec<ChangedConfigFileRecord>, Box<dyn std::error::Error>>;
}

// ---------------------------------------------------------------------------
// RPMBackend — production
// ---------------------------------------------------------------------------

pub struct RpmBackend {
    pub rpm_cmd: String,
    pub runner:  Box<dyn CommandRunner>,
}

impl PackageBackend for RpmBackend {
    fn list_installed(&self) -> Result<Vec<PackageRecord>, Box<dyn std::error::Error>> {
        Ok(Vec::new()) // filled in collect_pkg.rs
    }
    fn query_file(&self, _path: &str) -> Result<String, Box<dyn std::error::Error>> {
        Ok(String::new())
    }
    fn verify_all(&self) -> Result<Vec<ChangedConfigFileRecord>, Box<dyn std::error::Error>> {
        Ok(Vec::new())
    }
    fn verify_package(&self, _name: &str) -> Result<Vec<ChangedConfigFileRecord>, Box<dyn std::error::Error>> {
        Ok(Vec::new())
    }
}

// ---------------------------------------------------------------------------
// DpkgBackend — production
// ---------------------------------------------------------------------------

pub struct DpkgBackend {
    pub status_path: String,
}

impl PackageBackend for DpkgBackend {
    fn list_installed(&self) -> Result<Vec<PackageRecord>, Box<dyn std::error::Error>> {
        Ok(Vec::new()) // filled in collect_pkg.rs
    }
    fn query_file(&self, _path: &str) -> Result<String, Box<dyn std::error::Error>> {
        Ok(String::new())
    }
    fn verify_all(&self) -> Result<Vec<ChangedConfigFileRecord>, Box<dyn std::error::Error>> {
        Ok(Vec::new())
    }
    fn verify_package(&self, _name: &str) -> Result<Vec<ChangedConfigFileRecord>, Box<dyn std::error::Error>> {
        Ok(Vec::new())
    }
}

// ---------------------------------------------------------------------------
// NullBackend — fallback when no package manager found
// ---------------------------------------------------------------------------

pub struct NullBackend;

impl PackageBackend for NullBackend {
    fn list_installed(&self) -> Result<Vec<PackageRecord>, Box<dyn std::error::Error>> { Ok(Vec::new()) }
    fn query_file(&self, _: &str) -> Result<String, Box<dyn std::error::Error>> { Ok(String::new()) }
    fn verify_all(&self) -> Result<Vec<ChangedConfigFileRecord>, Box<dyn std::error::Error>> { Ok(Vec::new()) }
    fn verify_package(&self, _: &str) -> Result<Vec<ChangedConfigFileRecord>, Box<dyn std::error::Error>> { Ok(Vec::new()) }
}

// ---------------------------------------------------------------------------
// FakeFilesystem — test double
// ---------------------------------------------------------------------------

#[cfg(test)]
pub struct FakeFilesystem {
    pub files:       HashMap<String, String>,
    pub executables: Vec<String>,
    pub dirs:        Vec<String>,
    pub stat_info:   HashMap<String, FileInfo>,
}

#[cfg(test)]
impl FakeFilesystem {
    pub fn new() -> Self {
        FakeFilesystem {
            files:       HashMap::new(),
            executables: Vec::new(),
            dirs:        Vec::new(),
            stat_info:   HashMap::new(),
        }
    }
}

#[cfg(test)]
impl Filesystem for FakeFilesystem {
    fn read_file(&self, path: &str) -> Result<String, std::io::Error> {
        self.files.get(path)
            .cloned()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, path))
    }

    fn read_file_limited(&self, path: &str, limit: usize) -> Result<String, std::io::Error> {
        let content = self.read_file(path)?;
        Ok(content.chars().take(limit).collect())
    }

    fn glob(&self, pattern: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let prefix = pattern.trim_end_matches('*').trim_end_matches('/');
        let mut results: Vec<String> = self.files.keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect();
        results.sort();
        Ok(results)
    }

    fn exists(&self, path: &str) -> bool {
        self.files.contains_key(path) || self.dirs.contains(&path.to_string())
    }

    fn is_executable(&self, path: &str) -> bool {
        self.executables.contains(&path.to_string())
    }

    fn stat(&self, path: &str) -> Result<FileInfo, std::io::Error> {
        self.stat_info.get(path)
            .cloned()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, path))
    }

    fn read_dir(&self, path: &str) -> Result<Vec<String>, std::io::Error> {
        let prefix = if path.ends_with('/') { path.to_string() } else { format!("{}/", path) };
        let mut entries: Vec<String> = self.files.keys()
            .filter(|k| k.starts_with(&prefix) && !k[prefix.len()..].contains('/'))
            .cloned()
            .collect();
        entries.sort();
        Ok(entries)
    }

    fn is_dir(&self, path: &str) -> bool {
        self.dirs.contains(&path.to_string())
    }
}

// ---------------------------------------------------------------------------
// FakeCommandRunner — test double
// ---------------------------------------------------------------------------

#[cfg(test)]
pub struct FakeCommandRunner {
    pub responses: HashMap<String, (String, String)>,
}

#[cfg(test)]
impl FakeCommandRunner {
    pub fn new() -> Self {
        FakeCommandRunner { responses: HashMap::new() }
    }
}

#[cfg(test)]
impl CommandRunner for FakeCommandRunner {
    fn run(&self, cmd: &str, _args: &[&str])
        -> Result<(String, String), Box<dyn std::error::Error>>
    {
        Ok(self.responses.get(cmd)
            .cloned()
            .unwrap_or_else(|| (String::new(), String::new())))
    }
}

// ---------------------------------------------------------------------------
// FakeRenderer — test double
// ---------------------------------------------------------------------------

#[cfg(test)]
pub struct FakeRenderer {
    pub sections: std::sync::Mutex<Vec<String>>,
}

#[cfg(test)]
impl FakeRenderer {
    pub fn new() -> Self {
        FakeRenderer { sections: std::sync::Mutex::new(Vec::new()) }
    }
}

#[cfg(test)]
impl Renderer for FakeRenderer {
    fn header(&self, _: &SitarManifest) -> String { "<fake-header>".to_string() }
    fn toc(&self, _: &[String]) -> String { String::new() }
    fn section(&self, title: &str, _: u8, content: &str) -> String {
        self.sections.lock().unwrap().push(title.to_string());
        format!("[SECTION: {}]\n{}", title, content)
    }
    fn footer(&self) -> String { "</fake-footer>".to_string() }
    fn escape(&self, raw: &str) -> String { raw.to_string() }
}

// ---------------------------------------------------------------------------
// FakePackageBackend — test double
// ---------------------------------------------------------------------------

#[cfg(test)]
pub struct FakePackageBackend {
    pub packages:     Vec<PackageRecord>,
    pub file_owners:  HashMap<String, String>,
    pub verify_result: Vec<ChangedConfigFileRecord>,
}

#[cfg(test)]
impl FakePackageBackend {
    pub fn new() -> Self {
        FakePackageBackend {
            packages:      Vec::new(),
            file_owners:   HashMap::new(),
            verify_result: Vec::new(),
        }
    }
}

#[cfg(test)]
impl PackageBackend for FakePackageBackend {
    fn list_installed(&self) -> Result<Vec<PackageRecord>, Box<dyn std::error::Error>> {
        Ok(self.packages.clone())
    }
    fn query_file(&self, path: &str) -> Result<String, Box<dyn std::error::Error>> {
        Ok(self.file_owners.get(path).cloned().unwrap_or_default())
    }
    fn verify_all(&self) -> Result<Vec<ChangedConfigFileRecord>, Box<dyn std::error::Error>> {
        Ok(self.verify_result.clone())
    }
    fn verify_package(&self, _name: &str) -> Result<Vec<ChangedConfigFileRecord>, Box<dyn std::error::Error>> {
        Ok(self.verify_result.clone())
    }
}
