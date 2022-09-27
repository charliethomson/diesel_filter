use proc_macro2::Ident;

use crate::{filterable_type::FilterableType, opts::FilterOpts};

pub(crate) struct Filter {
    pub(crate) name: Ident,
    pub(crate) ty: FilterableType,
    pub(crate) opts: FilterOpts,
}
