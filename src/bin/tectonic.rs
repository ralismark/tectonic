// src/bin/tectonic.rs -- Command-line driver for the Tectonic engine.
// Copyright 2016-2018 the Tectonic Project
// Licensed under the MIT License.

use structopt::StructOpt;

use std::env;
use std::path::{Path, PathBuf};
use std::process;
use std::str::FromStr;
use std::time;

use tectonic::config::PersistentConfig;
use tectonic::driver::{OutputFormat, PassSetting, ProcessingSessionBuilder};
use tectonic::errors::{ErrorKind, Result};
use tectonic::status::plain::PlainStatusBackend;
use tectonic::status::termcolor::TermcolorStatusBackend;
use tectonic::status::{ChatterLevel, MessageKind, StatusBackend};
use tectonic::unstable_opts::{UnstableArg, UnstableOptions};
use tectonic::{errmsg, tt_error, tt_note};

#[derive(Debug, StructOpt)]
#[structopt(name = "Tectonic", about = "Process a (La)TeX document")]
struct CliOptions {
    /// The file to process, or "-" to process the standard input stream
    #[structopt(name = "input")]
    input: Option<String>,
    /// The name of the "format" file used to initialize the TeX engine
    #[structopt(long, short, name = "path", default_value = "latex")]
    format: String,
    /// Use this directory or Zip-format bundle file to find resource files instead of the default
    #[structopt(takes_value(true), parse(from_os_str), long, short, name = "file_path")]
    bundle: Option<PathBuf>,
    /// Use this URL to find resource files instead of the default
    #[structopt(takes_value(true), long, short, name = "url")]
    // TODO add URL validation
    web_bundle: Option<String>,
    /// How much chatter to print when running
    #[structopt(long = "chatter", short, name = "level", default_value = "default", possible_values(&["default", "minimal"]))]
    chatter_level: String,
    /// Enable/disable colorful log output.
    #[structopt(long = "color", name = "when", default_value = "auto", possible_values(&["always", "auto", "never"]))]
    cli_color: String,
    /// Use only resource files cached locally
    #[structopt(short = "C", long)]
    only_cached: bool,
    /// The kind of output to generate
    #[structopt(long, name = "format", default_value = "pdf", possible_values(&["pdf", "html", "xdv", "aux", "fmt"]))]
    outfmt: String,
    /// Write Makefile-format rules expressing the dependencies of this run to <dest_path>
    #[structopt(long, name = "dest_path")]
    makefile_rules: Option<PathBuf>,
    /// Which engines to run
    #[structopt(long, default_value = "default", possible_values(&["default", "tex", "bibtex_first"]))]
    pass: String,
    /// Rerun the TeX engine exactly this many times after the first
    #[structopt(name = "count", long = "reruns", short = "r")]
    reruns: Option<usize>,
    /// Keep the intermediate files generated during processing
    #[structopt(short, long)]
    keep_intermediates: bool,
    /// Keep the log files generated during processing
    #[structopt(long)]
    keep_logs: bool,
    /// Generate SyncTeX data
    #[structopt(long)]
    synctex: bool,
    /// Tell the engine that no file at <hide_path> exists, if it tries to read it
    #[structopt(long, name = "hide_path")]
    hide: Option<Vec<PathBuf>>,
    /// Print the engine's chatter during processing
    #[structopt(long = "print", short)]
    print_stdout: bool,
    /// The directory in which to place output files [default: the directory containing <input>]
    #[structopt(name = "outdir", short, long, parse(from_os_str))]
    outdir: Option<PathBuf>,
    /// Unstable options. Pass -Zhelp to show a list
    // TODO we can't pass -Zhelp without also passing <input>
    #[structopt(name = "option", short = "Z", number_of_values = 1)]
    unstable: Vec<UnstableArg>,
}
fn inner(args: CliOptions, config: PersistentConfig, status: &mut dyn StatusBackend) -> Result<()> {
    let unstable = UnstableOptions::from_unstable_args(args.unstable.into_iter());

    let mut sess_builder = ProcessingSessionBuilder::default();
    let format_path = args.format;
    sess_builder
        .unstables(unstable)
        .format_name(&format_path)
        .keep_logs(args.keep_logs)
        .keep_intermediates(args.keep_intermediates)
        .format_cache_path(config.format_cache_path()?)
        .synctex(args.synctex);

    sess_builder.output_format(OutputFormat::from_str(&args.outfmt).unwrap());

    let pass = PassSetting::from_str(&args.pass).unwrap();
    sess_builder.pass(pass);

    if let Some(s) = args.reruns {
        sess_builder.reruns(s);
    }

    if let Some(p) = args.makefile_rules {
        sess_builder.makefile_output_path(p);
    }

    // Input and path setup

    let input_path = args
        .input
        .ok_or("No input specified.\n\nFor more information try --help")?;
    if input_path == "-" {
        // Don't provide an input path to the ProcessingSession, so it will default to stdin.
        sess_builder.tex_input_name("texput.tex");
        sess_builder.output_dir(Path::new(""));
        tt_note!(
            status,
            "reading from standard input; outputs will appear under the base name \"texput\""
        );
    } else {
        let input_path = Path::new(&input_path);
        sess_builder.primary_input_path(input_path);

        if let Some(fname) = input_path.file_name() {
            sess_builder.tex_input_name(&fname.to_string_lossy());
        } else {
            return Err(errmsg!(
                "can't figure out a basename for input path \"{}\"",
                input_path.to_string_lossy()
            ));
        };

        if let Some(par) = input_path.parent() {
            sess_builder.output_dir(par);
        } else {
            return Err(errmsg!(
                "can't figure out a parent directory for input path \"{}\"",
                input_path.to_string_lossy()
            ));
        }
    }

    if let Some(output_dir) = args.outdir {
        if !output_dir.is_dir() {
            return Err(errmsg!(
                "output directory \"{}\" does not exist",
                output_dir.display()
            ));
        }
        sess_builder.output_dir(output_dir);
    }

    // Set up the rest of I/O.

    sess_builder.print_stdout(args.print_stdout);

    if let Some(items) = args.hide {
        for v in items {
            sess_builder.hide(v);
        }
    }

    let only_cached = args.only_cached;
    if only_cached {
        tt_note!(status, "using only cached resource files");
    }
    if let Some(path) = args.bundle {
        sess_builder.bundle(config.make_local_file_provider(path, status)?);
    } else if let Some(u) = args.web_bundle {
        sess_builder.bundle(config.make_cached_url_provider(&u, only_cached, None, status)?);
    } else {
        sess_builder.bundle(config.default_bundle(only_cached, status)?);
    }

    let build_date_str = env::var("SOURCE_DATE_EPOCH").ok();
    let build_date = match build_date_str {
        Some(s) => {
            let epoch = u64::from_str_radix(&s, 10).expect("invalid build date (not a number)");
            time::SystemTime::UNIX_EPOCH
                .checked_add(time::Duration::from_secs(epoch))
                .expect("time overflow")
        }
        None => time::SystemTime::now(),
    };
    sess_builder.build_date(build_date);

    let mut sess = sess_builder.create(status)?;
    let result = sess.run(status);

    if let Err(e) = &result {
        if let ErrorKind::EngineError(engine) = e.kind() {
            if let Some(output) = sess.io.mem.files.borrow().get(sess.io.mem.stdout_key()) {
                tt_error!(
                    status,
                    "something bad happened inside {}; its output follows:\n",
                    engine
                );

                status.dump_error_logs(&output.data);
            }
        }
    }
    result
}

fn main() {
    let args = CliOptions::from_args();

    // The Tectonic crate comes with a hidden internal "test mode" that forces
    // it to use a specified set of local files, rather than going to the
    // bundle -- this makes it so that we can run tests without having to go
    // to the network or touch the current user's cache.
    //
    // This mode is activated by setting a special environment variable. The
    // following call checks for it and activates the mode if necessary. Note
    // that this test infrastructure is lightweight, so I don't think it's a
    // big deal to include the code in the final executable artifacts we
    // distribute.

    tectonic::test_util::maybe_activate_test_mode();

    // I want the CLI program to take as little configuration as possible, but
    // we do need to at least provide a mechanism for storing the default
    // bundle.

    let config = match PersistentConfig::open(false) {
        Ok(c) => c,
        Err(ref e) => {
            // Uhoh, we couldn't get the configuration. Our main
            // error-printing code requires a 'status' object, which we don't
            // have yet. If we can't even load the config we might really be
            // in trouble, so it seems safest to keep things simple anyway and
            // just use bare stderr without colorization.
            e.dump_uncolorized();
            process::exit(1);
        }
    };

    // Set up colorized output. This comes after the config because you could
    // imagine wanting to be able to configure the colorization (which is
    // something I'd be relatively OK with since it'd only affect the progam
    // UI, not the processing results).

    let chatter_level = ChatterLevel::from_str(&args.chatter_level).unwrap();
    let use_cli_color = match &*args.cli_color {
        "always" => true,
        "auto" => atty::is(atty::Stream::Stdout),
        "never" => false,
        _ => unreachable!(),
    };

    let mut status = if use_cli_color {
        Box::new(TermcolorStatusBackend::new(chatter_level)) as Box<dyn StatusBackend>
    } else {
        Box::new(PlainStatusBackend::new(chatter_level)) as Box<dyn StatusBackend>
    };

    // For now ...

    tt_note!(
        status,
        "this is a BETA release; ask questions and report bugs at https://tectonic.newton.cx/"
    );

    // Now that we've got colorized output, pass off to the inner function ...
    // all so that we can print out the word "error:" in red. This code
    // parallels various bits of the `error_chain` crate.

    if let Err(ref e) = inner(args, config, &mut *status) {
        status.report_error(e);
        process::exit(1)
    }
}
