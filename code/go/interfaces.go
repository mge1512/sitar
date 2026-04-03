package main

import (
	"bytes"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"syscall"
)

// ─────────────────────────────────────────────────────────────────────────────
// FileInfo – thin wrapper returned by Stat so callers don't depend on os.FileInfo
// directly and test doubles can supply their own values.
// ─────────────────────────────────────────────────────────────────────────────

// FileInfo carries the subset of file metadata that sitar needs.
type FileInfo struct {
	Name    string
	Size    int64
	Mode    os.FileMode
	IsDir   bool
	UID     uint32
	GID     uint32
	ModTime int64 // Unix seconds
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. Filesystem interface
// ─────────────────────────────────────────────────────────────────────────────

// Filesystem abstracts all disk I/O so that production code and tests share
// the same call sites.
type Filesystem interface {
	ReadFile(path string) (string, error)
	ReadFileLimited(path string, limit int) (string, error)
	Glob(pattern string) ([]string, error)
	Exists(path string) bool
	IsExecutable(path string) bool
	Stat(path string) (FileInfo, error)
	WalkDir(root string, fn func(path string, isDir bool) error) error
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. OSFilesystem – production implementation
// ─────────────────────────────────────────────────────────────────────────────

// OSFilesystem delegates every Filesystem call to the real operating system.
type OSFilesystem struct{}

// ReadFile reads the entire content of path and returns it as a string.
func (fs *OSFilesystem) ReadFile(path string) (string, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return "", err
	}
	return string(data), nil
}

// ReadFileLimited reads at most limit bytes from path.
func (fs *OSFilesystem) ReadFileLimited(path string, limit int) (string, error) {
	f, err := os.Open(path)
	if err != nil {
		return "", err
	}
	defer f.Close()

	buf := make([]byte, limit)
	n, err := io.ReadFull(f, buf)
	if err != nil && err != io.ErrUnexpectedEOF && err != io.EOF {
		return "", err
	}
	return string(buf[:n]), nil
}

// Glob returns the names of all files matching pattern using filepath.Glob.
func (fs *OSFilesystem) Glob(pattern string) ([]string, error) {
	return filepath.Glob(pattern)
}

// Exists reports whether path exists on the filesystem.
func (fs *OSFilesystem) Exists(path string) bool {
	_, err := os.Stat(path)
	return err == nil
}

// IsExecutable reports whether path exists and has at least one executable bit set.
func (fs *OSFilesystem) IsExecutable(path string) bool {
	info, err := os.Stat(path)
	if err != nil {
		return false
	}
	return info.Mode()&0111 != 0
}

// Stat returns a FileInfo for path, including UID/GID extracted via syscall.
func (fs *OSFilesystem) Stat(path string) (FileInfo, error) {
	info, err := os.Stat(path)
	if err != nil {
		return FileInfo{}, err
	}

	fi := FileInfo{
		Name:    info.Name(),
		Size:    info.Size(),
		Mode:    info.Mode(),
		IsDir:   info.IsDir(),
		ModTime: info.ModTime().Unix(),
	}

	// Extract UID/GID from the underlying syscall.Stat_t when available.
	if sys, ok := info.Sys().(*syscall.Stat_t); ok {
		fi.UID = sys.Uid
		fi.GID = sys.Gid
	}

	return fi, nil
}

// WalkDir walks the directory tree rooted at root, calling fn for each file
// and directory encountered.
func (fs *OSFilesystem) WalkDir(root string, fn func(path string, isDir bool) error) error {
	return filepath.WalkDir(root, func(path string, d os.DirEntry, err error) error {
		if err != nil {
			return err
		}
		return fn(path, d.IsDir())
	})
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. FakeFilesystem – test double
// ─────────────────────────────────────────────────────────────────────────────

// FakeFilesystem is an in-memory Filesystem implementation for use in tests.
type FakeFilesystem struct {
	Files       map[string]string   // path -> content
	Dirs        map[string]bool     // path -> exists
	Executables map[string]bool     // path -> executable
	StatInfo    map[string]FileInfo // path -> stat
}

// ReadFile returns the content of path from the Files map.
func (fs *FakeFilesystem) ReadFile(path string) (string, error) {
	content, ok := fs.Files[path]
	if !ok {
		return "", fmt.Errorf("fake: file not found: %s", path)
	}
	return content, nil
}

// ReadFileLimited returns up to limit bytes of the content of path.
func (fs *FakeFilesystem) ReadFileLimited(path string, limit int) (string, error) {
	content, err := fs.ReadFile(path)
	if err != nil {
		return "", err
	}
	if len(content) > limit {
		return content[:limit], nil
	}
	return content, nil
}

// Glob matches all keys in Files against pattern using filepath.Match and
// returns the matching paths in iteration order.
func (fs *FakeFilesystem) Glob(pattern string) ([]string, error) {
	var matches []string
	for path := range fs.Files {
		matched, err := filepath.Match(pattern, path)
		if err != nil {
			return nil, err
		}
		if matched {
			matches = append(matches, path)
		}
	}
	return matches, nil
}

// Exists reports whether path is present in Files or Dirs.
func (fs *FakeFilesystem) Exists(path string) bool {
	if _, ok := fs.Files[path]; ok {
		return true
	}
	if exists, ok := fs.Dirs[path]; ok {
		return exists
	}
	return false
}

// IsExecutable reports whether path is marked executable in the Executables map.
func (fs *FakeFilesystem) IsExecutable(path string) bool {
	return fs.Executables[path]
}

// Stat returns the FileInfo registered in StatInfo for path, or a synthesised
// one derived from Files/Dirs when no explicit entry is present.
func (fs *FakeFilesystem) Stat(path string) (FileInfo, error) {
	if fi, ok := fs.StatInfo[path]; ok {
		return fi, nil
	}
	// Synthesise from Files map.
	if content, ok := fs.Files[path]; ok {
		return FileInfo{
			Name:  filepath.Base(path),
			Size:  int64(len(content)),
			Mode:  0644,
			IsDir: false,
		}, nil
	}
	// Synthesise from Dirs map.
	if exists, ok := fs.Dirs[path]; ok && exists {
		return FileInfo{
			Name:  filepath.Base(path),
			Mode:  0755,
			IsDir: true,
		}, nil
	}
	return FileInfo{}, fmt.Errorf("fake: stat: no such file or directory: %s", path)
}

// WalkDir iterates over all entries in Files and Dirs whose paths are rooted
// under root, calling fn for each one.  Directories are visited before the
// files they contain.
func (fs *FakeFilesystem) WalkDir(root string, fn func(path string, isDir bool) error) error {
	// Emit the root itself first (if it is a known directory).
	if exists, ok := fs.Dirs[root]; ok && exists {
		if err := fn(root, true); err != nil {
			return err
		}
	}

	// Emit sub-directories.
	for path, exists := range fs.Dirs {
		if !exists {
			continue
		}
		if path == root {
			continue
		}
		if strings.HasPrefix(path, root+"/") || root == "." {
			if err := fn(path, true); err != nil {
				return err
			}
		}
	}

	// Emit files.
	for path := range fs.Files {
		if strings.HasPrefix(path, root+"/") || root == "." || path == root {
			if err := fn(path, false); err != nil {
				return err
			}
		}
	}

	return nil
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. CommandRunner interface
// ─────────────────────────────────────────────────────────────────────────────

// CommandRunner abstracts external process execution so that production code
// and tests share the same call sites.
type CommandRunner interface {
	Run(cmd string, args []string) (stdout string, stderr string, err error)
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. OSCommandRunner – production implementation
// ─────────────────────────────────────────────────────────────────────────────

// OSCommandRunner executes real OS commands using os/exec.
type OSCommandRunner struct{}

// Run executes cmd with args, capturing stdout and stderr separately.
// It temporarily restricts PATH to a known-safe set of directories to avoid
// PATH-injection attacks.
func (r *OSCommandRunner) Run(cmd string, args []string) (string, string, error) {
	oldPath := os.Getenv("PATH")
	os.Setenv("PATH", "/sbin:/bin:/usr/bin:/usr/sbin")
	defer os.Setenv("PATH", oldPath)

	c := exec.Command(cmd, args...)
	var stdout, stderr bytes.Buffer
	c.Stdout = &stdout
	c.Stderr = &stderr
	err := c.Run()
	return stdout.String(), stderr.String(), err
}

// ─────────────────────────────────────────────────────────────────────────────
// 6. FakeCommandRunner – test double
// ─────────────────────────────────────────────────────────────────────────────

// CommandResponse holds the canned response for a single command invocation.
type CommandResponse struct {
	Stdout string
	Stderr string
	Err    error
}

// FakeCommandRunner returns pre-configured responses for known command strings
// and zero values for anything unregistered.
type FakeCommandRunner struct {
	Responses map[string]CommandResponse // "cmd arg1 arg2" -> response
}

// Run looks up the command string (cmd + space-joined args) in Responses.
// If no entry is found it returns empty strings and a nil error.
func (r *FakeCommandRunner) Run(cmd string, args []string) (string, string, error) {
	key := strings.Join(append([]string{cmd}, args...), " ")
	if resp, ok := r.Responses[key]; ok {
		return resp.Stdout, resp.Stderr, resp.Err
	}
	return "", "", nil
}

// ─────────────────────────────────────────────────────────────────────────────
// 7. SitarRenderer interface
// ─────────────────────────────────────────────────────────────────────────────

// SitarManifest is forward-declared here so that the Renderer interface can
// reference it; the full struct definition lives in manifest.go.
// (If manifest.go is compiled in the same package this declaration is omitted
// and the compiler will use the canonical one.)

// SitarRenderer is the output-format abstraction used by the report generator.
// Named SitarRenderer (not Renderer) to avoid collisions with other types in
// the package.
type SitarRenderer interface {
	Header(manifest *SitarManifest) string
	TOC(sections []string) string
	Section(title string, level int, content string) string
	Footer() string
	Escape(raw string) string
}

// ─────────────────────────────────────────────────────────────────────────────
// 8. HTMLRenderer – production implementation
// ─────────────────────────────────────────────────────────────────────────────

// HTMLRenderer emits a self-contained HTML5 document with inline CSS.
type HTMLRenderer struct{}

// Header returns the HTML preamble including DOCTYPE, <head> with inline CSS,
// an opening <body>, and an <h1> title derived from the manifest.
func (h *HTMLRenderer) Header(manifest *SitarManifest) string {
	title := "SITAR — System InformaTion At Runtime"
	if manifest != nil && manifest.Meta.Hostname != "" {
		title = fmt.Sprintf("SITAR Report: %s", manifest.Meta.Hostname)
	}
	return fmt.Sprintf(`<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>%s</title>
<style>
  body { font-family: sans-serif; margin: 2em; color: #222; }
  h1   { border-bottom: 2px solid #444; padding-bottom: .3em; }
  h2   { border-bottom: 1px solid #ccc; padding-bottom: .2em; margin-top: 2em; }
  pre  { background: #f5f5f5; padding: 1em; overflow-x: auto; }
  table{ border-collapse: collapse; width: 100%%; }
  th,td{ border: 1px solid #ccc; padding: .4em .8em; text-align: left; }
  th   { background: #eee; }
  a    { color: #0056b3; }
  nav  { background: #f0f0f0; padding: 1em; margin-bottom: 2em; }
  nav ol{ margin: 0; padding-left: 1.5em; }
</style>
</head>
<body>
<h1>%s</h1>
`, h.Escape(title), h.Escape(title))
}

// TOC returns a <nav> element containing a numbered anchor list of sections.
func (h *HTMLRenderer) TOC(sections []string) string {
	if len(sections) == 0 {
		return ""
	}
	var sb strings.Builder
	sb.WriteString("<nav>\n<ol>\n")
	for _, s := range sections {
		anchor := tocAnchor(s)
		sb.WriteString(fmt.Sprintf("  <li><a href=\"#%s\">%s</a></li>\n",
			anchor, h.Escape(s)))
	}
	sb.WriteString("</ol>\n</nav>\n")
	return sb.String()
}

// Section returns an HTML heading (h1 for level 1, h2 for level ≥ 2) followed
// by the pre-rendered content block.
func (h *HTMLRenderer) Section(title string, level int, content string) string {
	tag := "h2"
	if level == 1 {
		tag = "h1"
	}
	anchor := tocAnchor(title)
	return fmt.Sprintf("<%s id=\"%s\">%s</%s>\n%s\n",
		tag, anchor, h.Escape(title), tag, content)
}

// Footer closes the <body> and <html> elements.
func (h *HTMLRenderer) Footer() string {
	return "</body>\n</html>\n"
}

// Escape replaces &, <, and > with their HTML entity equivalents.
func (h *HTMLRenderer) Escape(raw string) string {
	raw = strings.ReplaceAll(raw, "&", "&amp;")
	raw = strings.ReplaceAll(raw, "<", "&lt;")
	raw = strings.ReplaceAll(raw, ">", "&gt;")
	return raw
}

// ─────────────────────────────────────────────────────────────────────────────
// 9. TeXRenderer – production implementation
// ─────────────────────────────────────────────────────────────────────────────

// TeXRenderer emits a LaTeX document using the KOMA-Script scrartcl class.
type TeXRenderer struct{}

// Header returns the LaTeX preamble: \documentclass, package imports, and
// \begin{document}.
func (t *TeXRenderer) Header(manifest *SitarManifest) string {
	title := "SITAR --- System InformaTion At Runtime"
	host := ""
	if manifest != nil && manifest.Meta.Hostname != "" {
		host = fmt.Sprintf("\n\\subtitle{%s}", t.Escape(manifest.Meta.Hostname))
	}
	return fmt.Sprintf(`\documentclass[a4paper,11pt]{scrartcl}
\usepackage[T1]{fontenc}
\usepackage[utf8]{inputenc}
\usepackage{lmodern}
\usepackage{longtable}
\usepackage{verbatim}
\usepackage{multicol}
\usepackage{hyperref}
\usepackage{booktabs}

\title{%s}%s

\begin{document}
\maketitle
`, t.Escape(title), host)
}

// TOC returns the LaTeX table-of-contents command.
func (t *TeXRenderer) TOC(sections []string) string {
	return "\\tableofcontents\n\\newpage\n"
}

// Section returns a \section or \subsection command followed by content.
func (t *TeXRenderer) Section(title string, level int, content string) string {
	cmd := "\\section"
	if level >= 2 {
		cmd = "\\subsection"
	}
	return fmt.Sprintf("%s{%s}\n%s\n", cmd, t.Escape(title), content)
}

// Footer closes the LaTeX document.
func (t *TeXRenderer) Footer() string {
	return "\\end{document}\n"
}

// Escape escapes the LaTeX special characters _, #, %, &, <, and >.
func (t *TeXRenderer) Escape(raw string) string {
	raw = strings.ReplaceAll(raw, "&", "\\&")
	raw = strings.ReplaceAll(raw, "%", "\\%")
	raw = strings.ReplaceAll(raw, "#", "\\#")
	raw = strings.ReplaceAll(raw, "_", "\\_")
	raw = strings.ReplaceAll(raw, "<", "\\textless{}")
	raw = strings.ReplaceAll(raw, ">", "\\textgreater{}")
	return raw
}

// ─────────────────────────────────────────────────────────────────────────────
// 10. DocBookRenderer – production implementation
// ─────────────────────────────────────────────────────────────────────────────

// DocBookRenderer emits a DocBook 5 XML article.
type DocBookRenderer struct{}

// Header returns the XML declaration and the root <article> element with the
// DocBook 5 namespace.
func (d *DocBookRenderer) Header(manifest *SitarManifest) string {
	title := "SITAR — System InformaTion At Runtime"
	if manifest != nil && manifest.Meta.Hostname != "" {
		title = fmt.Sprintf("SITAR Report: %s", manifest.Meta.Hostname)
	}
	return fmt.Sprintf(`<?xml version="1.0" encoding="UTF-8"?>
<article xmlns="http://docbook.org/ns/docbook" version="5.0"
         xmlns:xlink="http://www.w3.org/1999/xlink">
<info><title>%s</title></info>
`, d.Escape(title))
}

// TOC returns an empty string; DocBook processors generate the TOC
// automatically.
func (d *DocBookRenderer) TOC(sections []string) string {
	return ""
}

// Section wraps title and content in a DocBook <section> element.
func (d *DocBookRenderer) Section(title string, level int, content string) string {
	return fmt.Sprintf("<section>\n<title>%s</title>\n%s\n</section>\n",
		d.Escape(title), content)
}

// Footer closes the root <article> element.
func (d *DocBookRenderer) Footer() string {
	return "</article>\n"
}

// Escape replaces &, <, and > with XML entity references.
func (d *DocBookRenderer) Escape(raw string) string {
	raw = strings.ReplaceAll(raw, "&", "&amp;")
	raw = strings.ReplaceAll(raw, "<", "&lt;")
	raw = strings.ReplaceAll(raw, ">", "&gt;")
	return raw
}

// ─────────────────────────────────────────────────────────────────────────────
// 11. MarkdownRenderer – production implementation
// ─────────────────────────────────────────────────────────────────────────────

// MarkdownRenderer emits GitHub-Flavoured Markdown.
type MarkdownRenderer struct{}

// Header returns the top-level H1 title.
func (m *MarkdownRenderer) Header(manifest *SitarManifest) string {
	return "# SITAR — System InformaTion At Runtime\n\n"
}

// TOC returns an empty string; Markdown renderers typically auto-generate
// navigation.
func (m *MarkdownRenderer) TOC(sections []string) string {
	return ""
}

// Section returns a Markdown heading: ## for level 1, # for level ≥ 2 (or
// vice-versa – level 1 maps to ## to sit below the document H1, deeper
// sections use ###).
func (m *MarkdownRenderer) Section(title string, level int, content string) string {
	prefix := "##"
	if level >= 2 {
		prefix = "###"
	}
	return fmt.Sprintf("%s %s\n\n%s\n", prefix, title, content)
}

// Footer returns an empty string.
func (m *MarkdownRenderer) Footer() string {
	return ""
}

// Escape escapes pipe characters so that they do not break Markdown tables.
func (m *MarkdownRenderer) Escape(raw string) string {
	return strings.ReplaceAll(raw, "|", "\\|")
}

// ─────────────────────────────────────────────────────────────────────────────
// 12. JSONRenderer – minimal production implementation
// ─────────────────────────────────────────────────────────────────────────────

// JSONRenderer provides the structural scaffolding for a JSON report.
// Actual value serialisation is handled by the render-json subsystem using
// encoding/json directly.
type JSONRenderer struct{}

// Header returns the opening brace of the top-level JSON object.
func (j *JSONRenderer) Header(manifest *SitarManifest) string {
	return "{"
}

// TOC returns an empty string; JSON output has no table of contents.
func (j *JSONRenderer) TOC(sections []string) string {
	return ""
}

// Section returns the content as-is; the render-json subsystem is responsible
// for wrapping it in the correct JSON key/value structure.
func (j *JSONRenderer) Section(title string, level int, content string) string {
	return content
}

// Footer returns the closing brace of the top-level JSON object.
func (j *JSONRenderer) Footer() string {
	return "}"
}

// Escape returns raw unchanged; JSON string escaping is performed by
// encoding/json when values are marshalled.
func (j *JSONRenderer) Escape(raw string) string {
	return raw
}

// ─────────────────────────────────────────────────────────────────────────────
// 13. FakeRenderer – test double
// ─────────────────────────────────────────────────────────────────────────────

// FakeRenderer records the section titles it has seen and returns predictable
// values for every method.  It is intended exclusively for unit tests.
type FakeRenderer struct {
	Sections []string
}

// Header returns an empty string.
func (f *FakeRenderer) Header(manifest *SitarManifest) string {
	return ""
}

// TOC returns an empty string.
func (f *FakeRenderer) TOC(sections []string) string {
	return ""
}

// Section appends title to Sections and returns title concatenated with
// content so that tests can assert on the combined output.
func (f *FakeRenderer) Section(title string, level int, content string) string {
	f.Sections = append(f.Sections, title)
	return title + content
}

// Footer returns an empty string.
func (f *FakeRenderer) Footer() string {
	return ""
}

// Escape returns raw unchanged.
func (f *FakeRenderer) Escape(raw string) string {
	return raw
}

// ─────────────────────────────────────────────────────────────────────────────
// 14. PackageBackend interface + supporting types
// ─────────────────────────────────────────────────────────────────────────────

// PackageBackend is the abstraction over package-manager back-ends (RPM,
// dpkg, …).
type PackageBackend interface {
	ListInstalled() ([]PackageRecord, error)
	QueryFile(path string) (string, error)
	VerifyAll() ([]ChangedFileRecord, error)
	VerifyPackage(name string) ([]ChangedFileRecord, error)
}

// ─────────────────────────────────────────────────────────────────────────────
// 15. RPMBackend, DpkgBackend, NullBackend – production stubs (M0)
// ─────────────────────────────────────────────────────────────────────────────

// RPMBackend implements PackageBackend for RPM-based distributions.
// Full implementation is deferred to milestone M5.
type RPMBackend struct {
	Runner CommandRunner
}

// ListInstalled returns nil for M0.
func (b *RPMBackend) ListInstalled() ([]PackageRecord, error) { return nil, nil }

// QueryFile returns an empty string for M0.
func (b *RPMBackend) QueryFile(path string) (string, error) { return "", nil }

// VerifyAll returns nil for M0.
func (b *RPMBackend) VerifyAll() ([]ChangedFileRecord, error) { return nil, nil }

// VerifyPackage returns nil for M0.
func (b *RPMBackend) VerifyPackage(name string) ([]ChangedFileRecord, error) { return nil, nil }

// ─────────────────────────────────────────────────────────────────────────────

// DpkgBackend implements PackageBackend for Debian/Ubuntu distributions.
// Full implementation is deferred to milestone M5.
type DpkgBackend struct {
	Runner CommandRunner
}

// ListInstalled returns nil for M0.
func (b *DpkgBackend) ListInstalled() ([]PackageRecord, error) { return nil, nil }

// QueryFile returns an empty string for M0.
func (b *DpkgBackend) QueryFile(path string) (string, error) { return "", nil }

// VerifyAll returns nil for M0.
func (b *DpkgBackend) VerifyAll() ([]ChangedFileRecord, error) { return nil, nil }

// VerifyPackage returns nil for M0.
func (b *DpkgBackend) VerifyPackage(name string) ([]ChangedFileRecord, error) { return nil, nil }

// ─────────────────────────────────────────────────────────────────────────────

// NullBackend is a no-op PackageBackend used when no supported package manager
// is detected on the target system.
type NullBackend struct{}

// ListInstalled returns nil.
func (b *NullBackend) ListInstalled() ([]PackageRecord, error) { return nil, nil }

// QueryFile returns an empty string.
func (b *NullBackend) QueryFile(path string) (string, error) { return "", nil }

// VerifyAll returns nil.
func (b *NullBackend) VerifyAll() ([]ChangedFileRecord, error) { return nil, nil }

// VerifyPackage returns nil.
func (b *NullBackend) VerifyPackage(name string) ([]ChangedFileRecord, error) { return nil, nil }

// ─────────────────────────────────────────────────────────────────────────────
// 16. FakePackageBackend – test double
// ─────────────────────────────────────────────────────────────────────────────

// FakePackageBackend is a configurable in-memory PackageBackend for use in
// tests.
type FakePackageBackend struct {
	PackageList  []PackageRecord            // returned by ListInstalled
	FileOwnerMap map[string]string          // path -> package name, used by QueryFile
	VerifyResult []ChangedFileRecord        // returned by VerifyAll and VerifyPackage
	VerifyPerPkg map[string][]ChangedFileRecord // per-package override for VerifyPackage
}

// ListInstalled returns the pre-configured PackageList.
func (b *FakePackageBackend) ListInstalled() ([]PackageRecord, error) {
	return b.PackageList, nil
}

// QueryFile looks up path in FileOwnerMap and returns the owning package name,
// or an empty string when the path is not registered.
func (b *FakePackageBackend) QueryFile(path string) (string, error) {
	if b.FileOwnerMap == nil {
		return "", nil
	}
	return b.FileOwnerMap[path], nil
}

// VerifyAll returns the pre-configured VerifyResult slice.
func (b *FakePackageBackend) VerifyAll() ([]ChangedFileRecord, error) {
	return b.VerifyResult, nil
}

// VerifyPackage returns per-package results from VerifyPerPkg when present,
// falling back to VerifyResult.
func (b *FakePackageBackend) VerifyPackage(name string) ([]ChangedFileRecord, error) {
	if b.VerifyPerPkg != nil {
		if records, ok := b.VerifyPerPkg[name]; ok {
			return records, nil
		}
	}
	return b.VerifyResult, nil
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────────────────────

// tocAnchor converts a section title into a URL-safe anchor string by
// lower-casing and replacing spaces and non-alphanumeric runes with hyphens.
func tocAnchor(title string) string {
	title = strings.ToLower(title)
	var sb strings.Builder
	for _, r := range title {
		if (r >= 'a' && r <= 'z') || (r >= '0' && r <= '9') {
			sb.WriteRune(r)
		} else {
			sb.WriteRune('-')
		}
	}
	return strings.Trim(sb.String(), "-")
}
