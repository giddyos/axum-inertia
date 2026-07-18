use std::collections::BTreeSet;
use syn::{
    Expr, Ident, LitBool, LitInt, LitStr, Result, Token,
    parse::{Parse, ParseStream},
};

pub(crate) const DEFAULT_MAX_MANIFEST_SIZE: u64 = 16 * 1024 * 1024;
pub(crate) const DEFAULT_MAX_FILES: u64 = 100_000;
pub(crate) const DEFAULT_MAX_ASSET_SIZE: u64 = 512 * 1024 * 1024;
pub(crate) const DEFAULT_MAX_TOTAL_SIZE: u64 = 2 * 1024 * 1024 * 1024;

pub(crate) struct EmbedInput {
    pub(crate) root: LitStr,
    pub(crate) manifest: Option<LitStr>,
    pub(crate) entry: LitStr,
    pub(crate) public_path: LitStr,
    pub(crate) include_source_maps: bool,
    pub(crate) include_hidden: bool,
    pub(crate) max_manifest_size: u64,
    pub(crate) max_files: u64,
    pub(crate) max_asset_size: u64,
    pub(crate) max_total_size: u64,
}

impl Parse for EmbedInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut seen = BTreeSet::new();
        let mut root = None;
        let mut manifest = None;
        let mut entry = None;
        let mut public_path = None;
        let mut include_source_maps = false;
        let mut include_hidden = false;
        let mut cache = None;
        let mut max_manifest_size = DEFAULT_MAX_MANIFEST_SIZE;
        let mut max_files = DEFAULT_MAX_FILES;
        let mut max_asset_size = DEFAULT_MAX_ASSET_SIZE;
        let mut max_total_size = DEFAULT_MAX_TOTAL_SIZE;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            let name = key.to_string();
            if !seen.insert(name.clone()) {
                return Err(syn::Error::new(
                    key.span(),
                    format!("duplicate embed_frontend! setting `{name}`"),
                ));
            }
            input.parse::<Token![:]>()?;
            match name.as_str() {
                "root" => root = Some(input.parse::<LitStr>()?),
                "manifest" => manifest = Some(input.parse::<LitStr>()?),
                "entry" => entry = Some(input.parse::<LitStr>()?),
                "public_path" => public_path = Some(input.parse::<LitStr>()?),
                "include_source_maps" => {
                    include_source_maps = input.parse::<LitBool>()?.value;
                }
                "include_hidden" => {
                    include_hidden = input.parse::<LitBool>()?.value;
                }
                "cache" => {
                    let value = input.parse::<LitStr>()?;
                    if value.value() != "auto" {
                        return Err(syn::Error::new(
                            value.span(),
                            "embed_frontend! `cache` currently supports only \"auto\"",
                        ));
                    }
                    cache = Some(value);
                }
                "max_manifest_size" => {
                    max_manifest_size = limited_integer(
                        input.parse::<LitInt>()?,
                        "max_manifest_size",
                        DEFAULT_MAX_MANIFEST_SIZE,
                    )?;
                }
                "max_files" => {
                    max_files =
                        limited_integer(input.parse::<LitInt>()?, "max_files", DEFAULT_MAX_FILES)?;
                }
                "max_asset_size" => {
                    max_asset_size = limited_integer(
                        input.parse::<LitInt>()?,
                        "max_asset_size",
                        DEFAULT_MAX_ASSET_SIZE,
                    )?;
                }
                "max_total_size" => {
                    max_total_size = input.parse::<LitInt>()?.base10_parse()?;
                }
                _ => {
                    let _: Expr = input.parse()?;
                    return Err(syn::Error::new(
                        key.span(),
                        format!("unknown embed_frontend! setting `{name}`"),
                    ));
                }
            }
            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }

        let span = proc_macro2::Span::call_site();
        let _ = cache;
        Ok(Self {
            root: root
                .ok_or_else(|| syn::Error::new(span, "embed_frontend! requires `root: \"...\"`"))?,
            manifest,
            entry: entry.ok_or_else(|| {
                syn::Error::new(span, "embed_frontend! requires `entry: \"...\"`")
            })?,
            public_path: public_path.unwrap_or_else(|| LitStr::new("/build", span)),
            include_source_maps,
            include_hidden,
            max_manifest_size,
            max_files,
            max_asset_size,
            max_total_size,
        })
    }
}

fn limited_integer(value: LitInt, name: &str, maximum: u64) -> Result<u64> {
    let parsed = value.base10_parse::<u64>()?;
    if parsed == 0 || parsed > maximum {
        return Err(syn::Error::new(
            value.span(),
            format!(
                "embed_frontend! `{name}` must be between 1 and {maximum}; only `max_total_size` may be 0 to disable its limit"
            ),
        ));
    }
    Ok(parsed)
}
