use proc_macro2::{Ident, Span};
use quote::ToTokens;
use syn::TypePath;

pub enum FilterableType {
    String,
    Uuid,
    Foreign(String),
}

impl From<&TypePath> for FilterableType {
    fn from(ty: &TypePath) -> Self {
        match ty.to_token_stream().to_string().replace(' ', "").as_str() {
            "String" => Self::String,
            "Uuid" => Self::Uuid,
            "uuid::Uuid" => Self::Uuid,
            "Option<String>" => Self::String,
            "Option<Uuid>" => Self::Uuid,
            "Option<uuid::Uuid>" => Self::Uuid,
            other => Self::Foreign(other.to_string()),
        }
    }
}

impl From<FilterableType> for Ident {
    fn from(val: FilterableType) -> Self {
        match val {
            FilterableType::String => Ident::new("String", Span::call_site()),
            FilterableType::Uuid => Ident::new("Uuid", Span::call_site()),
            FilterableType::Foreign(ty) => Ident::new(&ty, Span::call_site()),
        }
    }
}
