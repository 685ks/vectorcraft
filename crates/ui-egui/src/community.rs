//! Project links (About, Home): this fork's GitHub repository and issue tracker, and the
//! provenance note. This build is Arthavan Projects' fork of storytold/vectorcraft.

use egui::Ui;

use crate::theme::Tokens;
use crate::{VectorcraftApp, icons};

/// The provenance line in About and Home. A legal notice, shown in English in every UI language.
pub const FORK_NOTE: &str = "A fork of storytold/vectorcraft, not affiliated with or endorsed by ArtCraft."; // brand-ok: the not-affiliated notice
/// Shown in About when the Arthavan Projects launcher started the app (`ARTHAVAN_LAUNCHER=1`).
/// VectorCraft has no updater of its own and makes no update requests; the launcher installs and
/// updates it from the 685ks/vectorcraft releases.
pub const LAUNCHER_UPDATES_NOTE: &str = "Updates are managed by Arthavan Projects";

/// Whether the Arthavan Projects launcher started this process.
pub fn launcher_managed() -> bool {
    launcher_managed_from(std::env::var("ARTHAVAN_LAUNCHER").ok().as_deref())
}

fn launcher_managed_from(value: Option<&str>) -> bool {
    value.is_some_and(|v| v.trim() == "1")
}

/// One link row: icon, label, opens the command's URL.
fn link(app: &mut VectorcraftApp, ui: &mut Ui, icon: &str, label: &str, cmd: &str, url: &str) {
    let t = Tokens::get(ui.ctx());
    ui.horizontal(|ui| {
        icons::icon(ui, icon, 16.0, t.icon);
        let r = ui.add(egui::Button::new(egui::RichText::new(label).color(t.accent).size(13.0)).frame(false)).on_hover_text(url);
        if r.clicked() {
            app.open_link(cmd);
        }
    });
}

/// The provenance note and the GitHub / issue tracker links.
pub fn links(app: &mut VectorcraftApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    ui.label(egui::RichText::new(FORK_NOTE).color(t.text_dim).size(12.0));
    ui.add_space(6.0);
    let l = app.session.execute("help.links", &serde_json::json!({})).unwrap_or_default();
    let s = |k: &str| l[k].as_str().unwrap_or("").to_string();
    link(app, ui, "git-branch", tl!("Source code on GitHub"), "help.github", &s("github"));
    link(app, ui, "external-link", tl!("Report an Issue…"), "help.reportIssue", &s("issues"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_clicks_open_the_right_urls() {
        use std::sync::{Arc, Mutex};
        let opened = Arc::new(Mutex::new(vec![]));
        let o = opened.clone();
        let services = crate::Services { open_url: Some(Box::new(move |u: &str| o.lock().unwrap().push(u.to_string()))), ..Default::default() };
        let mut app = VectorcraftApp::new(vectorcraft_engine::Session::new(), services);
        for id in ["help.github", "help.reportIssue"] {
            crate::menus::invoke(&mut app, id, serde_json::json!({}));
        }
        assert_eq!(*opened.lock().unwrap(), ["https://github.com/685ks/vectorcraft", "https://github.com/685ks/vectorcraft/issues"]);
        // The control channel / MCP path returns the URL without opening a browser.
        let v = app.run("help.github", serde_json::json!({})).unwrap();
        assert_eq!(v["url"], "https://github.com/685ks/vectorcraft");
        assert_eq!(opened.lock().unwrap().len(), 2);
        let ctx = egui::Context::default();
        crate::theme::install_fonts(&ctx);
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| links(&mut app, ui));
        out.textures_delta.clear();
    }

    #[test]
    fn no_artcraft_links_remain() {
        assert!(FORK_NOTE.contains("storytold/vectorcraft") && FORK_NOTE.contains("not affiliated"));
        let mut s = vectorcraft_engine::Session::new();
        let links = s.execute("help.links", &serde_json::json!({})).unwrap().to_string().to_ascii_lowercase();
        assert!(!links.contains("artcraft.com") && !links.contains("discord"), "{links}");
    }

    #[test]
    fn launcher_flag_is_read_from_arthavan_launcher() {
        assert!(launcher_managed_from(Some("1")));
        assert!(!launcher_managed_from(None));
        assert!(!launcher_managed_from(Some("0")));
        assert!(!launcher_managed_from(Some("")));
    }
}
