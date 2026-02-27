//! Item identity types for matching items across conflict sides.
//!
//! Two items from different conflict sides represent "the same thing" if they
//! share the same identity. This is the key mechanism that lets the merger
//! detect additions, deletions, and modifications.

use syn::{ImplItem, Item};

use super::tokens::render_tokens;

/// Identity of a top-level `syn::Item`, used as a map key for matching
/// items across conflict sides.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(super) enum ItemIdentity {
    /// A `use` statement, keyed by its rendered path.
    Use(String),
    /// A function, keyed by name.
    Function(String),
    /// A struct, keyed by name.
    Struct(String),
    /// An enum, keyed by name.
    Enum(String),
    /// A trait, keyed by name.
    Trait(String),
    /// An impl block, keyed by `"Type"` or `"Trait for Type"`.
    Impl(String),
    /// A const item, keyed by name.
    Const(String),
    /// A static item, keyed by name.
    Static(String),
    /// A type alias, keyed by name.
    TypeAlias(String),
    /// A module, keyed by name.
    Mod(String),
    /// A macro definition, keyed by name.
    Macro(String),
    /// Fallback for unrecognized items, keyed by rendered tokens.
    Unknown(String),
}

impl ItemIdentity {
    /// Extracts the identity from a `syn::Item`.
    pub(super) fn from_item(item: &Item) -> Self {
        match item {
            Item::Use(u) => Self::Use(render_tokens(&u.tree)),
            Item::Fn(f) => Self::Function(f.sig.ident.to_string()),
            Item::Struct(s) => Self::Struct(s.ident.to_string()),
            Item::Enum(e) => Self::Enum(e.ident.to_string()),
            Item::Trait(t) => Self::Trait(t.ident.to_string()),
            Item::Impl(i) => {
                let self_ty = render_tokens(&i.self_ty);
                if let Some((_, ref path, _)) = i.trait_ {
                    Self::Impl(format!("{} for {self_ty}", render_tokens(path)))
                } else {
                    Self::Impl(self_ty)
                }
            }
            Item::Const(c) => Self::Const(c.ident.to_string()),
            Item::Static(s) => Self::Static(s.ident.to_string()),
            Item::Type(t) => Self::TypeAlias(t.ident.to_string()),
            Item::Mod(m) => Self::Mod(m.ident.to_string()),
            Item::Macro(m) => {
                if let Some(ref ident) = m.ident {
                    Self::Macro(ident.to_string())
                } else {
                    Self::Unknown(render_tokens(item))
                }
            }
            _ => Self::Unknown(render_tokens(item)),
        }
    }
}

/// Identity of an item inside an `impl` block, used for matching methods
/// and associated items across conflict sides.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum ImplItemIdentity {
    /// A method, keyed by name.
    Method(String),
    /// An associated const, keyed by name.
    Const(String),
    /// An associated type, keyed by name.
    Type(String),
    /// Fallback, keyed by rendered tokens.
    Unknown(String),
}

impl ImplItemIdentity {
    /// Extracts the identity from a `syn::ImplItem`.
    pub(super) fn from_impl_item(item: &ImplItem) -> Self {
        match item {
            ImplItem::Fn(m) => Self::Method(m.sig.ident.to_string()),
            ImplItem::Const(c) => Self::Const(c.ident.to_string()),
            ImplItem::Type(t) => Self::Type(t.ident.to_string()),
            _ => Self::Unknown(render_tokens(item)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn function_identity() {
        let item: Item = syn::parse_quote! { fn foo() {} };
        assert_eq!(
            ItemIdentity::from_item(&item),
            ItemIdentity::Function("foo".to_string())
        );
    }

    #[test]
    fn struct_identity() {
        let item: Item = syn::parse_quote! { struct Bar { x: i32 } };
        assert_eq!(
            ItemIdentity::from_item(&item),
            ItemIdentity::Struct("Bar".to_string())
        );
    }

    #[test]
    fn enum_identity() {
        let item: Item = syn::parse_quote! { enum Color { Red, Blue } };
        assert_eq!(
            ItemIdentity::from_item(&item),
            ItemIdentity::Enum("Color".to_string())
        );
    }

    #[test]
    fn trait_identity() {
        let item: Item = syn::parse_quote! { trait Drawable { fn draw(&self); } };
        assert_eq!(
            ItemIdentity::from_item(&item),
            ItemIdentity::Trait("Drawable".to_string())
        );
    }

    #[test]
    fn impl_identity_inherent() {
        let item: Item = syn::parse_quote! { impl Foo { fn bar(&self) {} } };
        assert_eq!(
            ItemIdentity::from_item(&item),
            ItemIdentity::Impl("Foo".to_string())
        );
    }

    #[test]
    fn impl_identity_trait() {
        let item: Item = syn::parse_quote! { impl Display for Foo { fn fmt(&self, f: &mut Formatter) -> Result { Ok(()) } } };
        let identity = ItemIdentity::from_item(&item);
        match identity {
            ItemIdentity::Impl(s) => assert!(s.contains("for Foo"), "expected trait impl: {s}"),
            other => panic!("expected Impl, got {other:?}"),
        }
    }

    #[test]
    fn use_identity() {
        let item: Item = syn::parse_quote! { use std::io::Read; };
        match ItemIdentity::from_item(&item) {
            ItemIdentity::Use(s) => assert!(s.contains("io")),
            other => panic!("expected Use, got {other:?}"),
        }
    }

    #[test]
    fn const_identity() {
        let item: Item = syn::parse_quote! { const MAX: usize = 100; };
        assert_eq!(
            ItemIdentity::from_item(&item),
            ItemIdentity::Const("MAX".to_string())
        );
    }

    #[test]
    fn impl_item_method_identity() {
        let item: ImplItem = syn::parse_quote! { fn hello(&self) {} };
        assert_eq!(
            ImplItemIdentity::from_impl_item(&item),
            ImplItemIdentity::Method("hello".to_string())
        );
    }

    #[test]
    fn impl_item_const_identity() {
        let item: ImplItem = syn::parse_quote! { const VALUE: i32 = 42; };
        assert_eq!(
            ImplItemIdentity::from_impl_item(&item),
            ImplItemIdentity::Const("VALUE".to_string())
        );
    }

    #[test]
    fn impl_item_type_identity() {
        let item: ImplItem = syn::parse_quote! { type Output = i32; };
        assert_eq!(
            ImplItemIdentity::from_impl_item(&item),
            ImplItemIdentity::Type("Output".to_string())
        );
    }
}
