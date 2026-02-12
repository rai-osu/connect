#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteDecision {
    HandleLocally,
    ForwardToPpy,
}

pub fn route_request(host: &str, path: &str) -> RouteDecision {
    let host = host.split(':').next().unwrap_or(host);

    if host.ends_with("osu.ppy.sh") || host.ends_with("localhost") {
        if path.starts_with("/web/osu-search.php") || path.starts_with("/web/osu-search-set.php") {
            return RouteDecision::HandleLocally;
        }
        if path.starts_with("/d/") {
            return RouteDecision::HandleLocally;
        }
        if path.starts_with("/web/osu-getbeatmapinfo.php") {
            return RouteDecision::HandleLocally;
        }
    }

    if host.ends_with("b.ppy.sh") || host.ends_with("localhost") {
        if path.starts_with("/thumb/") || path.starts_with("/preview/") {
            return RouteDecision::HandleLocally;
        }
    }

    RouteDecision::ForwardToPpy
}

pub fn map_to_raimoe_url(original_path: &str, direct_base_url: &str) -> String {
    format!("{}{}", direct_base_url.trim_end_matches('/'), original_path)
}

pub fn map_to_ppy_url(host: &str, path: &str) -> String {
    let ppy_host = if host.starts_with("c.") || host.starts_with("c1.") {
        "c.ppy.sh"
    } else if host.starts_with("osu.") {
        "osu.ppy.sh"
    } else if host.starts_with("a.") {
        "a.ppy.sh"
    } else if host.starts_with("b.") {
        "b.ppy.sh"
    } else if host.starts_with("s.") {
        "s.ppy.sh"
    } else {
        "osu.ppy.sh"
    };

    format!("https://{}{}", ppy_host, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_osu_search() {
        assert_eq!(
            route_request("osu.ppy.sh", "/web/osu-search.php?q=test"),
            RouteDecision::HandleLocally
        );
    }

    #[test]
    fn test_route_download() {
        assert_eq!(
            route_request("osu.ppy.sh", "/d/123456"),
            RouteDecision::HandleLocally
        );
    }

    #[test]
    fn test_route_login_forwards() {
        assert_eq!(
            route_request("osu.ppy.sh", "/web/osu-submit-modular-selector.php"),
            RouteDecision::ForwardToPpy
        );
    }

    #[test]
    fn test_route_bancho_forwards() {
        assert_eq!(route_request("c.ppy.sh", "/"), RouteDecision::ForwardToPpy);
    }

    #[test]
    fn test_thumbnail_routes_locally() {
        assert_eq!(
            route_request("b.ppy.sh", "/thumb/123456l.jpg"),
            RouteDecision::HandleLocally
        );
    }
}
