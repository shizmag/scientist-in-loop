//! `sil paper check` command adapter.

use anyhow::{Result, bail};
use sil_app::{ManuscriptCheckOptions, run_manuscript_check};
use sil_core::{CheckProfile, SilUi};

use crate::util::load_project;

/// Parsed options for `paper check`.
pub struct CheckArgs {
    /// Requested profile.
    pub profile: String,
    /// Use strict policy.
    pub strict: bool,
    /// Request online checks.
    pub online: bool,
    /// Request a build.
    pub build: bool,
    /// Emit JSON.
    pub json: bool,
    /// Include observations.
    pub verbose: bool,
    /// Do not cap findings.
    pub all: bool,
}

/// Run the shared manuscript check and render the requested output.
pub fn run(args: CheckArgs, ui: &dyn SilUi) -> Result<()> {
    let (root, _config, _paths) = load_project()?;
    let profile = if args.strict {
        CheckProfile::Strict
    } else {
        match args.profile.as_str() {
            "draft" => CheckProfile::Draft,
            "submission" => CheckProfile::Submission,
            _ => bail!("profile must be draft or submission"),
        }
    };
    let report = run_manuscript_check(
        &root,
        ManuscriptCheckOptions {
            profile,
            build: args.build,
            online: args.online,
        },
    )?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        ui.println(&report.r#static.format_compact(args.all, args.verbose));
    }
    if !report.passes(&[]) {
        bail!(
            "manuscript check failed for {} profile",
            if args.strict {
                "strict"
            } else {
                match profile {
                    CheckProfile::Draft => "draft",
                    CheckProfile::Submission => "submission",
                    CheckProfile::Strict => "strict",
                }
            }
        );
    }
    Ok(())
}
