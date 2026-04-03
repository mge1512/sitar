package main

import (
	"bufio"
	"fmt"
	"os"
	"os/signal"
	"strconv"
	"strings"
	"syscall"
	"time"
)

const (
	version    = "0.9.0"
	binaryName = "sitar"
)

func parseBool(s string) bool {
	s = strings.ToLower(strings.TrimSpace(s))
	return s == "yes" || s == "true" || s == "1" || s == "on"
}

func debugLog(format string, args ...interface{}) {
	if os.Getenv("SITAR_DEBUG") == "1" {
		fmt.Fprintf(os.Stderr, "[debug] "+format+"\n", args...)
	}
}

func printHelp() {
	fmt.Printf(`%s %s - System Information Tool and Reporter

Usage:
  %s [COMMAND] [OPTION=VALUE ...]

Commands:
  all                  Collect all information, run check-consistency and find-unpacked
  check-consistency    Check system configuration consistency and exit
  find-unpacked        Find installed files not belonging to any package and exit
  help                 Print this help text and exit
  version              Print version and exit

Options (key=value):
  format=FORMAT        Output format: html, tex, sdocbook, json, markdown, all
                       (default: html)
  outfile=PATH         Write output to PATH instead of the default location
  outdir=PATH          Write output files into directory PATH
  limit=N              Skip files larger than N bytes (default: 700000)
  debug                Enable debug verbosity

Sysconfig:
  Options can also be set in /etc/sysconfig/sitar using KEY=VALUE pairs:
    SITAR_OPT_FORMAT         Output format
    SITAR_OPT_OUTDIR         Output directory
    SITAR_OPT_OUTFILE        Output file path
    SITAR_OPT_LIMIT          File size limit (bytes)
    SITAR_OPT_GCONF          Include GConf data (yes/true/1)
    SITAR_OPT_ALLCONFIGFILES Collect all config files (Auto/yes/no)
    SITAR_OPT_ALLSUBDOMAIN   Collect all subdomain info (Auto/yes/no)
    SITAR_OPT_ALLSYSCONFIG   Collect all sysconfig files (Auto/yes/no)
    SITAR_OPT_EXCLUDE        Comma-separated list of files to exclude
    SITAR_OPT_LVMARCHIVE     LVM archive path

`, binaryName, version, binaryName)
}

func parseSysconfigBool(value string) bool {
	v := strings.ToLower(strings.TrimSpace(value))
	return v == "yes" || v == "true" || v == "1"
}

func loadSysconfig(config *Config) {
	const sysconfigPath = "/etc/sysconfig/sitar"

	f, err := os.Open(sysconfigPath)
	if err != nil {
		if os.IsNotExist(err) {
			debugLog("sysconfig file %s not found, skipping", sysconfigPath)
			return
		}
		debugLog("could not open sysconfig file %s: %v", sysconfigPath, err)
		return
	}
	defer f.Close()

	debugLog("loading sysconfig from %s", sysconfigPath)

	scanner := bufio.NewScanner(f)
	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())

		// Skip empty lines and comments
		if line == "" || strings.HasPrefix(line, "#") {
			continue
		}

		idx := strings.IndexByte(line, '=')
		if idx < 0 {
			continue
		}

		key := strings.TrimSpace(line[:idx])
		val := strings.TrimSpace(line[idx+1:])

		// Strip surrounding quotes (single or double)
		if len(val) >= 2 {
			if (val[0] == '"' && val[len(val)-1] == '"') ||
				(val[0] == '\'' && val[len(val)-1] == '\'') {
				val = val[1 : len(val)-1]
			}
		}

		debugLog("sysconfig key=%q val=%q", key, val)

		switch key {
		case "SITAR_OPT_FORMAT":
			config.Format = val
		case "SITAR_OPT_OUTDIR":
			config.Outdir = val
		case "SITAR_OPT_OUTFILE":
			config.Outfile = val
		case "SITAR_OPT_LIMIT":
			n, err := strconv.Atoi(val)
			if err == nil {
				config.FileSizeLimit = n
			} else {
				debugLog("could not parse SITAR_OPT_LIMIT=%q as int: %v", val, err)
			}
		case "SITAR_OPT_GCONF":
			config.GConf = parseSysconfigBool(val)
		case "SITAR_OPT_ALLCONFIGFILES":
			config.AllConfigFiles = val
		case "SITAR_OPT_ALLSUBDOMAIN":
			config.AllSubdomain = val
		case "SITAR_OPT_ALLSYSCONFIG":
			config.AllSysconfig = val
		case "SITAR_OPT_EXCLUDE":
			if val != "" {
				parts := strings.Split(val, ",")
				for i, p := range parts {
					parts[i] = strings.TrimSpace(p)
				}
				config.Exclude = parts
			}
		case "SITAR_OPT_LVMARCHIVE":
			config.LvmArchive = parseBool(val)
		}
	}

	if err := scanner.Err(); err != nil {
		debugLog("error reading sysconfig file %s: %v", sysconfigPath, err)
	}
}

func prepareConfig(argv []string) *Config {
	// Apply default values
	config := &Config{
		FileSizeLimit:  700000,
		Exclude:        []string{"/etc/shadow"},
		AllConfigFiles: "Auto",
		AllSubdomain:   "Auto",
		AllSysconfig:   "Auto",
		Verbosity:      "normal",
	}

	// Load /etc/sysconfig/sitar if it exists (overrides compiled-in defaults)
	loadSysconfig(config)

	// Empty argv: print help and exit 0
	if len(argv) == 0 {
		printHelp()
		os.Exit(0)
	}

	// Parse argv tokens
	for _, token := range argv {
		switch {
		case token == "all":
			config.All = true
			config.Format = ""

		case token == "check-consistency":
			config.CheckConsistency = true

		case token == "find-unpacked":
			config.FindUnpacked = true

		case token == "help":
			printHelp()
			os.Exit(0)

		case token == "version":
			fmt.Printf("%s %s\n", binaryName, version)
			os.Exit(0)

		case token == "debug":
			config.Verbosity = "debug"

		case strings.HasPrefix(token, "format="):
			config.Format = strings.TrimPrefix(token, "format=")

		case strings.HasPrefix(token, "outfile="):
			config.Outfile = strings.TrimPrefix(token, "outfile=")

		case strings.HasPrefix(token, "outdir="):
			config.Outdir = strings.TrimPrefix(token, "outdir=")

		case strings.HasPrefix(token, "limit="):
			raw := strings.TrimPrefix(token, "limit=")
			n, err := strconv.Atoi(raw)
			if err != nil {
				fmt.Fprintf(os.Stderr, "%s: invalid value for limit=%q: %v\n", binaryName, raw, err)
				os.Exit(2)
			}
			config.FileSizeLimit = n

		default:
			fmt.Fprintf(os.Stderr, "%s: unknown argument %q\n", binaryName, token)
			os.Exit(2)
		}
	}

	// Validate format
	validFormats := map[string]bool{
		"html":     true,
		"tex":      true,
		"sdocbook": true,
		"json":     true,
		"markdown": true,
		"all":      true,
		"":         true,
	}
	if !validFormats[config.Format] {
		fmt.Fprintf(os.Stderr, "%s: invalid format %q: must be one of html, tex, sdocbook, json, markdown, all\n",
			binaryName, config.Format)
		os.Exit(2)
	}

	// Normalize "all" format to ""
	if config.Format == "all" {
		config.Format = ""
	}

	// "all" command implies check-consistency and find-unpacked
	if config.All {
		config.CheckConsistency = true
		config.FindUnpacked = true
	}

	debugLog("config after prepareConfig: %+v", config)

	return config
}

func main() {
	// Set up signal handling for clean exit on SIGTERM and SIGINT
	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGTERM, syscall.SIGINT)
	go func() {
		sig := <-sigCh
		debugLog("received signal %v, exiting", sig)
		os.Exit(0)
	}()

	config := prepareConfig(os.Args[1:])

	// Check-only modes: run the requested checks and exit
	if config.CheckConsistency || config.FindUnpacked {
		if !config.All {
			// Pure check/find run — no full collection needed.
			// Use a long timeout: rpm -V per package can take many minutes.
			cr := &OSCommandRunner{Timeout: 15 * time.Minute}
			if config.CheckConsistency {
				debugLog("running check-consistency")
				if _, err := checkConsistency(cr); err != nil {
					fmt.Fprintf(os.Stderr, "%s: check-consistency: %v\n", binaryName, err)
					os.Exit(1)
				}
			}
			if config.FindUnpacked {
				debugLog("running find-unpacked")
				if _, err := findUnpacked(cr); err != nil {
					fmt.Fprintf(os.Stderr, "%s: find-unpacked: %v\n", binaryName, err)
					os.Exit(1)
				}
			}
			os.Exit(0)
		}
	}

	// Full collection + render run
	fs := &OSFilesystem{}
	cr := &OSCommandRunner{}

	debugLog("starting collect")
	manifest := collect(config, fs, cr)

	debugLog("starting render")
	if err := renderManifest(manifest, config); err != nil {
		fmt.Fprintf(os.Stderr, "%s: render error: %v\n", binaryName, err)
		os.Exit(1)
	}
}
