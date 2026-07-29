//! Terminal UX abstraction: colors + progress, testable silent mode.

use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Handle to an indeterminate spinner (may be a no-op in tests).
pub trait SpinnerHandle: Send {
    /// Update spinner message.
    fn set_message(&mut self, msg: &str);
    /// Finish spinner with a final message (success path).
    fn finish_success(&mut self, msg: &str);
    /// Finish spinner indicating failure.
    fn finish_error(&mut self, msg: &str);
    /// Clear/abandon without a message.
    fn abandon(&mut self);
}

/// Handle to a determinate progress bar.
pub trait ProgressHandle: Send {
    /// Set position (0-based completed count).
    fn set_position(&mut self, pos: u64);
    /// Increment by one.
    fn inc(&mut self, delta: u64);
    /// Update message.
    fn set_message(&mut self, msg: &str);
    /// Finish successfully.
    fn finish_success(&mut self, msg: &str);
    /// Finish with error summary.
    fn finish_error(&mut self, msg: &str);
}

/// Terminal UI trait used by all long-running / user-facing operations.
pub trait SilUi: Send + Sync {
    /// Whether colors are enabled.
    fn colors_enabled(&self) -> bool;
    /// Whether interactive prompts are allowed.
    fn interactive(&self) -> bool;

    /// Print a success line (green).
    fn success(&self, msg: &str);
    /// Print a warning line (yellow).
    fn warn(&self, msg: &str);
    /// Print an error line (red).
    fn error(&self, msg: &str);
    /// Print an info line (cyan/blue).
    fn info(&self, msg: &str);
    /// Print muted secondary text.
    fn muted(&self, msg: &str);
    /// Print plain text without styling.
    fn println(&self, msg: &str);
    /// Print without newline.
    fn print(&self, msg: &str);

    /// Start an indeterminate spinner.
    fn spinner(&self, msg: &str) -> Box<dyn SpinnerHandle>;
    /// Start a progress bar for `total` items.
    fn progress(&self, total: u64, msg: &str) -> Box<dyn ProgressHandle>;
}

// ── Null (test) implementation ──────────────────────────────────────────────

struct NullSpinner;
impl SpinnerHandle for NullSpinner {
    fn set_message(&mut self, _msg: &str) {}
    fn finish_success(&mut self, _msg: &str) {}
    fn finish_error(&mut self, _msg: &str) {}
    fn abandon(&mut self) {}
}

struct NullProgress;
impl ProgressHandle for NullProgress {
    fn set_position(&mut self, _pos: u64) {}
    fn inc(&mut self, _delta: u64) {}
    fn set_message(&mut self, _msg: &str) {}
    fn finish_success(&mut self, _msg: &str) {}
    fn finish_error(&mut self, _msg: &str) {}
}

/// Silent UI for tests: no colors, no progress, captures optional output.
#[derive(Debug, Default)]
pub struct NullUi {
    /// Captured lines when recording is enabled.
    lines: Mutex<Vec<String>>,
    record: bool,
}

impl NullUi {
    /// Create a silent UI that discards all output.
    pub fn new() -> Self {
        Self {
            lines: Mutex::new(Vec::new()),
            record: false,
        }
    }

    /// Create a silent UI that records printed lines for assertions.
    pub fn recording() -> Self {
        Self {
            lines: Mutex::new(Vec::new()),
            record: true,
        }
    }

    /// Snapshot recorded lines.
    pub fn lines(&self) -> Vec<String> {
        self.lines.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn push(&self, kind: &str, msg: &str) {
        if self.record
            && let Ok(mut g) = self.lines.lock()
        {
            g.push(format!("{kind}:{msg}"));
        }
    }
}

impl SilUi for NullUi {
    fn colors_enabled(&self) -> bool {
        false
    }
    fn interactive(&self) -> bool {
        false
    }
    fn success(&self, msg: &str) {
        self.push("success", msg);
    }
    fn warn(&self, msg: &str) {
        self.push("warn", msg);
    }
    fn error(&self, msg: &str) {
        self.push("error", msg);
    }
    fn info(&self, msg: &str) {
        self.push("info", msg);
    }
    fn muted(&self, msg: &str) {
        self.push("muted", msg);
    }
    fn println(&self, msg: &str) {
        self.push("print", msg);
    }
    fn print(&self, msg: &str) {
        self.push("print", msg);
    }
    fn spinner(&self, _msg: &str) -> Box<dyn SpinnerHandle> {
        Box::new(NullSpinner)
    }
    fn progress(&self, _total: u64, _msg: &str) -> Box<dyn ProgressHandle> {
        Box::new(NullProgress)
    }
}

// ── Std (human) implementation ──────────────────────────────────────────────

/// Human-facing terminal UI with colors and indicatif progress.
pub struct StdUi {
    colors: bool,
    interactive: bool,
    out: Arc<Mutex<io::Stdout>>,
}

impl StdUi {
    /// Create UI auto-detecting TTY for color and interactivity.
    pub fn new() -> Self {
        let is_tty = console::Term::stdout().is_term();
        let no_color = std::env::var_os("NO_COLOR").is_some()
            || std::env::var("SIL_NO_COLOR").map(|v| v == "1").unwrap_or(false);
        Self {
            colors: is_tty && !no_color,
            interactive: is_tty
                && !std::env::var("SIL_NONINTERACTIVE")
                    .map(|v| v == "1")
                    .unwrap_or(false),
            out: Arc::new(Mutex::new(io::stdout())),
        }
    }

    /// Force non-interactive, colorless mode (CI-friendly).
    pub fn plain() -> Self {
        Self {
            colors: false,
            interactive: false,
            out: Arc::new(Mutex::new(io::stdout())),
        }
    }

    fn write_line(&self, styled: String, plain: &str) {
        let mut out = self.out.lock().unwrap_or_else(|e| e.into_inner());
        let line = if self.colors { styled } else { plain.to_string() };
        let _ = writeln!(out, "{line}");
        let _ = out.flush();
    }
}

impl Default for StdUi {
    fn default() -> Self {
        Self::new()
    }
}

impl SilUi for StdUi {
    fn colors_enabled(&self) -> bool {
        self.colors
    }
    fn interactive(&self) -> bool {
        self.interactive
    }

    fn success(&self, msg: &str) {
        use owo_colors::OwoColorize;
        self.write_line(format!("{} {msg}", "✔".green().bold()), &format!("✔ {msg}"));
    }
    fn warn(&self, msg: &str) {
        use owo_colors::OwoColorize;
        self.write_line(
            format!("{} {msg}", "⚠".yellow().bold()),
            &format!("⚠ {msg}"),
        );
    }
    fn error(&self, msg: &str) {
        use owo_colors::OwoColorize;
        self.write_line(format!("{} {msg}", "✖".red().bold()), &format!("✖ {msg}"));
    }
    fn info(&self, msg: &str) {
        use owo_colors::OwoColorize;
        self.write_line(
            format!("{} {msg}", "ℹ".cyan().bold()),
            &format!("ℹ {msg}"),
        );
    }
    fn muted(&self, msg: &str) {
        use owo_colors::OwoColorize;
        self.write_line(format!("{}", msg.dimmed()), msg);
    }
    fn println(&self, msg: &str) {
        self.write_line(msg.to_string(), msg);
    }
    fn print(&self, msg: &str) {
        let mut out = self.out.lock().unwrap_or_else(|e| e.into_inner());
        let _ = write!(out, "{msg}");
        let _ = out.flush();
    }

    fn spinner(&self, msg: &str) -> Box<dyn SpinnerHandle> {
        if !self.interactive {
            // Non-interactive: emit a single info line and use null spinner.
            self.info(msg);
            return Box::new(NullSpinner);
        }
        let pb = indicatif::ProgressBar::new_spinner();
        pb.set_style(
            indicatif::ProgressStyle::with_template("{spinner:.cyan} {msg}")
                .unwrap_or_else(|_| indicatif::ProgressStyle::default_spinner())
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
        );
        pb.set_message(msg.to_string());
        pb.enable_steady_tick(Duration::from_millis(80));
        Box::new(IndicatifSpinner { pb })
    }

    fn progress(&self, total: u64, msg: &str) -> Box<dyn ProgressHandle> {
        if !self.interactive {
            self.info(&format!("{msg} (0/{total})"));
            return Box::new(NullProgress);
        }
        let pb = indicatif::ProgressBar::new(total);
        pb.set_style(
            indicatif::ProgressStyle::with_template(
                "{spinner:.cyan} {msg} [{bar:30.cyan/blue}] {pos}/{len}",
            )
            .unwrap_or_else(|_| indicatif::ProgressStyle::default_bar())
            .progress_chars("█░"),
        );
        pb.set_message(msg.to_string());
        Box::new(IndicatifProgress { pb })
    }
}

struct IndicatifSpinner {
    pb: indicatif::ProgressBar,
}

impl SpinnerHandle for IndicatifSpinner {
    fn set_message(&mut self, msg: &str) {
        self.pb.set_message(msg.to_string());
    }
    fn finish_success(&mut self, msg: &str) {
        self.pb.finish_with_message(msg.to_string());
    }
    fn finish_error(&mut self, msg: &str) {
        self.pb.abandon_with_message(msg.to_string());
    }
    fn abandon(&mut self) {
        self.pb.finish_and_clear();
    }
}

impl Drop for IndicatifSpinner {
    fn drop(&mut self) {
        if !self.pb.is_finished() {
            self.pb.finish_and_clear();
        }
    }
}

struct IndicatifProgress {
    pb: indicatif::ProgressBar,
}

impl ProgressHandle for IndicatifProgress {
    fn set_position(&mut self, pos: u64) {
        self.pb.set_position(pos);
    }
    fn inc(&mut self, delta: u64) {
        self.pb.inc(delta);
    }
    fn set_message(&mut self, msg: &str) {
        self.pb.set_message(msg.to_string());
    }
    fn finish_success(&mut self, msg: &str) {
        self.pb.finish_with_message(msg.to_string());
    }
    fn finish_error(&mut self, msg: &str) {
        self.pb.abandon_with_message(msg.to_string());
    }
}

impl Drop for IndicatifProgress {
    fn drop(&mut self) {
        if !self.pb.is_finished() {
            self.pb.finish_and_clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_ui_records() {
        let ui = NullUi::recording();
        ui.success("ok");
        ui.error("bad");
        let lines = ui.lines();
        assert!(lines.iter().any(|l| l == "success:ok"));
        assert!(lines.iter().any(|l| l == "error:bad"));
    }

    #[test]
    fn null_progress_is_noop() {
        let ui = NullUi::new();
        let mut sp = ui.spinner("work");
        sp.set_message("still");
        sp.finish_success("done");
        let mut pb = ui.progress(10, "parse");
        pb.inc(1);
        pb.finish_success("done");
    }
}
