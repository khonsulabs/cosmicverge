use include_dir::include_dir;
use unic_langid::{langid, LanguageIdentifier};

pub const US_ENGLISH: LanguageIdentifier = langid!("en-US");

fluent_templates::static_loader! {
    pub static LOCALES = {
        locales: "./src/strings",
        fallback_language: "en-US",
    };
}

// TODO This is only here because of https://github.com/XAMPPRocky/fluent-templates/issues/2
#[allow(dead_code)]
fn unused() {
    include_dir!("./src/strings");
}
