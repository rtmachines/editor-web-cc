use std::fs;
use std::path::Path;

use recipe_domain::identity::Role;

use super::*;

/// Return every key that `t.get("...")` or `t.with("...", ...)` names in the templates.
fn template_keys() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates");
    let mut keys = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let text = fs::read_to_string(&path).unwrap();
        for marker in ["t.get(\"", "t.with(\""] {
            for part in text.split(marker).skip(1) {
                let key = part.split('"').next().unwrap().to_owned();
                keys.push((path.display().to_string(), key));
            }
        }
    }
    keys
}

#[test]
fn pages_texts_every_template_key_exists_in_both_bundles() {
    let bundles = Bundles::load().unwrap();
    let keys = template_keys();
    assert!(!keys.is_empty());
    for (file, key) in keys {
        assert!(bundles.has_message(Lang::En, &key), "{file}: {key} (en)");
        assert!(bundles.has_message(Lang::Pl, &key), "{file}: {key} (pl)");
    }
}

#[test]
fn pages_texts_a_missing_key_shows_the_key() {
    let bundles = Bundles::load().unwrap();
    let texts = Texts::new(&bundles, Lang::En);
    assert_eq!(texts.get("no-such-key"), "no-such-key");
}

#[test]
fn pages_texts_get_follows_the_language() {
    let bundles = Bundles::load().unwrap();
    assert_eq!(
        Texts::new(&bundles, Lang::En).get("language-name"),
        "English"
    );
    assert_eq!(
        Texts::new(&bundles, Lang::Pl).get("language-name"),
        "Polski"
    );
}

#[test]
fn pages_texts_with_inserts_the_arguments() {
    let bundles = Bundles::load().unwrap();
    let mut args = FluentArgs::new();
    args.set("limit", "1 MiB");
    let text = Texts::new(&bundles, Lang::En).with("error-E-WEB-003", &args);
    assert_eq!(text, "The request is larger than 1 MiB.");
}

#[test]
fn pages_texts_role_names_follow_the_language() {
    let bundles = Bundles::load().unwrap();
    let english = Texts::new(&bundles, Lang::En);
    let polish = Texts::new(&bundles, Lang::Pl);
    assert_eq!(english.role(Role::ProcessEngineer), "Process engineer");
    assert_eq!(polish.role(Role::ProcessEngineer), "Inżynier procesu");
    assert_eq!(english.role(Role::Operator), "Operator");
}
