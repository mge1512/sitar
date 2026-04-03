package main

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"time"
)

// renderManifest dispatches to format-specific renderers.
func renderManifest(manifest *SitarManifest, config *Config) error {
	// Determine active formats
	var activeFormats []string
	if config.Format == "" || config.Format == "all" {
		activeFormats = []string{"html", "tex", "sdocbook", "json", "markdown"}
	} else {
		activeFormats = []string{config.Format}
	}

	// Determine outdir
	outdir := config.Outdir
	if outdir == "" && len(activeFormats) > 1 {
		hostname := ""
		if manifest.Meta.Hostname != "" {
			hostname = manifest.Meta.Hostname
		}
		ts := time.Now().Format("2006010215")
		outdir = fmt.Sprintf("/tmp/sitar-%s-%s", hostname, ts)
	} else if outdir == "" {
		outdir = "."
	}

	if err := os.MkdirAll(outdir, 0755); err != nil {
		fmt.Fprintf(os.Stderr, "sitar: cannot create output directory %s: %v\n", outdir, err)
		os.Exit(1)
	}

	hostname := manifest.Meta.Hostname
	if hostname == "" {
		hostname = "unknown"
	}

	var lastErr error
	for _, format := range activeFormats {
		var outpath string
		if config.Outfile != "" && len(activeFormats) == 1 {
			outpath = config.Outfile
			if !filepath.IsAbs(outpath) && !strings.Contains(outpath, "/") && outdir != "." {
				outpath = filepath.Join(outdir, outpath)
			}
		} else {
			ext := formatExt(format)
			outpath = filepath.Join(outdir, "sitar-"+hostname+ext)
		}

		if err := os.MkdirAll(filepath.Dir(outpath), 0755); err != nil {
			fmt.Fprintf(os.Stderr, "sitar: cannot create directory for %s: %v\n", outpath, err)
			lastErr = err
			continue
		}

		fmt.Fprintf(os.Stderr, "Generating %s...\n", outpath)

		if format == "json" {
			if err := renderJSON(manifest, outpath); err != nil {
				fmt.Fprintf(os.Stderr, "sitar: render json: %v\n", err)
				lastErr = err
			}
		} else {
			if err := renderHuman(manifest, format, outpath); err != nil {
				fmt.Fprintf(os.Stderr, "sitar: render %s: %v\n", format, err)
				lastErr = err
			}
		}
	}
	return lastErr
}

func formatExt(format string) string {
	switch format {
	case "html":
		return ".html"
	case "tex":
		return ".tex"
	case "sdocbook":
		return ".sdocbook.xml"
	case "json":
		return ".json"
	case "markdown":
		return ".md"
	default:
		return "." + format
	}
}

// renderJSON serialises the SitarManifest to a JSON file.
func renderJSON(manifest *SitarManifest, outpath string) error {
	f, err := os.Create(outpath)
	if err != nil {
		return fmt.Errorf("create %s: %w", outpath, err)
	}
	defer f.Close()

	enc := json.NewEncoder(f)
	enc.SetIndent("", "  ")
	if err := enc.Encode(manifest); err != nil {
		return fmt.Errorf("encode JSON: %w", err)
	}
	return nil
}
