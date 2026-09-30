//! Environment-driven runtime configuration that is not part of
//! `indexer::Config`: currently the indexer poll interval and dividend cache TTL.

use std::time::Duration;

use crate::indexer::{DEFAULT_DIVIDEND_CACHE_TTL, POLL_INTERVAL};

/// Env var holding the poll interval in whole seconds.
pub const POLL_INTERVAL_VAR: &str = "RWA_POLL_INTERVAL_SECS";
const MIN_POLL_SECS: u64 = 1;
const MAX_POLL_SECS: u64 = 3600;

/// Parse a raw `RWA_POLL_INTERVAL_SECS` value (`None` = unset, use default).
pub fn parse_poll_interval(raw: Option<&str>) -> Result<Duration, String> {
    let Some(raw) = raw else {
        return Ok(POLL_INTERVAL);
    };
    let secs: u64 = raw.trim().parse().map_err(|_| {
        format!("{POLL_INTERVAL_VAR} must be a whole number of seconds, got {raw:?}")
    })?;
    if !(MIN_POLL_SECS..=MAX_POLL_SECS).contains(&secs) {
        return Err(format!(
            "{POLL_INTERVAL_VAR} must be between {MIN_POLL_SECS} and {MAX_POLL_SECS}, got {secs}"
        ));
    }
    Ok(Duration::from_secs(secs))
}

/// Poll interval from the environment; falls back to the default when unset
/// or invalid (invalid values are rejected at startup by `main`).
pub fn poll_interval() -> Duration {
    parse_poll_interval(std::env::var(POLL_INTERVAL_VAR).ok().as_deref()).unwrap_or(POLL_INTERVAL)
}

/// Env var holding the dividend cache TTL in whole seconds.
pub const DIVIDEND_CACHE_TTL_VAR: &str = "RWA_DIVIDEND_CACHE_TTL_SECS";
const MIN_DIVIDEND_CACHE_SECS: u64 = 0;
const MAX_DIVIDEND_CACHE_SECS: u64 = 3600;

/// Parse a raw `RWA_DIVIDEND_CACHE_TTL_SECS` value.
pub fn parse_dividend_cache_ttl(raw: Option<&str>) -> Result<Duration, String> {
    let Some(raw) = raw else {
        return Ok(DEFAULT_DIVIDEND_CACHE_TTL);
    };
    let secs: u64 = raw.trim().parse().map_err(|_| {
        format!("{DIVIDEND_CACHE_TTL_VAR} must be a whole number of seconds, got {raw:?}")
    })?;
    if !(MIN_DIVIDEND_CACHE_SECS..=MAX_DIVIDEND_CACHE_SECS).contains(&secs) {
        return Err(format!(
            "{DIVIDEND_CACHE_TTL_VAR} must be between {MIN_DIVIDEND_CACHE_SECS} and {MAX_DIVIDEND_CACHE_SECS}, got {secs}"
        ));
    }
    Ok(Duration::from_secs(secs))
}

/// Dividend cache TTL from the environment; falls back to the default when
/// unset or invalid (invalid values are rejected at startup by `main`).
pub fn dividend_cache_ttl() -> Duration {
    parse_dividend_cache_ttl(std::env::var(DIVIDEND_CACHE_TTL_VAR).ok().as_deref())
        .unwrap_or(DEFAULT_DIVIDEND_CACHE_TTL)
}

const TESTNET_RPC: &str = "https://soroban-testnet.stellar.org";

/// Validate every configuration value using `get` as the variable lookup.
/// Returns one message per invalid variable, each naming the variable.
pub fn validate_with<F: Fn(&str) -> Option<String>>(get: F) -> Result<(), Vec<String>> {
    let mut errs = Vec::new();

    let rpc = get("RWA_RPC_URL").unwrap_or_else(|| TESTNET_RPC.to_string());
    match url::Url::parse(&rpc) {
        Ok(u) if u.scheme() == "http" || u.scheme() == "https" => {}
        Ok(_) => errs.push(format!("RWA_RPC_URL must use http or https, got {rpc:?}")),
        Err(e) => errs.push(format!("RWA_RPC_URL is not a valid URL ({e}): {rpc:?}")),
    }

    if rpc != TESTNET_RPC && (get("RWA_REGISTRY_ID").is_none() || get("RWA_DIVIDEND_ID").is_none())
    {
        errs.push("RWA_REGISTRY_ID and RWA_DIVIDEND_ID are required when RWA_RPC_URL is not Testnet".to_string());
    }

    for var in ["RWA_REGISTRY_ID", "RWA_DIVIDEND_ID"] {
        if let Some(v) = get(var) {
            if stellar_strkey::Contract::from_string(&v).is_err() {
                errs.push(format!("{var} is not a valid contract id (expected C... strkey): {v:?}"));
            }
        }
    }
    if let Some(v) = get("RWA_READ_SOURCE") {
        if stellar_strkey::ed25519::PublicKey::from_string(&v).is_err() {
            errs.push(format!("RWA_READ_SOURCE is not a valid account id (expected G... strkey): {v:?}"));
        }
    }

    if let Err(e) = parse_poll_interval(get(POLL_INTERVAL_VAR).as_deref()) {
        errs.push(e);
    }

    if let Err(e) = parse_dividend_cache_ttl(get(DIVIDEND_CACHE_TTL_VAR).as_deref()) {
        errs.push(e);
    }

    if let Some(p) = get("PORT") {
        if p.trim().parse::<u16>().map_or(true, |n| n == 0) {
            errs.push(format!("PORT must be an integer between 1 and 65535, got {p:?}"));
        }
    }

    if errs.is_empty() {
        Ok(())
    } else {
        Err(errs)
    }
}

/// Validate the process environment at startup. On failure logs every
/// offending variable and exits with status 1.
pub fn validate() {
    if let Err(errs) = validate_with(|k| std::env::var(k).ok()) {
        for e in &errs {
            tracing::error!("invalid configuration: {e}");
        }
        tracing::error!("config validation failed; exiting");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        move |k| pairs.iter().find(|(n, _)| *n == k).map(|(_, v)| v.to_string())
    }

    #[test]
    fn empty_env_is_valid() {
        assert!(validate_with(env(&[])).is_ok());
    }

    #[test]
    fn bad_values_name_the_variable() {
        let cases: &[(&str, &str)] = &[
            ("RWA_RPC_URL", "not a url"),
            ("RWA_REGISTRY_ID", "nope"),
            ("RWA_DIVIDEND_ID", "GAIQ"),
            ("RWA_READ_SOURCE", "CBX5"),
            ("RWA_POLL_INTERVAL_SECS", "0"),
            ("PORT", "99999"),
        ];
        for (var, val) in cases {
            let (var, val) = (var.to_string(), val.to_string());
            let errs = validate_with(|k| (k == var).then(|| val.clone())).unwrap_err();
            assert!(errs.iter().any(|e| e.contains(&var)), "{var}: {errs:?}");
        }
    }

    #[test]
    fn custom_rpc_requires_contract_ids() {
        let errs = validate_with(env(&[("RWA_RPC_URL", "https://rpc.example.com")])).unwrap_err();
        assert!(errs[0].contains("RWA_REGISTRY_ID"));
    }

    #[test]
    fn default_when_unset() {
        assert_eq!(parse_poll_interval(None).unwrap(), POLL_INTERVAL);
    }

    #[test]
    fn parses_valid_value() {
        assert_eq!(parse_poll_interval(Some("30")).unwrap(), Duration::from_secs(30));
    }

    #[test]
    fn rejects_invalid_values() {
        for bad in ["abc", "0", "-1", "99999", ""] {
            let err = parse_poll_interval(Some(bad)).unwrap_err();
            assert!(err.contains("RWA_POLL_INTERVAL_SECS"), "{err}");
        }
    }
}
