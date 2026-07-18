use inertia_core::{
    AssetContext, AssetError, AssetProvider, AssetRequest, AssetResponse, AssetSource, AssetTags,
    AssetVersion,
};
use std::sync::Arc;

/// One emitted frontend asset compiled into the application binary.
#[derive(Clone, Copy, Debug)]
pub struct EmbeddedAsset {
    /// Percent-encoded, public-path-relative lookup path.
    pub path: &'static str,
    /// Static bytes stored in the executable.
    pub bytes: &'static [u8],
    /// Valid response `Content-Type`.
    pub content_type: &'static str,
    /// Quoted compile-time SHA-256 entity tag.
    pub etag: &'static str,
    /// Whether the filename is conservatively recognized as content-addressed.
    pub immutable: bool,
    /// Optional content encoding for a directly addressed precompressed file.
    pub encoding: Option<&'static str>,
}

/// A complete production frontend compiled into the application binary.
#[derive(Clone, Copy, Debug)]
pub struct EmbeddedFrontend {
    /// URL prefix under which adapters mount the asset source.
    pub public_path: &'static str,
    /// Vite manifest entry used to generate tags.
    pub entry: &'static str,
    /// Deterministic deployment version.
    pub version: &'static str,
    /// Trusted deterministic stylesheet, preload, and script markup.
    pub tags: &'static str,
    /// Sorted static asset table.
    pub assets: &'static [EmbeddedAsset],
}

impl EmbeddedFrontend {
    /// Creates a compile-time frontend value.
    pub const fn new(
        public_path: &'static str,
        entry: &'static str,
        version: &'static str,
        tags: &'static str,
        assets: &'static [EmbeddedAsset],
    ) -> Self {
        Self {
            public_path,
            entry,
            version,
            tags,
            assets,
        }
    }

    /// Finds an exact asset without allocation or path decoding.
    pub fn find(&self, request_path: &str) -> Option<&'static EmbeddedAsset> {
        let path = request_path
            .split_once('?')
            .map_or(request_path, |(path, _)| path);
        let path = path.strip_prefix('/').unwrap_or(path);
        if path.is_empty()
            || path.starts_with('/')
            || path.contains('\\')
            || path
                .split('/')
                .any(|segment| segment.is_empty() || segment == "." || segment == "..")
        {
            return None;
        }
        self.assets
            .binary_search_by_key(&path, |asset| asset.path)
            .ok()
            .map(|index| &self.assets[index])
    }
}

impl AssetSource for &'static EmbeddedFrontend {
    fn get(&self, request: AssetRequest<'_>) -> Option<AssetResponse> {
        crate::request::respond(self, request)
    }
}

impl AssetProvider for &'static EmbeddedFrontend {
    fn version(&self) -> AssetVersion {
        AssetVersion::from(self.version)
    }

    fn render_tags(&self, _context: AssetContext<'_>) -> Result<AssetTags, AssetError> {
        Ok(AssetTags::new(self.tags.to_owned()))
    }

    fn source(&self) -> Option<Arc<dyn AssetSource>> {
        Some(Arc::new(*self))
    }

    fn public_path(&self) -> Option<&str> {
        Some(self.public_path)
    }
}
