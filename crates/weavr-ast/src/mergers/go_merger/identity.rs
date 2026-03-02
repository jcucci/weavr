//! Declaration identity types for matching Go declarations across conflict sides.
//!
//! Two declarations from different conflict sides represent "the same thing" if they
//! share the same identity. This is the key mechanism that lets the merger
//! detect additions, deletions, and modifications.

/// Identity of a top-level Go declaration, used as a map key for matching
/// declarations across conflict sides.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(super) enum GoIdentity {
    /// An import declaration, keyed by module path and kind.
    Import(ImportKey),
    /// A function declaration, keyed by name.
    Function(String),
    /// A method declaration, keyed by receiver type and method name.
    Method(String, String),
    /// A type declaration (struct, interface, type alias), keyed by name.
    Type(String),
    /// A const declaration, keyed by name or group key.
    Const(String),
    /// A var declaration, keyed by name or group key.
    Var(String),
    /// Fallback for unrecognized declarations, keyed by source text.
    Unknown(String),
}

/// Key for identifying an import across conflict sides.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(super) struct ImportKey {
    /// The import path (e.g., `"fmt"`, `"github.com/pkg/errors"`).
    pub path: String,
    /// The kind of import (normal, blank, named, dot).
    pub kind: ImportKind,
}

/// The kind of Go import.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(super) enum ImportKind {
    /// `import "fmt"`
    Normal,
    /// `import _ "fmt"`
    Blank,
    /// `import f "fmt"` — alias is part of the key.
    Named(String),
    /// `import . "fmt"`
    Dot,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_equality() {
        assert_eq!(
            GoIdentity::Function("main".to_string()),
            GoIdentity::Function("main".to_string()),
        );
        assert_ne!(
            GoIdentity::Function("main".to_string()),
            GoIdentity::Function("init".to_string()),
        );
    }

    #[test]
    fn identity_ordering() {
        let a = GoIdentity::Function("alpha".to_string());
        let b = GoIdentity::Function("beta".to_string());
        assert!(a < b);
    }

    #[test]
    fn import_key_equality() {
        let a = ImportKey {
            path: "fmt".to_string(),
            kind: ImportKind::Normal,
        };
        let b = ImportKey {
            path: "fmt".to_string(),
            kind: ImportKind::Normal,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn import_key_different_kind() {
        let normal = ImportKey {
            path: "fmt".to_string(),
            kind: ImportKind::Normal,
        };
        let blank = ImportKey {
            path: "fmt".to_string(),
            kind: ImportKind::Blank,
        };
        assert_ne!(normal, blank);
    }

    #[test]
    fn named_import_includes_alias() {
        let a = ImportKey {
            path: "fmt".to_string(),
            kind: ImportKind::Named("f".to_string()),
        };
        let b = ImportKey {
            path: "fmt".to_string(),
            kind: ImportKind::Named("ff".to_string()),
        };
        assert_ne!(a, b);
    }

    #[test]
    fn different_identity_variants_not_equal() {
        let func = GoIdentity::Function("Foo".to_string());
        let typ = GoIdentity::Type("Foo".to_string());
        assert_ne!(func, typ);
    }

    #[test]
    fn method_identity_includes_receiver() {
        let a = GoIdentity::Method("MyStruct".to_string(), "Foo".to_string());
        let b = GoIdentity::Method("OtherStruct".to_string(), "Foo".to_string());
        assert_ne!(a, b);
    }
}
