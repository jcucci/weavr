//! Language detection types.
//!
//! All types in this module are **stable** and covered by semantic versioning.

use std::fmt;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Programming or markup language identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    /// Rust (.rs)
    Rust,
    /// C# (.cs)
    CSharp,
    /// TypeScript (.ts, .tsx)
    TypeScript,
    /// JavaScript (.js, .jsx)
    JavaScript,
    /// Go (.go)
    Go,
    /// Python (.py)
    Python,
    /// Ruby (.rb)
    Ruby,
    /// Java (.java)
    Java,
    /// Kotlin (.kt, .kts)
    Kotlin,
    /// Swift (.swift)
    Swift,
    /// C (.c, .h)
    C,
    /// C++ (.cpp, .cc, .cxx, .hpp)
    Cpp,
    /// JSON (.json)
    Json,
    /// YAML (.yaml, .yml)
    Yaml,
    /// TOML (.toml)
    Toml,
    /// Markdown (.md)
    Markdown,
    /// Unknown or unsupported language.
    Unknown,
}

impl Language {
    /// Returns a human-readable display name for the language.
    #[must_use]
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::CSharp => "csharp",
            Self::TypeScript => "typescript",
            Self::JavaScript => "javascript",
            Self::Go => "go",
            Self::Python => "python",
            Self::Ruby => "ruby",
            Self::Java => "java",
            Self::Kotlin => "kotlin",
            Self::Swift => "swift",
            Self::C => "c",
            Self::Cpp => "cpp",
            Self::Json => "json",
            Self::Yaml => "yaml",
            Self::Toml => "toml",
            Self::Markdown => "markdown",
            Self::Unknown => "unknown",
        }
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.display_name())
    }
}

/// Detects programming language from a file path's extension.
///
/// Extension matching is case-insensitive. Returns [`Language::Unknown`] for
/// unrecognized extensions or paths without an extension. This function
/// performs no filesystem I/O.
#[must_use]
pub fn detect_language(path: &Path) -> Language {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return Language::Unknown;
    };

    match ext.to_lowercase().as_str() {
        "rs" => Language::Rust,
        "cs" => Language::CSharp,
        "ts" | "tsx" => Language::TypeScript,
        "js" | "jsx" => Language::JavaScript,
        "go" => Language::Go,
        "py" => Language::Python,
        "rb" => Language::Ruby,
        "java" => Language::Java,
        "kt" | "kts" => Language::Kotlin,
        "swift" => Language::Swift,
        "c" | "h" => Language::C,
        "cpp" | "cc" | "cxx" | "hpp" => Language::Cpp,
        "json" => Language::Json,
        "yaml" | "yml" => Language::Yaml,
        "toml" => Language::Toml,
        "md" => Language::Markdown,
        _ => Language::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_rust() {
        assert_eq!(detect_language(Path::new("src/main.rs")), Language::Rust);
    }

    #[test]
    fn detect_csharp() {
        assert_eq!(detect_language(Path::new("Program.cs")), Language::CSharp);
    }

    #[test]
    fn detect_typescript_variants() {
        assert_eq!(detect_language(Path::new("index.ts")), Language::TypeScript);
        assert_eq!(detect_language(Path::new("App.tsx")), Language::TypeScript);
    }

    #[test]
    fn detect_javascript_variants() {
        assert_eq!(detect_language(Path::new("app.js")), Language::JavaScript);
        assert_eq!(detect_language(Path::new("App.jsx")), Language::JavaScript);
    }

    #[test]
    fn detect_go() {
        assert_eq!(detect_language(Path::new("main.go")), Language::Go);
    }

    #[test]
    fn detect_python() {
        assert_eq!(detect_language(Path::new("script.py")), Language::Python);
    }

    #[test]
    fn detect_ruby() {
        assert_eq!(detect_language(Path::new("app.rb")), Language::Ruby);
    }

    #[test]
    fn detect_java() {
        assert_eq!(detect_language(Path::new("Main.java")), Language::Java);
    }

    #[test]
    fn detect_kotlin_variants() {
        assert_eq!(detect_language(Path::new("App.kt")), Language::Kotlin);
        assert_eq!(
            detect_language(Path::new("build.gradle.kts")),
            Language::Kotlin
        );
    }

    #[test]
    fn detect_swift() {
        assert_eq!(
            detect_language(Path::new("ViewController.swift")),
            Language::Swift
        );
    }

    #[test]
    fn detect_c_variants() {
        assert_eq!(detect_language(Path::new("main.c")), Language::C);
        assert_eq!(detect_language(Path::new("header.h")), Language::C);
    }

    #[test]
    fn detect_cpp_variants() {
        assert_eq!(detect_language(Path::new("main.cpp")), Language::Cpp);
        assert_eq!(detect_language(Path::new("main.cc")), Language::Cpp);
        assert_eq!(detect_language(Path::new("main.cxx")), Language::Cpp);
        assert_eq!(detect_language(Path::new("header.hpp")), Language::Cpp);
    }

    #[test]
    fn detect_json() {
        assert_eq!(detect_language(Path::new("package.json")), Language::Json);
    }

    #[test]
    fn detect_yaml_variants() {
        assert_eq!(detect_language(Path::new("config.yaml")), Language::Yaml);
        assert_eq!(detect_language(Path::new("config.yml")), Language::Yaml);
    }

    #[test]
    fn detect_toml() {
        assert_eq!(detect_language(Path::new("Cargo.toml")), Language::Toml);
    }

    #[test]
    fn detect_markdown() {
        assert_eq!(detect_language(Path::new("README.md")), Language::Markdown);
    }

    #[test]
    fn detect_unknown_extension() {
        assert_eq!(detect_language(Path::new("file.xyz")), Language::Unknown);
    }

    #[test]
    fn detect_no_extension() {
        assert_eq!(detect_language(Path::new("Makefile")), Language::Unknown);
    }

    #[test]
    fn detect_case_insensitive() {
        assert_eq!(detect_language(Path::new("Main.RS")), Language::Rust);
        assert_eq!(detect_language(Path::new("App.Tsx")), Language::TypeScript);
        assert_eq!(detect_language(Path::new("Main.JAVA")), Language::Java);
    }

    #[test]
    fn display_name_roundtrip() {
        let languages = [
            Language::Rust,
            Language::CSharp,
            Language::TypeScript,
            Language::JavaScript,
            Language::Go,
            Language::Python,
            Language::Ruby,
            Language::Java,
            Language::Kotlin,
            Language::Swift,
            Language::C,
            Language::Cpp,
            Language::Json,
            Language::Yaml,
            Language::Toml,
            Language::Markdown,
            Language::Unknown,
        ];

        for lang in languages {
            // display_name and Display trait should produce the same output
            assert_eq!(lang.display_name(), lang.to_string());
        }
    }

    #[test]
    fn serde_roundtrip() {
        let lang = Language::Rust;
        let json = serde_json::to_string(&lang).unwrap();
        let deserialized: Language = serde_json::from_str(&json).unwrap();
        assert_eq!(lang, deserialized);
    }

    #[test]
    fn serde_roundtrip_all_variants() {
        let languages = [
            Language::Rust,
            Language::CSharp,
            Language::TypeScript,
            Language::Unknown,
        ];

        for lang in languages {
            let json = serde_json::to_string(&lang).unwrap();
            let deserialized: Language = serde_json::from_str(&json).unwrap();
            assert_eq!(lang, deserialized);
        }
    }
}
