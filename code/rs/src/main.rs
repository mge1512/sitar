// main.rs — sitar entry point
// System InformaTion At Runtime
// License: GPL-2.0-or-later
// Author: Matthias G. Eckermann <pcd@mailbox.org>
#![allow(dead_code)]
#![allow(unused_imports)]

mod types;
mod interfaces;
mod detect;
mod collect;
mod collect_hw;
mod collect_storage;
mod collect_network;
mod collect_pkg;
mod collect_config;
mod render_human;
mod render_json;
mod render;

use interfaces::{OSFilesystem, OSCommandRunner};
use types::{Config, OutputFormat, Verbosity};

const SITAR_VERSION: &str = "0.9.0";

fn main() {
    // Parse config (includes sysconfig file + argv)
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let config = prepare_config("/etc/sysconfig/sitar", &argv);

    // Detect distribution
    let fs = OSFilesystem;
    let cr = OSCommandRunner;
    let dist = detect::detect_distribution(&fs);

    // Collect
    let manifest = collect::collect(&config, &dist, &fs, &cr);

    // Render
    let files_written = render::render(&manifest, &config);

    if files_written.is_empty() && config.format.is_some() {
        eprintln!("sitar: no output files written");
        std::process::exit(1);
    }

    std::process::exit(0);
}

/// prepare-config BEHAVIOR
fn prepare_config(config_file: &str, argv: &[String]) -> Config {
    let mut config = Config::default();

    // Step 1: parse sysconfig file
    if let Ok(content) = std::fs::read_to_string(config_file) {
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('#') || line.is_empty() { continue; }
            if let Some(pos) = line.find('=') {
                let key = line[..pos].trim();
                let val = line[pos+1..].trim().trim_matches('"').to_string();
                match key {
                    "SITAR_OPT_FORMAT"         => {
                        config.format = OutputFormat::from_str(&val);
                    }
                    "SITAR_OPT_OUTDIR"         => config.outdir  = val,
                    "SITAR_OPT_OUTFILE"        => config.outfile = val,
                    "SITAR_OPT_LIMIT"          => {
                        config.file_size_limit = val.parse().unwrap_or(700_000);
                    }
                    "SITAR_OPT_GCONF"          => {
                        config.gconf = val.to_lowercase() == "true" || val == "1";
                    }
                    "SITAR_OPT_ALLCONFIGFILES" => config.allconfigfiles = val,
                    "SITAR_OPT_ALLSUBDOMAIN"   => config.allsubdomain   = val,
                    "SITAR_OPT_ALLSYSCONFIG"   => config.allsysconfig   = val,
                    "SITAR_OPT_EXCLUDE"        => {
                        config.exclude = val.split(':')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                        if !config.exclude.contains(&"/etc/shadow".to_string()) {
                            config.exclude.push("/etc/shadow".to_string());
                        }
                    }
                    "SITAR_OPT_LVMARCHIVE"     => {
                        config.lvmarchive = val.to_lowercase() == "true" || val == "1";
                    }
                    _ => {} // ignore silently
                }
            }
        }
    }

    // Step 2: parse argv
    if argv.is_empty() {
        print_help();
        std::process::exit(0);
    }

    for token in argv {
        if let Some(pos) = token.find('=') {
            // key=value option
            let key = &token[..pos];
            let val = &token[pos+1..];
            match key {
                "format" => {
                    match OutputFormat::from_str(val) {
                        Some(f) => config.format = Some(f),
                        None => {
                            eprintln!("sitar: unrecognised format value: {}", val);
                            std::process::exit(2);
                        }
                    }
                }
                "outfile" => config.outfile = val.to_string(),
                "outdir"  => config.outdir  = val.to_string(),
                "limit"   => {
                    match val.parse::<u64>() {
                        Ok(n) => config.file_size_limit = n,
                        Err(_) => {
                            eprintln!("sitar: invalid limit value: {}", val);
                            std::process::exit(2);
                        }
                    }
                }
                _ => {
                    eprintln!("sitar: unrecognised option: {}", token);
                    std::process::exit(2);
                }
            }
        } else {
            // bare-word command
            match token.as_str() {
                "all" => {
                    config.all               = true;
                    config.format            = None; // all formats
                    config.check_consistency = true;
                    config.find_unpacked     = true;
                }
                "check-consistency" => {
                    config.check_consistency = true;
                    // Only run check-consistency, not full collection
                    run_check_consistency_only();
                }
                "find-unpacked" => {
                    config.find_unpacked = true;
                    // Only run find-unpacked, not full collection
                    run_find_unpacked_only();
                }
                "help" => {
                    print_help();
                    std::process::exit(0);
                }
                "version" => {
                    println!("sitar {}", SITAR_VERSION);
                    std::process::exit(0);
                }
                "debug" => {
                    config.verbosity = Verbosity::Debug;
                    config.debug     = true;
                }
                _ => {
                    eprintln!("sitar: unrecognised argument: {}", token);
                    std::process::exit(2);
                }
            }
        }
    }

    // Step 3: validate format
    // (already validated above in the match arm)

    // Step 4: if all=true, set check_consistency and find_unpacked
    if config.all {
        config.check_consistency = true;
        config.find_unpacked     = true;
    }

    // Step 5: normalise format (already done: None == all)

    config
}

fn run_check_consistency_only() {
    // Verify root
    let uid = libc_geteuid();
    if uid != 0 {
        eprintln!("Please run sitar as user root.");
        std::process::exit(1);
    }
    let cr = OSCommandRunner;
    match collect_config::check_consistency(&cr, "/var/lib/support", "Configuration_Consistency.json") {
        Ok(path) => {
            println!("sitar: check-consistency: cache written to {}", path);
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("sitar: check-consistency failed: {}", e);
            std::process::exit(1);
        }
    }
}

fn run_find_unpacked_only() {
    let uid = libc_geteuid();
    if uid != 0 {
        eprintln!("Please run sitar as user root.");
        std::process::exit(1);
    }
    let cr = OSCommandRunner;
    match collect_config::find_unpacked(&cr, "/var/lib/support", "Find_Unpacked.json", "/etc") {
        Ok(path) => {
            println!("sitar: find-unpacked: cache written to {}", path);
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("sitar: find-unpacked failed: {}", e);
            std::process::exit(1);
        }
    }
}

fn print_help() {
    println!(
        "Usage: sitar [COMMAND] [OPTIONS]

System InformaTion At Runtime — collects hardware, kernel, network,
storage, security, and package information from a running Linux system.

Commands (bare-word):
  all                   Produce all output formats; run consistency + find-unpacked cache
  check-consistency     Pre-run cache generation: RPM config file consistency check
  find-unpacked         Pre-run cache generation: find files not owned by any RPM
  help                  Print this help text and exit
  version               Print version string and exit
  debug                 Enable debug output to stderr

Options (key=value):
  format=<fmt>          Output format: html | tex | sdocbook | json | markdown | all
  outfile=<path>        Output file path (single format only)
  outdir=<path>         Output directory (default: /tmp/sitar-<hostname>-<datetime>)
  limit=<n>             Maximum config file size in bytes (default: 700000; 0=unlimited)

Exit codes:
  0    Success
  1    Logical error or fatal error
  2    Invocation error (bad arguments, unknown format)

Examples:
  sitar format=json outfile=/tmp/sitar.json
  sitar format=html outdir=/var/tmp/sitar-out
  sitar all outdir=/tmp/myreport
  sitar check-consistency
  sitar find-unpacked

Configuration file: /etc/sysconfig/sitar
  SITAR_OPT_FORMAT, SITAR_OPT_OUTDIR, SITAR_OPT_OUTFILE,
  SITAR_OPT_LIMIT, SITAR_OPT_GCONF, SITAR_OPT_ALLCONFIGFILES,
  SITAR_OPT_ALLSUBDOMAIN, SITAR_OPT_ALLSYSCONFIG,
  SITAR_OPT_EXCLUDE, SITAR_OPT_LVMARCHIVE

Version: sitar {}
",
        SITAR_VERSION
    );
}

fn libc_geteuid() -> u32 {
    unsafe {
        extern "C" { fn geteuid() -> u32; }
        geteuid()
    }
}
