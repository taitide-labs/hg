#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeScope {
    Public,
    Dynamic,
    Playback,
}

impl EdgeScope {
    #[must_use]
    pub const fn endpoint_path(self) -> &'static str {
        match self {
            Self::Public => "/api/public/request",
            Self::Dynamic => "/api/dynamic/request",
            Self::Playback => "/api/playback/request",
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Dynamic => "dynamic",
            Self::Playback => "playback",
        }
    }
}

#[must_use]
pub fn resolve_edge_scope(method: &str, path_with_query: &str) -> EdgeScope {
    let (path, query) = match path_with_query.split_once('?') {
        Some((p, q)) => (p, q),
        None => (path_with_query, ""),
    };

    if method.eq_ignore_ascii_case("POST") && path.ends_with("/playback") {
        return EdgeScope::Playback;
    }

    if path.starts_with("/api/search")
        || path.contains("/comments")
        || has_dynamic_sort(query)
        || !method.eq_ignore_ascii_case("GET")
    {
        return EdgeScope::Dynamic;
    }

    EdgeScope::Public
}

fn has_dynamic_sort(query: &str) -> bool {
    query.split('&').any(|pair| {
        matches!(
            pair.split_once('='),
            Some(("sort", value)) if value.eq_ignore_ascii_case("random") || value.eq_ignore_ascii_case("rand")
        )
    })
}
