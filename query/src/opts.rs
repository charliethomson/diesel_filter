use syn::{NestedMeta, Path};

use crate::FilterKind;

pub(crate) struct FilterOpts {
    pub(crate) multiple: bool,
    pub(crate) kind: FilterKind,
}

impl Default for FilterOpts {
    fn default() -> Self {
        Self {
            multiple: false,
            kind: FilterKind::Basic,
        }
    }
}

impl From<Vec<NestedMeta>> for FilterOpts {
    fn from(m: Vec<NestedMeta>) -> Self {
        let meta = m
            .into_iter()
            .filter_map(|m| match m {
                NestedMeta::Meta(m) => Some(m.path().to_owned()),
                _ => None,
            })
            .collect::<Vec<_>>();

        let matches =
            |m: &Vec<Path>, tested: &[&str]| tested.iter().all(|t| m.iter().any(|m| m.is_ident(t)));

        let kind = if matches(&meta, &["substring", "insensitive"]) {
            FilterKind::SubstrInsensitive
        } else if matches(&meta, &["substring"]) {
            FilterKind::Substr
        } else if matches(&meta, &["insensitive"]) {
            FilterKind::Insensitive
        } else {
            FilterKind::Basic
        };

        Self {
            multiple: matches(&meta, &["multiple"]),
            kind,
        }
    }
}
