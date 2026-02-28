//! Declaration identity types for matching TypeScript declarations across conflict sides.
//!
//! Two declarations from different conflict sides represent "the same thing" if they
//! share the same identity. This is the key mechanism that lets the merger
//! detect additions, deletions, and modifications.

/// Identity of a top-level TypeScript declaration, used as a map key for matching
/// declarations across conflict sides.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(super) enum TsIdentity {
    /// An import statement, keyed by module specifier and import kind.
    Import(ImportKey),
    /// An export statement, keyed by source text.
    Export(String),
    /// A function declaration, keyed by name.
    Function(String),
    /// A class declaration, keyed by name.
    Class(String),
    /// An interface declaration, keyed by name.
    Interface(String),
    /// A type alias declaration, keyed by name.
    TypeAlias(String),
    /// An enum declaration, keyed by name.
    Enum(String),
    /// A variable declaration, keyed by name.
    Variable(String),
    /// A namespace/module declaration, keyed by name.
    Namespace(String),
}

/// Import identity key: module specifier + import kind.
///
/// Two imports share an identity when they have the same module specifier AND
/// same kind. This allows merging `import { A } from 'react'` (left) with
/// `import { B } from 'react'` (right) while keeping `import type { T } from 'react'`
/// separate.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(super) struct ImportKey {
    /// The module specifier (e.g., `react`, `./utils`).
    pub module: String,
    /// The kind of import (value, type-only, side-effect, namespace).
    pub kind: ImportKind,
}

/// The kind of an import statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(super) enum ImportKind {
    /// `import { A } from 'x'` — named value imports.
    Value,
    /// `import type { T } from 'x'` — type-only imports.
    TypeOnly,
    /// `import './polyfill'` — side-effect-only imports.
    SideEffect,
    /// `import * as X from 'x'` — namespace imports.
    Namespace,
}

/// A single import specifier within an import statement.
///
/// Represents one name from `import { A, B as C } from 'x'`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(super) struct ImportSpecifier {
    /// The imported name (e.g., `A` or `B`).
    pub name: String,
    /// The local alias, if any (e.g., `C` in `B as C`).
    pub alias: Option<String>,
    /// Whether this specific specifier is type-only (e.g., `type A` in `import { type A }`).
    pub is_type: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_key_equality() {
        let a = ImportKey {
            module: "react".to_string(),
            kind: ImportKind::Value,
        };
        let b = ImportKey {
            module: "react".to_string(),
            kind: ImportKind::Value,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn import_key_different_kind() {
        let value = ImportKey {
            module: "react".to_string(),
            kind: ImportKind::Value,
        };
        let type_only = ImportKey {
            module: "react".to_string(),
            kind: ImportKind::TypeOnly,
        };
        assert_ne!(value, type_only);
    }

    #[test]
    fn import_key_different_module() {
        let a = ImportKey {
            module: "react".to_string(),
            kind: ImportKind::Value,
        };
        let b = ImportKey {
            module: "vue".to_string(),
            kind: ImportKind::Value,
        };
        assert_ne!(a, b);
    }

    #[test]
    fn specifier_equality() {
        let a = ImportSpecifier {
            name: "useState".to_string(),
            alias: None,
            is_type: false,
        };
        let b = ImportSpecifier {
            name: "useState".to_string(),
            alias: None,
            is_type: false,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn specifier_with_alias() {
        let a = ImportSpecifier {
            name: "useState".to_string(),
            alias: Some("state".to_string()),
            is_type: false,
        };
        let b = ImportSpecifier {
            name: "useState".to_string(),
            alias: None,
            is_type: false,
        };
        assert_ne!(a, b);
    }

    #[test]
    fn specifier_ordering() {
        let a = ImportSpecifier {
            name: "A".to_string(),
            alias: None,
            is_type: false,
        };
        let b = ImportSpecifier {
            name: "B".to_string(),
            alias: None,
            is_type: false,
        };
        assert!(a < b);
    }

    #[test]
    fn identity_variants_not_equal() {
        let func = TsIdentity::Function("Foo".to_string());
        let class = TsIdentity::Class("Foo".to_string());
        assert_ne!(func, class);
    }
}
