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

    if !method.eq_ignore_ascii_case("GET") || has_dynamic_sort(query) {
        return EdgeScope::Dynamic;
    }

    if is_public_readonly_path(path) {
        EdgeScope::Public
    } else {
        EdgeScope::Dynamic
    }
}

fn is_public_readonly_path(path: &str) -> bool {
    if matches!(
        path,
        "/api/home/latest"
            | "/api/home/recommends"
            | "/api/search/hot"
            | "/api/search/suggest"
            | "/api/search/tags/hot"
            | "/api/site/categories/video"
            | "/api/site/sorts/video"
            | "/api/tags"
            | "/api/topics"
            | "/api/videos"
            | "/api/videos/long"
    ) {
        return true;
    }

    is_single_segment_under(path, "/api/ranks/")
        || is_single_segment_under(path, "/api/topics/")
        || is_single_segment_under(path, "/api/videos/category/")
        || is_single_segment_under(path, "/api/videos/sort/")
        || is_single_segment_under(path, "/api/videos/tag/")
}

fn is_single_segment_under(path: &str, prefix: &str) -> bool {
    path.strip_prefix(prefix)
        .is_some_and(|rest| !rest.is_empty() && !rest.contains('/'))
}

fn has_dynamic_sort(query: &str) -> bool {
    query.split('&').any(|pair| {
        matches!(
            pair.split_once('='),
            Some(("sort", value)) if value.eq_ignore_ascii_case("random") || value.eq_ignore_ascii_case("rand")
        )
    })
}
