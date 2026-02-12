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

    if (host.ends_with("b.ppy.sh") || host.ends_with("localhost"))
        && (path.starts_with("/thumb/") || path.starts_with("/preview/"))
    {
        return RouteDecision::HandleLocally;
    }

    RouteDecision::ForwardToPpy
}

pub fn map_to_raimoe_url(original_path: &str, direct_base_url: &str) -> String {
    format!("{}{}", direct_base_url.trim_end_matches('/'), original_path)
}

/// Maps a host from the local proxy to the corresponding ppy.sh subdomain.
///
/// This is the single source of truth for host mapping logic.
/// Handles all known osu! subdomains:
/// - `c.*`, `c1.*`, `ce.*` -> `c.ppy.sh` (Bancho/chat)
/// - `a.*` -> `a.ppy.sh` (Avatars)
/// - `b.*` -> `b.ppy.sh` (Beatmap assets)
/// - `s.*` -> `s.ppy.sh` (Spectator)
/// - `i.*` -> `i.ppy.sh` (Images)
/// - Default -> `osu.ppy.sh`
pub fn map_host_to_ppy(host: &str) -> &'static str {
    // Strip port if present
    let host = host.split(':').next().unwrap_or(host);

    if host.starts_with("c.") || host.starts_with("c1.") || host.starts_with("ce.") {
        "c.ppy.sh"
    } else if host.starts_with("a.") {
        "a.ppy.sh"
    } else if host.starts_with("b.") {
        "b.ppy.sh"
    } else if host.starts_with("s.") {
        "s.ppy.sh"
    } else if host.starts_with("i.") {
        "i.ppy.sh"
    } else {
        "osu.ppy.sh"
    }
}

pub fn map_to_ppy_url(host: &str, path: &str) -> String {
    let ppy_host = map_host_to_ppy(host);
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

    // Edge case tests for ce.* subdomain handling
    #[test]
    fn test_ce_subdomain_forwards_to_bancho() {
        // ce.* is used for Bancho connections in some regions
        assert_eq!(
            map_host_to_ppy("ce.ppy.sh"),
            "c.ppy.sh"
        );
        assert_eq!(
            map_host_to_ppy("ce.osu.ppy.sh"),
            "c.ppy.sh"
        );
    }

    #[test]
    fn test_c1_subdomain_forwards_to_bancho() {
        assert_eq!(
            map_host_to_ppy("c1.ppy.sh"),
            "c.ppy.sh"
        );
    }

    // Port stripping tests
    #[test]
    fn test_port_stripping_from_host() {
        // route_request should strip port from host
        assert_eq!(
            route_request("osu.ppy.sh:443", "/web/osu-search.php"),
            RouteDecision::HandleLocally
        );
        assert_eq!(
            route_request("osu.ppy.sh:80", "/d/123456"),
            RouteDecision::HandleLocally
        );
        assert_eq!(
            route_request("b.ppy.sh:443", "/thumb/123.jpg"),
            RouteDecision::HandleLocally
        );
    }

    #[test]
    fn test_map_host_to_ppy_strips_port() {
        assert_eq!(map_host_to_ppy("c.ppy.sh:443"), "c.ppy.sh");
        assert_eq!(map_host_to_ppy("a.ppy.sh:80"), "a.ppy.sh");
        assert_eq!(map_host_to_ppy("osu.ppy.sh:8080"), "osu.ppy.sh");
    }

    // Empty and edge path handling
    #[test]
    fn test_empty_path_forwards() {
        assert_eq!(
            route_request("osu.ppy.sh", ""),
            RouteDecision::ForwardToPpy
        );
    }

    #[test]
    fn test_root_path_forwards() {
        assert_eq!(
            route_request("osu.ppy.sh", "/"),
            RouteDecision::ForwardToPpy
        );
    }

    #[test]
    fn test_path_without_leading_slash() {
        // Paths without leading slash shouldn't match our patterns
        assert_eq!(
            route_request("osu.ppy.sh", "d/123456"),
            RouteDecision::ForwardToPpy
        );
        assert_eq!(
            route_request("osu.ppy.sh", "web/osu-search.php"),
            RouteDecision::ForwardToPpy
        );
    }

    // Security edge cases - prevent matching malicious domains
    #[test]
    fn test_malicious_domain_not_matching() {
        // osu.ppy.sh.evil.com should NOT be treated as osu.ppy.sh
        assert_eq!(
            route_request("osu.ppy.sh.evil.com", "/web/osu-search.php"),
            RouteDecision::ForwardToPpy
        );
        assert_eq!(
            route_request("osu.ppy.sh.evil.com", "/d/123456"),
            RouteDecision::ForwardToPpy
        );
    }

    #[test]
    fn test_similar_domain_not_matching() {
        // Domains that look similar but aren't osu.ppy.sh
        assert_eq!(
            route_request("fakeosu.ppy.sh", "/web/osu-search.php"),
            RouteDecision::ForwardToPpy
        );
        assert_eq!(
            route_request("nosu.ppy.sh", "/d/123456"),
            RouteDecision::ForwardToPpy
        );
    }

    #[test]
    fn test_b_ppy_sh_evil_com_not_matching() {
        // b.ppy.sh.evil.com should NOT be treated as b.ppy.sh
        assert_eq!(
            route_request("b.ppy.sh.evil.com", "/thumb/123.jpg"),
            RouteDecision::ForwardToPpy
        );
    }

    // Preview path tests
    #[test]
    fn test_preview_routes_locally() {
        assert_eq!(
            route_request("b.ppy.sh", "/preview/123456.mp3"),
            RouteDecision::HandleLocally
        );
    }

    // osu-search-set.php test
    #[test]
    fn test_osu_search_set_routes_locally() {
        assert_eq!(
            route_request("osu.ppy.sh", "/web/osu-search-set.php?b=123"),
            RouteDecision::HandleLocally
        );
    }

    // osu-getbeatmapinfo.php test
    #[test]
    fn test_osu_getbeatmapinfo_routes_locally() {
        assert_eq!(
            route_request("osu.ppy.sh", "/web/osu-getbeatmapinfo.php"),
            RouteDecision::HandleLocally
        );
    }

    // localhost handling tests
    #[test]
    fn test_localhost_search_routes_locally() {
        assert_eq!(
            route_request("localhost", "/web/osu-search.php"),
            RouteDecision::HandleLocally
        );
    }

    #[test]
    fn test_localhost_download_routes_locally() {
        assert_eq!(
            route_request("localhost", "/d/123456"),
            RouteDecision::HandleLocally
        );
    }

    #[test]
    fn test_localhost_thumb_routes_locally() {
        assert_eq!(
            route_request("localhost", "/thumb/123.jpg"),
            RouteDecision::HandleLocally
        );
    }

    // map_to_raimoe_url tests
    #[test]
    fn test_map_to_raimoe_url_basic() {
        assert_eq!(
            map_to_raimoe_url("/d/123456", "https://direct.rai.moe"),
            "https://direct.rai.moe/d/123456"
        );
    }

    #[test]
    fn test_map_to_raimoe_url_strips_trailing_slash() {
        assert_eq!(
            map_to_raimoe_url("/d/123456", "https://direct.rai.moe/"),
            "https://direct.rai.moe/d/123456"
        );
    }

    // map_to_ppy_url tests
    #[test]
    fn test_map_to_ppy_url_basic() {
        assert_eq!(
            map_to_ppy_url("osu.ppy.sh", "/web/test"),
            "https://osu.ppy.sh/web/test"
        );
    }

    #[test]
    fn test_map_to_ppy_url_with_port() {
        assert_eq!(
            map_to_ppy_url("c.ppy.sh:443", "/"),
            "https://c.ppy.sh/"
        );
    }

    // Additional map_host_to_ppy coverage
    #[test]
    fn test_map_host_to_ppy_all_subdomains() {
        assert_eq!(map_host_to_ppy("a.ppy.sh"), "a.ppy.sh");
        assert_eq!(map_host_to_ppy("b.ppy.sh"), "b.ppy.sh");
        assert_eq!(map_host_to_ppy("c.ppy.sh"), "c.ppy.sh");
        assert_eq!(map_host_to_ppy("s.ppy.sh"), "s.ppy.sh");
        assert_eq!(map_host_to_ppy("i.ppy.sh"), "i.ppy.sh");
        assert_eq!(map_host_to_ppy("unknown.ppy.sh"), "osu.ppy.sh");
    }
}
