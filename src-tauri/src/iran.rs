//! Iranian destination ranges for the "Direct Iranian sites" switch.
//!
//! Sources: daily-aggregated Iranian IPv4/IPv6 prefixes (see
//! `scripts/refresh-iran-ranges.sh`). They are embedded in the binary with
//! `include_str!` and, when `direct_iran` is on, written to a `--routes`
//! file for the core — never to the command line, because ~2850 prefixes
//! (≈50KB) blow past Windows' 32KB command-line limit.

/// Aggregated Iranian IPv4 prefixes, one CIDR per line.
pub const IR_V4: &str = include_str!("../data/iran-v4.txt");

/// Aggregated Iranian IPv6 prefixes, one CIDR per line.
pub const IR_V6: &str = include_str!("../data/iran-v6.txt");

/// High-traffic Iranian domains + `private`, kept alongside the prefixes so
/// app/CDN front-ends that resolve outside the ranges still go direct.
pub const IR_DOMAINS_PRESET: &str = "private,digikala.com,aparat.com,filimo.com,\
    telewebion.com,varzesh3.com,snapp.ir,cafebazaar.ir,divar.ir,namava.ir,shad.ir";

/// File name (in the OS temp dir) of the generated `--routes` file.
pub const ROUTES_FILE_NAME: &str = "aethery-iran-routes.txt";

/// Build the core's `--routes` file content: the user's own direct entries
/// plus the full Iranian preset (domains first, then every prefix).
pub fn build_routes_content(user_direct: &str) -> String {
    let mut out = String::from("[direct]\n");
    let user_direct = user_direct.trim();
    if !user_direct.is_empty() {
        out.push_str(&user_direct.replace(',', "\n"));
        out.push('\n');
    }
    out.push_str(&IR_DOMAINS_PRESET.replace(',', "\n"));
    out.push('\n');
    out.push_str(IR_V4);
    if !IR_V4.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(IR_V6);
    if !IR_V6.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// Path of the generated routes file, shared by every connect.
pub fn routes_file_path() -> std::path::PathBuf {
    std::env::temp_dir().join(ROUTES_FILE_NAME)
}

/// Write the generated file; returns its path for `--routes`.
pub fn write_routes_file(user_direct: &str) -> Result<std::path::PathBuf, String> {
    let path = routes_file_path();
    std::fs::write(&path, build_routes_content(user_direct))
        .map_err(|e| format!("cannot write Iran routes file: {e}"))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn lists_are_substantial_and_valid_cidr() {
        let count = |s: &str| s.lines().filter(|l| !l.trim().is_empty()).count();
        let v4 = count(IR_V4);
        let v6 = count(IR_V6);
        assert!(v4 > 1500, "expected 1500+ Iranian v4 prefixes, got {v4}");
        assert!(v6 > 100, "expected 100+ Iranian v6 prefixes, got {v6}");
        for line in IR_V4.lines().chain(IR_V6.lines()) {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            assert!(
                std::net::IpAddr::from_str(line.split('/').next().unwrap_or("")).is_ok()
                    && line.contains('/'),
                "not a CIDR: {line}"
            );
        }
    }

    #[test]
    fn known_iranian_ranges_present() {
        for known in ["5.160.0.0/16", "94.182.0.0/16", "78.38.0.0/15"] {
            assert!(IR_V4.lines().any(|l| l.trim() == known), "missing {known}");
        }
    }

    #[test]
    fn routes_content_has_sections_and_user_entries() {
        let c = build_routes_content("banking.example, 10.9.0.0/16");
        assert!(c.starts_with("[direct]\n"));
        assert!(c.contains("banking.example"));
        assert!(c.contains("10.9.0.0/16"));
        assert!(c.contains("digikala.com"));
        assert!(c.contains("5.160.0.0/16"));
        assert!(!c.contains(','));
    }

    #[test]
    fn routes_file_roundtrips_to_disk() {
        let p = write_routes_file("").expect("write");
        let back = std::fs::read_to_string(&p).expect("read");
        assert!(back.contains("[direct]"));
        assert!(back.contains("5.160.0.0/16"));
    }
}
