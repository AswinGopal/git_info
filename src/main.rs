//! src/main.rs
//! Print a coloured Git-status segment suitable for Bash/Zsh prompts.
//!
//! Build: `cargo build --release --bin git_info`

use std::fmt::Write as _;
use std::process::Command;

fn main() {
    // ─── Run git status in porcelain-v2 mode ──────────────────────────
    let output = match Command::new("git")
        .args(["status", "--porcelain=v2", "-b"])
        .output()
    {
        Ok(out) if out.status.success() => out,
        _ => return, // not a Git repo or git missing
    };

    // Convert bytes → str (lossy is fine for prompt)
    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut repo = RepoStatus::default();

    for line in stdout.lines() {
        if line.starts_with("# ") {
            repo.parse_header_line(line);
        } else {
            repo.parse_record_line(line);
        }
    }

    println!("{}", repo.render_prompt());
}

/* ───────────────────────── RepoStatus ───────────────────────────── */

#[derive(Default)]
struct RepoStatus {
    branch_head: Option<String>, // e.g. "main" or "(detached)"
    branch_oid: Option<String>,  // full 40-char hash
    ahead: u32,
    behind: u32,
    staged: u32,
    unstaged: u32,
    untracked: u32,
}

impl RepoStatus {
    /* ----- Header lines: "# branch.*" -------------------------------- */
    fn parse_header_line(&mut self, line: &str) {
        // strip leading "# "
        let mut words = line[2..].split_whitespace();
        match words.next() {
            Some("branch.oid") => {
                if let Some(oid) = words.next() {
                    self.branch_oid = Some(oid.to_string());
                }
            }
            Some("branch.head") => {
                if let Some(head) = words.next() {
                    self.branch_head = Some(head.to_string());
                }
            }
            Some("branch.ab") => {
                for token in words {
                    if let Some(rest) = token.strip_prefix('+') {
                        self.ahead = rest.parse().unwrap_or(0);
                    } else if let Some(rest) = token.strip_prefix('-') {
                        self.behind = rest.parse().unwrap_or(0);
                    }
                }
            }
            _ => {}
        }
    }

    /* ----- Record lines: file status --------------------------------- */
    fn parse_record_line(&mut self, line: &str) {
        if line.starts_with("? ") {
            self.untracked += 1;
            return;
        }
        if line.starts_with("! ") {
            return; // ignored file
        }

        // Format: "1 XY ...", "2 XY ..." or "u XY ..."
        //        index   worktree
        let mut parts = line.split_whitespace();
        let _rec_type = parts.next(); // '1', '2', 'u', etc.
        let xy = parts.next().unwrap_or(".."); // "XY"

        let x = xy.chars().nth(0).unwrap_or('.');
        let y = xy.chars().nth(1).unwrap_or('.');

        if x != '.' && x != ' ' {
            self.staged += 1;
        }
        if y != '.' && y != ' ' {
            self.unstaged += 1;
        }
    }

    /* ----- Render coloured segment ----------------------------------- */
    fn render_prompt(&self) -> String {
        // Decide what to show as the branch label
        let branch_label = match self.branch_head.as_deref() {
            Some("(detached)") | None => {
                // Fallback to short commit hash
                self.branch_oid
                    .as_deref()
                    .map(|h| &h[..7])
                    .unwrap_or("DETACHED")
                    .to_string()
            }
            Some(name) => name.to_string(),
        };

        // Assemble the text part
        let mut text = String::with_capacity(64);
        write!(text, " {}", branch_label).unwrap();

        if self.ahead > 0 {
            write!(text, " ↑{}", self.ahead).unwrap();
        }
        if self.behind > 0 {
            write!(text, " ↓{}", self.behind).unwrap();
        }
        if self.staged > 0 {
            write!(text, " [!{}]", self.staged).unwrap();
        }
        if self.unstaged > 0 {
            write!(text, " [+{}]", self.unstaged).unwrap();
        }
        if self.untracked > 0 {
            write!(text, " [?{}]", self.untracked).unwrap();
        }

        // 24-bit colour background + white foreground
        format!("\x1b[38;2;255;255;255;48;2;6;150;154m {} \x1b[0m", text)
    }
}
