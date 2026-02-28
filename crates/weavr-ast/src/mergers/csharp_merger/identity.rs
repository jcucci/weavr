//! Declaration identity types for matching C# declarations across conflict sides.
//!
//! Two declarations from different conflict sides represent "the same thing" if they
//! share the same identity. This is the key mechanism that lets the merger
//! detect additions, deletions, and modifications.

/// Identity of a top-level C# declaration, used as a map key for matching
/// declarations across conflict sides.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(super) enum CSharpIdentity {
    /// A `using` directive, keyed by its normalized path.
    Using(String),
    /// A namespace declaration, keyed by its fully-qualified name.
    Namespace(String),
    /// A class declaration, keyed by name (includes generic arity for `Foo<T>`).
    Class(String),
    /// A struct declaration, keyed by name.
    Struct(String),
    /// An interface declaration, keyed by name.
    Interface(String),
    /// An enum declaration, keyed by name.
    Enum(String),
    /// A delegate declaration, keyed by name.
    Delegate(String),
    /// A record declaration (C# 9+), keyed by name.
    Record(String),
    /// Fallback for unrecognized declarations, keyed by source text.
    Unknown(String),
}

/// Identity of a member inside a class/struct/interface, used for matching
/// members across conflict sides.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(super) enum MemberIdentity {
    /// A method, keyed by name and parameter count (handles overloads).
    Method(String, usize),
    /// A property, keyed by name.
    Property(String),
    /// A field, keyed by name.
    Field(String),
    /// A constructor, keyed by parameter count.
    Constructor(usize),
    /// An event, keyed by name.
    Event(String),
    /// An indexer, keyed by parameter count.
    Indexer(usize),
    /// Fallback, keyed by source text.
    Unknown(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_equality() {
        assert_eq!(
            CSharpIdentity::Using("System".to_string()),
            CSharpIdentity::Using("System".to_string()),
        );
        assert_ne!(
            CSharpIdentity::Using("System".to_string()),
            CSharpIdentity::Using("System.IO".to_string()),
        );
    }

    #[test]
    fn identity_ordering() {
        let a = CSharpIdentity::Using("System".to_string());
        let b = CSharpIdentity::Using("System.IO".to_string());
        assert!(a < b);
    }

    #[test]
    fn member_identity_equality() {
        assert_eq!(
            MemberIdentity::Method("Foo".to_string(), 2),
            MemberIdentity::Method("Foo".to_string(), 2),
        );
        // Different param count = different overload
        assert_ne!(
            MemberIdentity::Method("Foo".to_string(), 2),
            MemberIdentity::Method("Foo".to_string(), 3),
        );
    }

    #[test]
    fn different_identity_variants_not_equal() {
        let class = CSharpIdentity::Class("Foo".to_string());
        let iface = CSharpIdentity::Interface("Foo".to_string());
        assert_ne!(class, iface);
    }
}
