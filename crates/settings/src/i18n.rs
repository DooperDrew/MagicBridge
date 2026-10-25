//! Internationalization (i18n) loader and dictionary for MagicBridge settings.
//!
//! Complies with F7.3 and AGENTS.md:
//! - All user-facing UI text is managed in external/bundled translation files.
//! - English (`en.toml`) and Italian (`it.toml`) are bundled by default.
//! - Fallback to English is provided if a key is missing in another language.

use std::collections::HashMap;
use std::sync::RwLock;

/// Bundled English translation TOML.
const BUNDLED_EN: &str = include_str!("../locales/en.toml");
/// Bundled Italian translation TOML.
const BUNDLED_IT: &str = include_str!("../locales/it.toml");

/// Global or instance-based localization manager.
#[derive(Debug)]
pub struct I18n {
    current_lang: String,
    translations: HashMap<String, HashMap<String, String>>,
}

impl Default for I18n {
    fn default() -> Self {
        Self::new("en")
    }
}

impl I18n {
    /// Creates a new `I18n` instance initialized with the given language code ("en" or "it").
    #[must_use]
    pub fn new(lang: &str) -> Self {
        let mut i18n = Self {
            current_lang: if lang == "it" {
                "it".to_string()
            } else {
                "en".to_string()
            },
            translations: HashMap::new(),
        };

        i18n.load_bundled("en", BUNDLED_EN);
        i18n.load_bundled("it", BUNDLED_IT);

        i18n
    }

    /// Loads translation strings from a TOML document.
    pub fn load_bundled(&mut self, lang: &str, toml_str: &str) {
        if let Ok(toml_val) = toml::from_str::<toml::Table>(toml_str) {
            let mut map = HashMap::new();
            flatten_table("", &toml_val, &mut map);
            self.translations.insert(lang.to_string(), map);
        }
    }

    /// Sets the current active UI language (e.g. "en" or "it").
    pub fn set_language(&mut self, lang: &str) {
        self.current_lang = if self.translations.contains_key(lang) {
            lang.to_string()
        } else {
            "en".to_string()
        };
    }

    /// Returns the current language code.
    #[must_use]
    pub fn current_language(&self) -> &str {
        &self.current_lang
    }

    /// Returns the list of supported languages as `(code, display_name)` pairs.
    #[must_use]
    pub fn available_languages(&self) -> Vec<(&'static str, &'static str)> {
        vec![("en", "English"), ("it", "Italiano")]
    }

    /// Translates a key string into the active language, falling back to English.
    #[must_use]
    pub fn t<'a>(&'a self, key: &'a str) -> &'a str {
        if let Some(map) = self.translations.get(&self.current_lang)
            && let Some(val) = map.get(key)
        {
            return val.as_str();
        }
        if self.current_lang != "en"
            && let Some(en_map) = self.translations.get("en")
            && let Some(val) = en_map.get(key)
        {
            return val.as_str();
        }
        key
    }

    /// Translates a key and substitutes placeholder parameters `{param}` with values.
    #[must_use]
    pub fn t_format(&self, key: &str, args: &[(&str, &str)]) -> String {
        let mut result = self.t(key).to_string();
        for (k, v) in args {
            let pattern = format!("{{{k}}}");
            result = result.replace(&pattern, v);
        }
        result
    }
}

fn flatten_table(prefix: &str, table: &toml::Table, out: &mut HashMap<String, String>) {
    for (k, v) in table {
        let full_key = if prefix.is_empty() {
            k.clone()
        } else {
            format!("{prefix}.{k}")
        };
        match v {
            toml::Value::String(s) => {
                out.insert(full_key, s.clone());
            }
            toml::Value::Table(sub) => {
                flatten_table(&full_key, sub, out);
            }
            _ => {
                out.insert(full_key, v.to_string());
            }
        }
    }
}

/// Global lazy-initialized thread-safe localization state for convenient UI access.
static GLOBAL_I18N: RwLock<Option<I18n>> = RwLock::new(None);

/// Initializes or updates the global language.
pub fn set_global_language(lang: &str) {
    let mut lock = match GLOBAL_I18N.write() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    if let Some(ref mut i18n) = *lock {
        i18n.set_language(lang);
    } else {
        *lock = Some(I18n::new(lang));
    }
}

/// Convenience global translate function.
#[must_use]
pub fn tr(key: &str) -> String {
    let lock = match GLOBAL_I18N.read() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    if let Some(ref i18n) = *lock {
        i18n.t(key).to_string()
    } else {
        key.to_string()
    }
}

/// Convenience global formatted translate function.
#[must_use]
pub fn tr_format(key: &str, args: &[(&str, &str)]) -> String {
    let lock = match GLOBAL_I18N.read() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    if let Some(ref i18n) = *lock {
        i18n.t_format(key, args)
    } else {
        let mut res = key.to_string();
        for (k, v) in args {
            res = res.replace(&format!("{{{k}}}"), v);
        }
        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_i18n_english_translation() {
        let i18n = I18n::new("en");
        assert_eq!(i18n.t("tab.keyboard"), "Keyboard");
        assert_eq!(i18n.t("tab.top_row"), "Top Row & Fn");
        assert_eq!(i18n.t("tab.battery"), "Battery");
        assert_eq!(i18n.t("tab.general"), "General");
        assert_eq!(i18n.t("tab.about"), "About");
    }

    #[test]
    fn test_i18n_italian_translation() {
        let i18n = I18n::new("it");
        assert_eq!(i18n.t("tab.keyboard"), "Tastiera");
        assert_eq!(i18n.t("tab.top_row"), "Tasti Funzione & Fn");
        assert_eq!(i18n.t("tab.battery"), "Batteria");
        assert_eq!(i18n.t("tab.general"), "Generale");
        assert_eq!(i18n.t("tab.about"), "Informazioni");
    }

    #[test]
    fn test_i18n_fallback_to_english() {
        let mut i18n = I18n::new("it");
        // Test fallback for non-existent key returns key itself
        assert_eq!(i18n.t("non.existent.key"), "non.existent.key");
        // Switching language works dynamically
        i18n.set_language("en");
        assert_eq!(i18n.t("tab.keyboard"), "Keyboard");
    }

    #[test]
    fn test_i18n_formatting_args() {
        let i18n = I18n::new("en");
        let formatted = i18n.t_format("profile.delete_confirm", &[("name", "Work & Development")]);
        assert_eq!(
            formatted,
            "Are you sure you want to delete profile 'Work & Development'?"
        );
    }
}
