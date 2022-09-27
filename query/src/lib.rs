use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use std::default::Default;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    Data, DeriveInput, Fields, Meta, PathSegment, Token, Type,
};

use crate::{filter::Filter, filterable_type::FilterableType, opts::FilterOpts};

mod filter;
mod filterable_type;
mod opts;

enum FilterKind {
    Basic,
    Substr,
    Insensitive,
    SubstrInsensitive,
}

struct TableName {
    name: Ident,
}

impl Parse for TableName {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attr_name: Ident = input.parse()?;
        if attr_name != "table_name" {
            return Err(syn::Error::new(attr_name.span(), "Wrong attribute name"));
        }
        input.parse::<Token![=]>()?;
        let name: Ident = input.parse()?;
        Ok(TableName { name })
    }
}

struct SchemaPrefix {
    prefix: Punctuated<PathSegment, Token![::]>,
}

impl Parse for SchemaPrefix {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let prefix: Punctuated<PathSegment, Token![::]> =
            input.parse_terminated::<PathSegment, Token![::]>(PathSegment::parse)?;
        Ok(SchemaPrefix { prefix })
    }
}

#[proc_macro_derive(
    DieselFilter,
    attributes(filter, table_name, pagination, schema_prefix, ts)
)]
pub fn filter(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let table_name = match input
        .attrs
        .iter()
        .filter(|attr| attr.path.is_ident("diesel"))
        .filter_map(|a| a.parse_args::<TableName>().ok())
        .next()
    {
        Some(tn) => tn.name,
        None => panic!("please provide #[diesel(table_name = ...)] attribute"),
    };

    let pagination = input
        .attrs
        .iter()
        .filter(|m| m.path.is_ident("pagination"))
        .last()
        .is_some();

    let schema = match input
        .attrs
        .iter()
        .filter(|attr| attr.path.is_ident("schema_prefix"))
        .filter_map(|a| a.parse_args::<SchemaPrefix>().ok())
        .last()
    {
        Some(SchemaPrefix { prefix }) => quote! { #prefix::schema },
        None => quote! { crate::schema },
    };

    let struct_name = input.ident;
    let mut filters = vec![];

    if let Data::Struct(data) = input.data {
        if let Fields::Named(fields) = data.fields {
            for field in fields.named {
                match field.ident {
                    Some(name) => {
                        let field_type = field.ty;
                        for attr in field.attrs.into_iter() {
                            if !attr.path.is_ident("filter") {
                                continue;
                            }
                            let opts = match attr.parse_meta().unwrap() {
                                Meta::List(te) => {
                                    FilterOpts::from(te.nested.into_iter().collect::<Vec<_>>())
                                }
                                Meta::Path(_) => FilterOpts::default(),
                                _ => continue,
                            };

                            if let Type::Path(ty) = &field_type {
                                let ty = FilterableType::from(ty);
                                let name = name.clone();

                                filters.push(Filter { name, ty, opts });
                                continue;
                            }
                            panic!("this type is not supported");
                        }
                    }
                    None => continue,
                }
            }
        }
    }

    let filter_struct_ident = Ident::new(&format!("{}Filters", struct_name), struct_name.span());

    if filters.is_empty() {
        panic!("please annotate at least one field to filter with #[filter] on your struct");
    }

    let mut fields = vec![];
    let mut queries = vec![];
    let mut uses = vec![];
    let mut has_multiple = false;
    for filter in filters {
        let field = filter.name;
        let ty: Ident = filter.ty.into();
        let opts = filter.opts;

        let q = if opts.multiple {
            has_multiple = true;
            #[cfg(feature = "rocket")]
            fields.push(quote! {
                #[field(default = Option::None)]
                pub #field: Option<Vec<#ty>>,
            });
            #[cfg(not(feature = "rocket"))]
            fields.push(quote! {
                pub #field: Option<Vec<#ty>>,
            });
            match opts.kind {
                FilterKind::Basic => {
                    quote! { #table_name::#field.eq(any(filter)) }
                }
                FilterKind::Substr => {
                    quote! {
                        #table_name::#field.like(any(
                            filter.iter().map(|f| format!("%{}%", f)).collect::<Vec<_>>()
                        ))
                    }
                }
                FilterKind::Insensitive => {
                    quote! { #table_name::#field.ilike(any(filter)) }
                }
                FilterKind::SubstrInsensitive => {
                    quote! {
                        #table_name::#field.ilike(any(
                            filter.iter().map(|f| format!("%{}%", f)).collect::<Vec<_>>()
                        ))
                    }
                }
            }
        } else {
            fields.push(quote! {
                pub #field: Option<#ty>,
            });
            match opts.kind {
                FilterKind::Basic => {
                    quote! { #table_name::#field.eq(filter) }
                }
                FilterKind::Substr => {
                    quote! { #table_name::#field.like(format!("%{}%", filter)) }
                }
                FilterKind::Insensitive => {
                    quote! { #table_name::#field.ilike(filter) }
                }
                FilterKind::SubstrInsensitive => {
                    quote! { #table_name::#field.ilike(format!("%{}%", filter)) }
                }
            }
        };

        queries.push(quote! {
            if let Some(ref filter) = filters.#field {
                query = query.filter(#q);
            }
        });
    }

    if has_multiple {
        uses.push(quote! { use diesel::dsl::any; })
    }
    if pagination {
        fields.push(quote! {
            pub page: Option<i64>,
            pub per_page: Option<i64>,
        });
    }

    let mut filters_derives = vec![quote! { utoipa::ToSchema }];
    if cfg!(feature = "rocket") {
        filters_derives.push(quote! { FromForm });
    }
    if cfg!(any(feature = "actix", feature = "axum")) {
        filters_derives.push(quote! { serde::Deserialize });
    }

    filters_derives.push(quote! { Debug });

    let attrs = quote! {
            #[derive( #( #filters_derives, )* )]
    };

    #[cfg(feature = "rocket")]
    let filters_struct = quote! {
        #attrs
        pub struct #filter_struct_ident {
            #( #fields )*
        }
    };

    #[cfg(any(feature = "actix", feature = "axum"))]
    let filters_struct = quote! {
        #attrs
        pub struct #filter_struct_ident {
            #( #fields )*
        }
    };

    #[cfg(not(any(feature = "rocket", feature = "actix", feature = "axum")))]
    let filters_struct = quote! {
        #attrs
        pub struct #filter_struct_ident {
            #( #fields )*
        }
    };

    let expanded = match pagination {
        true => {
            quote! {
                #filters_struct

                impl #struct_name {
                    pub fn filtered(filters: &#filter_struct_ident, conn: &mut PgConnection) -> Result<(Vec<#struct_name>, i64), diesel::result::Error> {
                        Self::filter(filters)
                          .paginate(filters.page)
                          .per_page(filters.per_page)
                          .load_and_count::<#struct_name>(conn)
                    }

                    pub fn filter<'a>(filters: &'a #filter_struct_ident) -> #schema::#table_name::BoxedQuery<'a, diesel::pg::Pg> {
                        #( #uses )*
                        let mut query = #schema::#table_name::table.into_boxed();

                        #( #queries )*

                        query
                    }
                }
            }
        }
        false => {
            quote! {
                #filters_struct

                impl #struct_name {
                    pub fn filtered(filters: &#filter_struct_ident, conn: &mut PgConnection) -> Result<Vec<#struct_name>, diesel::result::Error> {
                        Self::filter(filters).load::<#struct_name>(conn)
                    }

                    pub fn filter<'a>(filters: &'a #filter_struct_ident) -> #schema::#table_name::BoxedQuery<'a, diesel::pg::Pg> {
                        #( #uses )*
                        let mut query = #schema::#table_name::table.into_boxed();

                        #( #queries )*

                        query
                    }
                }
            }
        }
    };
    TokenStream::from(expanded)
}
