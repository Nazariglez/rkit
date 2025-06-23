use std::borrow::Cow;

use bevy_ecs::prelude::*;
use fluent::{FluentArgs, FluentResource, FluentValue, concurrent::FluentBundle};
use rustc_hash::FxHashMap;
use unic_langid::LanguageIdentifier;

#[derive(Resource)]
pub struct Locales {
    pub log_errors: bool,
    fallback_lang: String,
    content: FxHashMap<String, FluentBundle<FluentResource>>,
}

impl Default for Locales {
    fn default() -> Self {
        // default fallback to English
        Locales::new(cfg!(debug_assertions), "en")
    }
}

impl Locales {
    pub fn new(log_errors: bool, fallback_lang: &str) -> Self {
        Locales {
            log_errors,
            fallback_lang: fallback_lang.to_string(),
            content: FxHashMap::default(),
        }
    }

    pub fn load(&mut self, lang: &str, source: &str) -> Result<(), String> {
        let res = FluentResource::try_new(source.to_string()).map_err(|(_self, errs)| {
            errs.iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("\n")
        })?;
        let lang_id: LanguageIdentifier = lang
            .parse()
            .map_err(|e| format!("Cannot parse '{lang}' into a language id: {e:?}"))?;
        let mut bundle = FluentBundle::new_concurrent(vec![lang_id]);
        bundle.add_resource(res).map_err(|errs| {
            errs.iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("\n")
        })?;
        self.content.insert(lang.to_string(), bundle);
        log::info!("Locale '{lang}' added.");
        Ok(())
    }

    pub fn translate<'b>(
        &'b self,
        lang: &str,
        key: &'b str,
        args: Option<&LocaleArgs<'b>>,
    ) -> Result<Cow<'b, str>, Vec<String>> {
        let bundle_opt = self
            .content
            .get(lang)
            .or_else(|| self.content.get(&self.fallback_lang));

        if let Some(bundle) = bundle_opt {
            if let Some(message) = bundle.get_message(key) {
                if let Some(pattern) = message.value() {
                    let mut errors = Vec::new();
                    let parsed =
                        bundle.format_pattern(pattern, args.map(|a| &a.inner), &mut errors);
                    if errors.is_empty() {
                        return Ok(parsed);
                    } else {
                        return Err(errors.into_iter().map(|e| e.to_string()).collect());
                    }
                }
            }
        }

        Ok(Cow::Borrowed(key))
    }
}

#[derive(Debug, Default)]
pub struct LocaleArgs<'a> {
    inner: FluentArgs<'a>,
}

impl<'a> LocaleArgs<'a> {
    /// Set an argument with given name and value.
    pub fn set<V>(&mut self, name: &'a str, value: V)
    where
        V: Into<FluentValue<'a>>,
    {
        self.inner.set(name, value);
    }
}

#[macro_export]
macro_rules! tr {
    // no-arg version
    ( $loc:expr, $lang:expr, $key:expr ) => {{
        match $loc.translate($lang, $key, None) {
            Ok(cow) => cow,
            Err(errs) => {
                if $loc.log_errors {
                    log::warn!("Translation errors for key `{}`: {:?}", $key, errs);
                }
                std::borrow::Cow::Borrowed($key)
            }
        }
    }};

    // version with args
    ( $loc:expr, $lang:expr, $key:expr, $($arg_name:ident = $arg_val:expr),+ $(,)? ) => {{
        let mut args = LocaleArgs::default();
        $( args.set(stringify!($arg_name), $arg_val); )+
        match $loc.translate($lang, $key, Some(&args)) {
            Ok(cow) => cow,
            Err(errs) => {
                if $loc.log_errors {
                    log::warn!("Translation errors for key `{}`: {:?}", $key, errs);
                }
                std::borrow::Cow::Borrowed($key)
            }
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Strip Unicode direction isolates before comparing
    fn strip_isolates(s: &str) -> String {
        s.chars()
            .filter(|&c| !(matches!(c, '\u{2066}' | '\u{2067}' | '\u{2068}' | '\u{2069}')))
            .collect()
    }

    const FTL_EN: &str = r#"
hello = Hello, world!
bye = Goodbye
count = You have { $n } messages.
"#;

    const FTL_ES: &str = r#"
hello = ¡Hola, mundo!
bye = Adiós
count = Tienes { $n } mensajes.
"#;

    const FTL_AR: &str = r#"
hello = مرحبا بالعالم
bye = مع السلامة
count = لديك { $n } رسائل.
"#;

    const FTL_RU: &str = r#"
hello = Привет, мир!
bye = До свидания
count = У вас { $n } сообщений.
"#;

    #[test]
    fn test_english() {
        let mut loc = Locales::new(false, "en");
        loc.load("en", FTL_EN).unwrap();
        assert_eq!(
            strip_isolates(&loc.translate("en", "hello", None).unwrap()),
            "Hello, world!"
        );

        let mut a = LocaleArgs::default();
        a.set("n", 5);
        assert_eq!(
            strip_isolates(&loc.translate("en", "count", Some(&a)).unwrap()),
            "You have 5 messages."
        );
    }

    #[test]
    fn test_spanish_and_fallback() {
        let mut loc = Locales::new(false, "en");
        loc.load("en", FTL_EN).unwrap();
        loc.load("es", FTL_ES).unwrap();

        assert_eq!(
            strip_isolates(&loc.translate("es", "hello", None).unwrap()),
            "¡Hola, mundo!"
        );
        assert_eq!(
            strip_isolates(&loc.translate("fr", "bye", None).unwrap()),
            "Goodbye"
        );
    }

    #[test]
    fn test_rtl() {
        let mut loc = Locales::new(false, "en");
        loc.load("en", FTL_EN).unwrap();
        loc.load("ar", FTL_AR).unwrap();

        assert_eq!(
            strip_isolates(&loc.translate("ar", "hello", None).unwrap()),
            "مرحبا بالعالم"
        );
        let mut a = LocaleArgs::default();
        a.set("n", 2);
        assert_eq!(
            strip_isolates(&loc.translate("ar", "count", Some(&a)).unwrap()),
            "لديك 2 رسائل."
        );
    }

    #[test]
    fn test_russian() {
        let mut loc = Locales::new(false, "en");
        loc.load("en", FTL_EN).unwrap();
        loc.load("ru", FTL_RU).unwrap();

        assert_eq!(
            strip_isolates(&loc.translate("ru", "hello", None).unwrap()),
            "Привет, мир!"
        );
        let mut a = LocaleArgs::default();
        a.set("n", 3);
        assert_eq!(
            strip_isolates(&loc.translate("ru", "count", Some(&a)).unwrap()),
            "У вас 3 сообщений."
        );
    }

    #[test]
    fn test_macro_simple_and_args() {
        let mut loc = Locales::new(false, "en");
        loc.load("en", FTL_EN).unwrap();
        loc.load("es", FTL_ES).unwrap();
        loc.load("ru", FTL_RU).unwrap();

        assert_eq!(strip_isolates(&tr!(loc, "ru", "hello")), "Привет, мир!");
        assert_eq!(
            strip_isolates(&tr!(loc, "ru", "count", n = 7)),
            "У вас 7 сообщений."
        );
    }

    #[test]
    fn test_macro_missing_and_fallback() {
        let mut loc = Locales::new(false, "es");
        loc.load("es", FTL_ES).unwrap();

        assert_eq!(strip_isolates(&tr!(loc, "fr", "hello")), "¡Hola, mundo!");
        assert_eq!(strip_isolates(&tr!(loc, "es", "nope")), "nope");
    }
}
