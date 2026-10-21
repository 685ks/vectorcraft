//! Help → project links (this fork's GitHub repository and its issue tracker). The commands
//! return the URL; the frontend opens it. This build is Arthavan Projects' fork of
//! storytold/vectorcraft, not affiliated with or endorsed by ArtCraft.

use serde_json::{Value, json};

use super::*;

/// The app's id on GitHub.
pub const APP_ID: &str = "vectorcraft";
/// The project this build is a fork of.
pub const UPSTREAM_URL: &str = "https://github.com/storytold/vectorcraft";

/// This app's source repository.
pub fn github_url() -> String {
    format!("https://github.com/685ks/{APP_ID}")
}

/// Where bug reports go.
pub fn issues_url() -> String {
    format!("{}/issues", github_url())
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(query "help.github", "VectorCraft on GitHub", ["Help"], None, "{} → {url} source code, issues and releases", always, |_, _| url(github_url())),
        cmd!(query "help.reportIssue", "Report an Issue…", ["Help"], None, "{} → {url} the issue tracker", always, |_, _| url(issues_url())),
        cmd!(query "help.links", "Links", [], None, "{} → {github, issues, upstream}", always, |_, _| {
            Ok(json!({ "github": github_url(), "issues": issues_url(), "upstream": UPSTREAM_URL }))
        }),
    ]
}

fn url(u: String) -> Result<Value> {
    Ok(json!({ "url": u }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links() {
        let mut s = Session::new();
        assert_eq!(s.execute("help.github", &json!({})).unwrap()["url"], "https://github.com/685ks/vectorcraft");
        assert_eq!(s.execute("help.reportIssue", &json!({})).unwrap()["url"], "https://github.com/685ks/vectorcraft/issues");
        assert_eq!(s.execute("help.links", &json!({})).unwrap()["upstream"], "https://github.com/storytold/vectorcraft");
        for gone in ["help.discord", "help.website", "help.appPage"] {
            assert!(s.execute(gone, &json!({})).is_err(), "{gone} is an ArtCraft link");
        }
    }
}
