#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteDecision {
    HandleLocally,
    ForwardToPpy,
    RedirectToPpy,
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

    // API paths need transparent proxying (osu! client expects exact responses)
    if host.ends_with("c.ppy.sh") || host.ends_with("c.localhost") {
        // Bancho server - always proxy
        return RouteDecision::ForwardToPpy;
    }

    if path.starts_with("/api/") || path.starts_with("/oauth/") || path.starts_with("/web/") {
        return RouteDecision::ForwardToPpy;
    }

    // Asset subdomains should proxy (avatars, beatmap assets, etc.)
    if host.ends_with("a.ppy.sh")
        || host.ends_with("a.localhost")
        || host.ends_with("b.ppy.sh")
        || host.ends_with("b.localhost")
        || host.ends_with("i.ppy.sh")
        || host.ends_with("i.localhost")
    {
        return RouteDecision::ForwardToPpy;
    }

    // Website paths - redirect browser to real osu.ppy.sh
    RouteDecision::RedirectToPpy
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
        assert_eq!(map_host_to_ppy("ce.ppy.sh"), "c.ppy.sh");
        assert_eq!(map_host_to_ppy("ce.osu.ppy.sh"), "c.ppy.sh");
    }

    #[test]
    fn test_c1_subdomain_forwards_to_bancho() {
        assert_eq!(map_host_to_ppy("c1.ppy.sh"), "c.ppy.sh");
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

    // Empty and edge path handling - website paths redirect to ppy.sh
    #[test]
    fn test_empty_path_redirects() {
        assert_eq!(
            route_request("osu.ppy.sh", ""),
            RouteDecision::RedirectToPpy
        );
    }

    #[test]
    fn test_root_path_redirects() {
        assert_eq!(
            route_request("osu.ppy.sh", "/"),
            RouteDecision::RedirectToPpy
        );
    }

    #[test]
    fn test_path_without_leading_slash() {
        // Paths without leading slash shouldn't match our patterns, redirect to website
        assert_eq!(
            route_request("osu.ppy.sh", "d/123456"),
            RouteDecision::RedirectToPpy
        );
        assert_eq!(
            route_request("osu.ppy.sh", "web/osu-search.php"),
            RouteDecision::RedirectToPpy
        );
    }

    // Security edge cases - malicious domains are handled safely
    // (proxy rejects non-localhost hosts before routing matters)
    #[test]
    fn test_malicious_domain_not_matching() {
        // osu.ppy.sh.evil.com should NOT be treated as osu.ppy.sh
        // /web/ paths forward (API pattern), /d/ paths redirect (not locally handled)
        assert_eq!(
            route_request("osu.ppy.sh.evil.com", "/web/osu-search.php"),
            RouteDecision::ForwardToPpy // matches /web/ API pattern
        );
        assert_eq!(
            route_request("osu.ppy.sh.evil.com", "/d/123456"),
            RouteDecision::RedirectToPpy // doesn't match any pattern
        );
    }

    #[test]
    fn test_subdomain_handling() {
        // Note: The current implementation uses ends_with("osu.ppy.sh"),
        // which means subdomains like "xxx.osu.ppy.sh" will also match.
        // This is acceptable because the hosts file controls what domains
        // are actually routed to this proxy.

        // Subdomains of osu.ppy.sh are handled locally for osu!direct paths
        assert_eq!(
            route_request("sub.osu.ppy.sh", "/web/osu-search.php"),
            RouteDecision::HandleLocally
        );

        // Non-osu!direct paths redirect to the website
        assert_eq!(
            route_request("sub.osu.ppy.sh", "/home"),
            RouteDecision::RedirectToPpy
        );
    }

    #[test]
    fn test_b_ppy_sh_evil_com_not_matching() {
        // b.ppy.sh.evil.com should NOT be treated as b.ppy.sh
        // Redirects because it doesn't match known asset domains
        assert_eq!(
            route_request("b.ppy.sh.evil.com", "/thumb/123.jpg"),
            RouteDecision::RedirectToPpy
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
        assert_eq!(map_to_ppy_url("c.ppy.sh:443", "/"), "https://c.ppy.sh/");
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
