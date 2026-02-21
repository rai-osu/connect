#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteDecision {
    HandleLocally,
    ForwardToUpstream,
    RedirectToUpstream,
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
        return RouteDecision::ForwardToUpstream;
    }

    if path.starts_with("/api/") || path.starts_with("/oauth/") || path.starts_with("/web/") {
        return RouteDecision::ForwardToUpstream;
    }

    // Asset subdomains should proxy (avatars, beatmap assets, etc.)
    if host.ends_with("a.ppy.sh")
        || host.ends_with("a.localhost")
        || host.ends_with("b.ppy.sh")
        || host.ends_with("b.localhost")
        || host.ends_with("i.ppy.sh")
        || host.ends_with("i.localhost")
    {
        return RouteDecision::ForwardToUpstream;
    }

    // Website paths - redirect browser to real osu.ppy.sh
    RouteDecision::RedirectToUpstream
}

pub fn map_to_raimoe_url(original_path: &str, direct_base_url: &str) -> String {
    format!("{}{}", direct_base_url.trim_end_matches('/'), original_path)
}

pub fn map_host_to_upstream(host: &str, upstream_server: &str) -> String {
    let host = host.split(':').next().unwrap_or(host);
    let subdomain = host.find('.').map(|pos| &host[..pos]).unwrap_or("osu");
    let subdomain = match subdomain {
        "c1" | "ce" => "c",
        other => other,
    };
    format!("{}.{}", subdomain, upstream_server)
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
            RouteDecision::ForwardToUpstream
        );
    }

    #[test]
    fn test_route_bancho_forwards() {
        assert_eq!(
            route_request("c.ppy.sh", "/"),
            RouteDecision::ForwardToUpstream
        );
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
        assert_eq!(map_host_to_upstream("ce.ppy.sh", "ppy.sh"), "c.ppy.sh");
        assert_eq!(map_host_to_upstream("ce.osu.ppy.sh", "ppy.sh"), "c.ppy.sh");
    }

    #[test]
    fn test_c1_subdomain_forwards_to_bancho() {
        assert_eq!(map_host_to_upstream("c1.ppy.sh", "ppy.sh"), "c.ppy.sh");
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
    fn test_map_host_to_upstream_strips_port() {
        assert_eq!(map_host_to_upstream("c.ppy.sh:443", "ppy.sh"), "c.ppy.sh");
        assert_eq!(map_host_to_upstream("a.ppy.sh:80", "ppy.sh"), "a.ppy.sh");
        assert_eq!(
            map_host_to_upstream("osu.ppy.sh:8080", "ppy.sh"),
            "osu.ppy.sh"
        );
    }

    // Empty and edge path handling - website paths redirect to ppy.sh
    #[test]
    fn test_empty_path_redirects() {
        assert_eq!(
            route_request("osu.ppy.sh", ""),
            RouteDecision::RedirectToUpstream
        );
    }

    #[test]
    fn test_root_path_redirects() {
        assert_eq!(
            route_request("osu.ppy.sh", "/"),
            RouteDecision::RedirectToUpstream
        );
    }

    #[test]
    fn test_path_without_leading_slash() {
        // Paths without leading slash shouldn't match our patterns, redirect to website
        assert_eq!(
            route_request("osu.ppy.sh", "d/123456"),
            RouteDecision::RedirectToUpstream
        );
        assert_eq!(
            route_request("osu.ppy.sh", "web/osu-search.php"),
            RouteDecision::RedirectToUpstream
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
            RouteDecision::ForwardToUpstream // matches /web/ API pattern
        );
        assert_eq!(
            route_request("osu.ppy.sh.evil.com", "/d/123456"),
            RouteDecision::RedirectToUpstream // doesn't match any pattern
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
            RouteDecision::RedirectToUpstream
        );
    }

    #[test]
    fn test_b_ppy_sh_evil_com_not_matching() {
        // b.ppy.sh.evil.com should NOT be treated as b.ppy.sh
        // Redirects because it doesn't match known asset domains
        assert_eq!(
            route_request("b.ppy.sh.evil.com", "/thumb/123.jpg"),
            RouteDecision::RedirectToUpstream
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

    // map_host_to_upstream tests
    #[test]
    fn test_map_host_to_upstream_known_subdomains() {
        assert_eq!(map_host_to_upstream("a.localhost", "ppy.sh"), "a.ppy.sh");
        assert_eq!(map_host_to_upstream("b.localhost", "ppy.sh"), "b.ppy.sh");
        assert_eq!(map_host_to_upstream("c.localhost", "ppy.sh"), "c.ppy.sh");
        assert_eq!(map_host_to_upstream("s.localhost", "ppy.sh"), "s.ppy.sh");
        assert_eq!(map_host_to_upstream("i.localhost", "ppy.sh"), "i.ppy.sh");
        assert_eq!(
            map_host_to_upstream("osu.localhost", "ppy.sh"),
            "osu.ppy.sh"
        );
    }

    #[test]
    fn test_map_host_to_upstream_bancho_variants() {
        assert_eq!(map_host_to_upstream("c1.localhost", "ppy.sh"), "c.ppy.sh");
        assert_eq!(map_host_to_upstream("ce.localhost", "ppy.sh"), "c.ppy.sh");
    }

    #[test]
    fn test_map_host_to_upstream_preserves_unknown() {
        assert_eq!(
            map_host_to_upstream("store.localhost", "ppy.sh"),
            "store.ppy.sh"
        );
        assert_eq!(
            map_host_to_upstream("api.localhost", "ppy.sh"),
            "api.ppy.sh"
        );
    }

    #[test]
    fn test_map_host_to_upstream_custom_server() {
        assert_eq!(
            map_host_to_upstream("c.localhost", "ripple.moe"),
            "c.ripple.moe"
        );
        assert_eq!(
            map_host_to_upstream("osu.localhost", "ripple.moe"),
            "osu.ripple.moe"
        );
        assert_eq!(
            map_host_to_upstream("a.localhost", "ripple.moe"),
            "a.ripple.moe"
        );
    }

    #[test]
    fn test_map_host_to_upstream_fallback() {
        assert_eq!(map_host_to_upstream("localhost", "ppy.sh"), "osu.ppy.sh");
    }
}
