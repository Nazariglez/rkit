use std::borrow::Cow;

use bevy_ecs::prelude::*;
use fluent::{FluentArgs, FluentResource, FluentValue, concurrent::FluentBundle};
use rustc_hash::FxHashMap;
use unic_langid::LanguageIdentifier;

#[derive(Resource)]
pub struct Locales {
    pub log_errors: bool,
    pub selected_lang: String,
    fallback_lang: String,
    content: FxHashMap<String, FluentBundle<FluentResource>>,
}

impl Default for Locales {
    fn default() -> Self {
        Self::new("en", "en")
    }
}

fn line_col_from_byte_pos(source: &str, byte_pos: usize) -> (usize, usize) {
    let mut pos = byte_pos.min(source.len());
    while pos > 0 && !source.is_char_boundary(pos) {
        pos -= 1;
    }

    let before = &source[..pos];
    let line = before.as_bytes().iter().filter(|&&b| b == b'\n').count() + 1;
    let line_start = before.rfind('\n').map(|idx| idx + 1).unwrap_or(0);
    let col = before[line_start..].chars().count() + 1;
    (line, col)
}

impl Locales {
    pub fn new(selected_lang: &str, fallback_lang: &str) -> Self {
        Locales {
            log_errors: cfg!(debug_assertions),
            selected_lang: selected_lang.to_string(),
            fallback_lang: fallback_lang.to_string(),
            content: FxHashMap::default(),
        }
    }

    pub fn load(&mut self, lang: &str, source: &str) -> Result<(), String> {
        let res = FluentResource::try_new(source.to_string()).map_err(|(_self, errs)| {
            errs.iter()
                .map(|e| {
                    let (line, col) = line_col_from_byte_pos(source, e.pos.start);
                    format!("line {line}, col {col}: {e}")
                })
                .collect::<Vec<_>>()
                .join("; ")
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
        bundle.add_builtins().map_err(|err| err.to_string())?;

        self.content.insert(lang.to_string(), bundle);
        log::info!("Locale '{lang}' added.");
        Ok(())
    }

    #[inline]
    pub fn translate<'b>(
        &'b self,
        key: &'b str,
        args: Option<&LocaleArgs<'b>>,
    ) -> Result<Cow<'b, str>, Vec<String>> {
        self.translate_with_lang(&self.selected_lang, key, args)
    }

    pub fn translate_with_lang<'b>(
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

        if self.log_errors {
            log::warn!("Locale '{lang}' missing key: {key}");
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
macro_rules! tr_lang {
    // without arguments
    ($lang:expr, $loc:expr, $key:expr) => {{
        match $loc.translate_with_lang($lang, $key, None) {
            Ok(cow) => cow,
            Err(errs) => {
                if $loc.log_errors {
                    log::warn!("Translation errors for key `{}`: {:?}", $key, errs);
                }
                std::borrow::Cow::Borrowed($key)
            }
        }
    }};

    // with named arguments
    ($lang:expr, $loc:expr, $key:expr, $($arg_name:ident = $arg_val:expr),+ $(,)?) => {{
        let mut args = LocaleArgs::default();
        $( args.set(stringify!($arg_name), $arg_val); )+
        match $loc.translate_with_lang($lang, $key, Some(&args)) {
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

#[macro_export]
macro_rules! tr {
    // without arguments
    ($loc:expr, $key:expr) => {
        tr_lang!(&$loc.selected_lang, $loc, $key)
    };

    // With named arguments
    ($loc:expr, $key:expr, $($arg_name:ident = $arg_val:expr),+ $(,)?) => {
        tr_lang!(&$loc.selected_lang, $loc, $key, $($arg_name = $arg_val),+)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    /// strip unicode direction isolates before comparing
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

    const FTL_BAD: &str = r#"
hello = Hello
@bad = nope
"#;

    #[test]
    fn test_english() {
        let mut loc = Locales::default();
        loc.load("en", FTL_EN).unwrap();
        assert_eq!(
            strip_isolates(&loc.translate_with_lang("en", "hello", None).unwrap()),
            "Hello, world!"
        );

        let mut a = LocaleArgs::default();
        a.set("n", 5);
        assert_eq!(
            strip_isolates(&loc.translate_with_lang("en", "count", Some(&a)).unwrap()),
            "You have 5 messages."
        );
    }

    #[test]
    fn test_spanish_and_fallback() {
        let mut loc = Locales::default();
        loc.load("en", FTL_EN).unwrap();
        loc.load("es", FTL_ES).unwrap();

        assert_eq!(
            strip_isolates(&loc.translate_with_lang("es", "hello", None).unwrap()),
            "¡Hola, mundo!"
        );
        assert_eq!(
            strip_isolates(&loc.translate_with_lang("fr", "bye", None).unwrap()),
            "Goodbye"
        );
    }

    #[test]
    fn test_load_error_includes_line_col_single_line() {
        let mut loc = Locales::default();
        let err = loc.load("en", FTL_BAD).unwrap_err();
        assert!(err.contains("line 2"));
        assert!(err.contains("col"));
        assert!(!err.contains('\n'));
    }

    #[test]
    fn test_rtl() {
        let mut loc = Locales::default();
        loc.load("en", FTL_EN).unwrap();
        loc.load("ar", FTL_AR).unwrap();

        assert_eq!(
            strip_isolates(&loc.translate_with_lang("ar", "hello", None).unwrap()),
            "مرحبا بالعالم"
        );
        let mut a = LocaleArgs::default();
        a.set("n", 2);
        assert_eq!(
            strip_isolates(&loc.translate_with_lang("ar", "count", Some(&a)).unwrap()),
            "لديك 2 رسائل."
        );
    }

    #[test]
    fn test_russian() {
        let mut loc = Locales::default();
        loc.load("en", FTL_EN).unwrap();
        loc.load("ru", FTL_RU).unwrap();

        assert_eq!(
            strip_isolates(&loc.translate_with_lang("ru", "hello", None).unwrap()),
            "Привет, мир!"
        );
        let mut a = LocaleArgs::default();
        a.set("n", 3);
        assert_eq!(
            strip_isolates(&loc.translate_with_lang("ru", "count", Some(&a)).unwrap()),
            "У вас 3 сообщений."
        );
    }

    #[test]
    fn test_macro_simple_and_args() {
        let mut loc = Locales::default();
        loc.load("en", FTL_EN).unwrap();
        loc.load("es", FTL_ES).unwrap();
        loc.load("ru", FTL_RU).unwrap();

        assert_eq!(
            strip_isolates(&tr_lang!("ru", loc, "hello")),
            "Привет, мир!"
        );
        assert_eq!(
            strip_isolates(&tr_lang!("ru", loc, "count", n = 7)),
            "У вас 7 сообщений."
        );
    }

    #[test]
    fn test_macro_missing_and_fallback() {
        let mut loc = Locales::new("fr", "es");
        loc.load("es", FTL_ES).unwrap();

        assert_eq!(
            strip_isolates(&tr_lang!("fr", loc, "hello")),
            "¡Hola, mundo!"
        );
        assert_eq!(strip_isolates(&tr_lang!("es", loc, "nope")), "nope");
    }

    #[test]
    fn test_translate_default_method() {
        let mut loc = Locales::default();
        loc.load("en", FTL_EN).unwrap();
        loc.load("es", FTL_ES).unwrap();

        assert_eq!(
            strip_isolates(&loc.translate("hello", None).unwrap()),
            "Hello, world!"
        );

        // switch selected_lang to spanish
        loc.selected_lang = "es".to_string();
        let mut args = LocaleArgs::default();
        args.set("n", 4);
        assert_eq!(
            strip_isolates(&loc.translate("count", Some(&args)).unwrap()),
            "Tienes 4 mensajes."
        );
    }

    #[test]
    fn test_macro_default_lang_simple_and_args() {
        let mut loc = Locales::default();
        loc.load("en", FTL_EN).unwrap();
        loc.load("ru", FTL_RU).unwrap();

        // should use loc.selected_lang ("en")
        assert_eq!(strip_isolates(&tr!(&loc, "hello")), "Hello, world!");

        // change selected_lang to russian
        loc.selected_lang = "ru".to_string();
        assert_eq!(strip_isolates(&tr!(&loc, "hello")), "Привет, мир!");

        // with args
        assert_eq!(
            strip_isolates(&tr!(&loc, "count", n = 10)),
            "У вас 10 сообщений."
        );
    }

    #[test]
    fn test_macro_default_fallback_and_missing_key() {
        let mut loc = Locales::new("fr", "en");
        loc.load("en", FTL_EN).unwrap();

        // key exists in fallback
        assert_eq!(strip_isolates(&tr!(&loc, "bye")), "Goodbye");

        // should return the key
        assert_eq!(
            strip_isolates(&tr!(&loc, "does_not_exist")),
            "does_not_exist"
        );
    }
}
