use std::collections::HashMap;

use fluent_templates::{fluent_bundle::FluentValue, loader::Loader};
use include_dir::include_dir;
use unic_langid::{langid, LanguageIdentifier};
use yew::prelude::*;
use yew_bulma::{
    forms::FormField,
    markdown::render_markdown,
    validations::{FieldError, ValidationError},
};

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

pub fn localize(name: &str) -> Html {
    render_markdown(&localize_raw(name))
}

pub fn localize_with_args(name: &str, args: &HashMap<String, FluentValue>) -> Html {
    render_markdown(&localize_raw_with_args(name, args))
}

pub fn localize_raw(name: &str) -> String {
    LOCALES.lookup(&US_ENGLISH, name)
}

pub fn localize_raw_with_args(name: &str, args: &HashMap<String, FluentValue>) -> String {
    LOCALES.lookup_with_args(&US_ENGLISH, name, args)
}

#[macro_export]
macro_rules! localize_html {
    ($name:expr) => {
        crate::strings::localize($name)
    };
    ($name:expr, $($key:expr => $value:expr),+) => {{
        let mut args = std::collections::HashMap::new();
        $(
            args.insert(String::from($key), fluent_templates::fluent_bundle::FluentValue::from(String::from($value)));
        )+
        crate::strings::localize_with_args($name, &args)
    }};
}

#[macro_export]
macro_rules! localize {
    ($name:expr) => {
        crate::strings::localize_raw($name)
    };
    ($name:expr, $($key:expr => $value:expr),+) => {{
        let mut args = std::collections::HashMap::new();
        $(
            args.insert(String::from($key), fluent_templates::fluent_bundle::FluentValue::from($value));
        )+
        crate::strings::localize_raw_with_args($name, &args)
    }};
}

pub trait Namable {
    fn name(&self) -> &'static str;
    fn localized_name(&self) -> String {
        localize!(&self.name())
    }
}

pub fn translate_error<T>(error: &FieldError<T>) -> String
where
    T: FormField,
{
    let key = match error.error {
        ValidationError::NotPresent => "validation-error-not-present",
        ValidationError::NotAbsent => "validation-error-not-absent",
        ValidationError::InvalidValue => "validation-error-invalid-valid",
        ValidationError::Custom(key) => key,
    };

    localize!(key)
}
