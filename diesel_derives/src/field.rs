use proc_macro2::Span;
use quote;
use syn::spanned::Spanned;
use syn;

use meta::*;
use util::*;

pub struct Field {
    pub ty: syn::Type,
    pub name: FieldName,
    pub span: Span,
    pub sql_type: Option<syn::Type>,
    column_name_from_attribute: Option<syn::Ident>,
    flags: MetaItem,
}

impl Field {
    pub fn from_struct_field(field: &syn::Field, index: usize) -> Self {
        let column_name_from_attribute =
            MetaItem::with_name(&field.attrs, "column_name").map(|m| m.expect_ident_value());
        let name = match field.ident {
            Some(mut x) => {
                // https://github.com/rust-lang/rust/issues/47983#issuecomment-362817105
                let span = x.span();
                x.set_span(fix_span(span, Span::call_site()));
                FieldName::Named(x)
            }
            None => FieldName::Unnamed(syn::Index {
                index: index as u32,
                // https://github.com/rust-lang/rust/issues/47312
                span: Span::call_site(),
            }),
        };
        let sql_type = MetaItem::with_name(&field.attrs, "sql_type")
            .and_then(|m| m.ty_value().map_err(Diagnostic::emit).ok());
        let flags = MetaItem::with_name(&field.attrs, "diesel")
            .unwrap_or_else(|| MetaItem::empty("diesel"));
        let span = field.span();

        Self {
            ty: field.ty.clone(),
            column_name_from_attribute,
            name,
            sql_type,
            flags,
            span,
        }
    }

    pub fn column_name(&self) -> syn::Ident {
        self.column_name_from_attribute
            .unwrap_or_else(|| match self.name {
                FieldName::Named(x) => x,
                _ => {
                    self.span
                        .error(
                            "All fields of tuple structs must be annotated with `#[column_name]`",
                        )
                        .emit();
                    "unknown_column".into()
                }
            })
    }

    pub fn has_flag(&self, flag: &str) -> bool {
        self.flags.has_flag(flag)
    }
}

pub enum FieldName {
    Named(syn::Ident),
    Unnamed(syn::Index),
}

impl FieldName {
    pub fn assign(&self, expr: syn::Expr) -> syn::FieldValue {
        let span = self.span();
        // Parens are to work around https://github.com/rust-lang/rust/issues/47311
        let tokens = quote_spanned!(span=> #self: (#expr));
        parse_quote!(#tokens)
    }

    pub fn access(&self) -> quote::Tokens {
        let span = self.span();
        // Span of the dot is important due to
        // https://github.com/rust-lang/rust/issues/47312
        quote_spanned!(span=> .#self)
    }

    pub fn span(&self) -> Span {
        match *self {
            FieldName::Named(x) => x.span(),
            FieldName::Unnamed(ref x) => x.span,
        }
    }
}

impl quote::ToTokens for FieldName {
    fn to_tokens(&self, tokens: &mut quote::Tokens) {
        match *self {
            FieldName::Named(x) => x.to_tokens(tokens),
            FieldName::Unnamed(ref x) => x.to_tokens(tokens),
        }
    }
}
