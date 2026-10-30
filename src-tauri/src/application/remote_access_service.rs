use std::collections::{HashMap, VecDeque};
use std::net::IpAddr;
use std::sync::{Arc, Mutex, MutexGuard};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::Rng;

use super::ports::{ConfigRepository, MonotonicClock, SessionRevoker};
use crate::domain::error::{DomainError, DomainResult};
use crate::domain::remote::{
    hash_token, matches_any, pin_matches, PIN_DIGITS, PIN_MAX_FAILURES, PIN_TTL_MS,
    RATE_MAX_ATTEMPTS, RATE_WINDOW_MS,
};

/// A freshly issued pairing PIN.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairingOffer {
    pub pin: String,
    pub ttl_ms: u64,
}

struct ActivePin {
    pin: String,
    expires_at_ms: u64,
    failures: u32,
}

struct State {
    enabled: bool,
    hashes: Vec<String>,
    pin: Option<ActivePin>,
    /// Attempt timestamps per peer inside the rate window.
    attempts: HashMap<IpAddr, VecDeque<u64>>,
}

/// Remote-access policy: the global toggle, PIN pairing and token hashes.
///
/// The toggle and hashes are cached in memory so the per-request auth check
/// never touches the disk; every change is written through to the config.
/// Only SHA-256 hashes of tokens are stored, and each pairing mints its own
/// token so devices can be paired side by side.
pub struct RemoteAccessService {
    repo: Arc<dyn ConfigRepository>,
    clock: Arc<dyn MonotonicClock>,
    revoker: Arc<dyn SessionRevoker>,
    state: Mutex<State>,
}

impl RemoteAccessService {
    pub fn new(
        repo: Arc<dyn ConfigRepository>,
        clock: Arc<dyn MonotonicClock>,
        revoker: Arc<dyn SessionRevoker>,
    ) -> Self {
        let config = repo.load();
        Self {
            repo,
            clock,
            revoker,
            state: Mutex::new(State {
                enabled: config.remote_enabled,
                hashes: config.remote_token_hashes,
                pin: None,
                attempts: HashMap::new(),
            }),
        }
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn is_enabled(&self) -> bool {
        self.lock().enabled
    }

    pub fn paired_devices(&self) -> usize {
        self.lock().hashes.len()
    }

    /// Turning it off closes every open remote session and burns the active
    /// PIN, but keeps the token hashes so paired devices work again once it
    /// is turned back on.
    pub fn set_enabled(&self, enabled: bool) -> DomainResult<()> {
        let mut state = self.lock();
        if state.enabled == enabled {
            return Ok(());
        }
        let mut config = self.repo.load();
        config.remote_enabled = enabled;
        self.repo.save(&config)?;
        state.enabled = enabled;
        if !enabled {
            state.pin = None;
            drop(state);
            self.revoker.revoke_all();
        }
        Ok(())
    }

    /// Replaces any active PIN with a new single-use one.
    pub fn start_pairing(&self) -> DomainResult<PairingOffer> {
        let mut state = self.lock();
        if !state.enabled {
            return Err(DomainError::RemoteDisabled);
        }
        let number = rand::rng().random_range(0..10u32.pow(PIN_DIGITS as u32));
        let pin = format!("{number:0width$}", width = PIN_DIGITS);
        state.pin = Some(ActivePin {
            pin: pin.clone(),
            expires_at_ms: self.clock.now_ms().saturating_add(PIN_TTL_MS),
            failures: 0,
        });
        Ok(PairingOffer {
            pin,
            ttl_ms: PIN_TTL_MS,
        })
    }

    pub fn cancel_pairing(&self) {
        self.lock().pin = None;
    }

    /// Exchanges a valid PIN for a new token (returned once, never stored).
    ///
    /// Attempts are rate limited per peer before the PIN is even looked at,
    /// so a flood is rejected regardless of correctness. A wrong guess counts
    /// against the PIN, which is burned after `PIN_MAX_FAILURES`.
    pub fn pair(&self, peer: IpAddr, pin: &str) -> DomainResult<String> {
        let mut state = self.lock();
        if !state.enabled {
            return Err(DomainError::RemoteDisabled);
        }
        let now = self.clock.now_ms();
        Self::check_rate(&mut state, peer, now)?;

        let Some(active) = state.pin.as_mut() else {
            return Err(DomainError::RemoteUnauthorized);
        };
        if now >= active.expires_at_ms {
            state.pin = None;
            return Err(DomainError::RemoteUnauthorized);
        }
        if !pin_matches(&active.pin, pin) {
            active.failures += 1;
            if active.failures >= PIN_MAX_FAILURES {
                state.pin = None;
            }
            return Err(DomainError::RemoteUnauthorized);
        }

        let mut bytes = [0u8; 32];
        rand::rng().fill(&mut bytes);
        let token = URL_SAFE_NO_PAD.encode(bytes);

        let mut config = self.repo.load();
        config.remote_token_hashes.push(hash_token(&token));
        self.repo.save(&config)?;
        state.hashes = config.remote_token_hashes;
        state.pin = None;
        Ok(token)
    }

    /// Records the attempt and rejects the peer once it exceeds the window quota.
    fn check_rate(state: &mut State, peer: IpAddr, now: u64) -> DomainResult<()> {
        state.attempts.retain(|_, hits| {
            while hits
                .front()
                .is_some_and(|t| now.saturating_sub(*t) >= RATE_WINDOW_MS)
            {
                hits.pop_front();
            }
            !hits.is_empty()
        });
        let hits = state.attempts.entry(peer).or_default();
        if hits.len() >= RATE_MAX_ATTEMPTS {
            return Err(DomainError::RemotePairingRateLimited);
        }
        hits.push_back(now);
        Ok(())
    }

    /// Constant-time check of a presented token against every stored hash.
    pub fn verify_token(&self, token: &str) -> bool {
        matches_any(&self.lock().hashes, token)
    }

    /// Forgets every paired device and closes every open remote session.
    pub fn revoke_all(&self) -> DomainResult<()> {
        let mut state = self.lock();
        let mut config = self.repo.load();
        config.remote_token_hashes.clear();
        self.repo.save(&config)?;
        state.hashes.clear();
        drop(state);
        self.revoker.revoke_all();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::{ConfigRepository, MonotonicClock, SessionRevoker};
    use crate::domain::config::AppConfig;
    use crate::domain::error::DomainError;
    use crate::domain::remote::hash_token;
    use std::net::IpAddr;
    use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    struct FakeClock(AtomicU64);

    impl FakeClock {
        fn advance(&self, ms: u64) {
            self.0.fetch_add(ms, Ordering::SeqCst);
        }
    }

    impl MonotonicClock for FakeClock {
        fn now_ms(&self) -> u64 {
            self.0.load(Ordering::SeqCst)
        }
    }

    #[derive(Default)]
    struct FakeRevoker(AtomicUsize);

    impl SessionRevoker for FakeRevoker {
        fn revoke_all(&self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    struct MemRepo(Mutex<AppConfig>);

    impl ConfigRepository for MemRepo {
        fn load(&self) -> AppConfig {
            self.0.lock().unwrap().clone()
        }

        fn save(&self, config: &AppConfig) -> crate::domain::error::DomainResult<()> {
            *self.0.lock().unwrap() = config.clone();
            Ok(())
        }
    }

    struct Fixture {
        svc: RemoteAccessService,
        clock: Arc<FakeClock>,
        revoker: Arc<FakeRevoker>,
        repo: Arc<MemRepo>,
    }

    fn fixture_with(config: AppConfig) -> Fixture {
        let clock = Arc::new(FakeClock(AtomicU64::new(1_000)));
        let revoker = Arc::new(FakeRevoker::default());
        let repo = Arc::new(MemRepo(Mutex::new(config)));
        let svc = RemoteAccessService::new(repo.clone(), clock.clone(), revoker.clone());
        Fixture {
            svc,
            clock,
            revoker,
            repo,
        }
    }

    fn enabled() -> Fixture {
        fixture_with(AppConfig {
            remote_enabled: true,
            ..AppConfig::default()
        })
    }

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    const LAN: &str = "192.168.1.50";

    #[test]
    fn pin_is_6_digits() {
        let f = enabled();
        for _ in 0..50 {
            let offer = f.svc.start_pairing().unwrap();
            assert_eq!(offer.pin.len(), 6);
            assert!(offer.pin.chars().all(|c| c.is_ascii_digit()));
        }
    }

    #[test]
    fn pin_expires_after_120s() {
        let f = enabled();
        let offer = f.svc.start_pairing().unwrap();
        assert_eq!(offer.ttl_ms, 120_000);
        f.clock.advance(120_000);
        assert!(matches!(
            f.svc.pair(ip(LAN), &offer.pin),
            Err(DomainError::RemoteUnauthorized)
        ));

        let offer = f.svc.start_pairing().unwrap();
        f.clock.advance(119_999);
        assert!(f.svc.pair(ip(LAN), &offer.pin).is_ok());
    }

    #[test]
    fn pin_single_use() {
        let f = enabled();
        let offer = f.svc.start_pairing().unwrap();
        assert!(f.svc.pair(ip(LAN), &offer.pin).is_ok());
        assert!(matches!(
            f.svc.pair(ip(LAN), &offer.pin),
            Err(DomainError::RemoteUnauthorized)
        ));
    }

    #[test]
    fn wrong_pin_issues_nothing() {
        let f = enabled();
        let offer = f.svc.start_pairing().unwrap();
        let wrong = if offer.pin == "000000" {
            "000001"
        } else {
            "000000"
        };
        assert!(matches!(
            f.svc.pair(ip(LAN), wrong),
            Err(DomainError::RemoteUnauthorized)
        ));
        assert_eq!(f.svc.paired_devices(), 0);
        assert!(f.svc.pair(ip(LAN), &offer.pin).is_ok());
    }

    #[test]
    fn five_failures_burn_pin() {
        let f = enabled();
        let offer = f.svc.start_pairing().unwrap();
        let wrong = if offer.pin == "000000" {
            "000001"
        } else {
            "000000"
        };
        for _ in 0..5 {
            assert!(f.svc.pair(ip(LAN), wrong).is_err());
        }
        assert!(matches!(
            f.svc.pair(ip(LAN), &offer.pin),
            Err(DomainError::RemoteUnauthorized)
        ));
        assert_eq!(f.svc.paired_devices(), 0);
    }

    #[test]
    fn rate_limit_10_per_min_per_ip_returns_429_code() {
        let f = enabled();
        for _ in 0..10 {
            let _ = f.svc.pair(ip(LAN), "not-a-pin");
        }
        let offer = f.svc.start_pairing().unwrap();
        assert!(matches!(
            f.svc.pair(ip(LAN), &offer.pin),
            Err(DomainError::RemotePairingRateLimited)
        ));

        // Another address is unaffected.
        let other = f.svc.start_pairing().unwrap();
        assert!(f.svc.pair(ip("192.168.1.51"), &other.pin).is_ok());

        // The window slides.
        f.clock.advance(60_001);
        let again = f.svc.start_pairing().unwrap();
        assert!(f.svc.pair(ip(LAN), &again.pin).is_ok());
    }

    #[test]
    fn pairing_mints_distinct_tokens_and_stores_sha256_only() {
        let f = enabled();
        let first = f.svc.start_pairing().unwrap();
        let t1 = f.svc.pair(ip(LAN), &first.pin).unwrap();
        let second = f.svc.start_pairing().unwrap();
        let t2 = f.svc.pair(ip("192.168.1.51"), &second.pin).unwrap();

        assert_ne!(t1, t2);
        assert_eq!(t1.len(), 43, "32 random bytes, base64url without padding");
        assert_eq!(
            f.repo.load().remote_token_hashes,
            vec![hash_token(&t1), hash_token(&t2)]
        );
        assert!(f.svc.verify_token(&t1));
        assert!(f.svc.verify_token(&t2));
        assert!(!f.svc.verify_token("nope"));
        assert_eq!(f.svc.paired_devices(), 2);
    }

    #[test]
    fn plaintext_never_persisted() {
        let f = enabled();
        let offer = f.svc.start_pairing().unwrap();
        let token = f.svc.pair(ip(LAN), &offer.pin).unwrap();
        let stored = serde_json::to_string(&f.repo.load()).unwrap();
        assert!(!stored.contains(&token));
        assert!(!stored.contains(&offer.pin));
    }

    #[test]
    fn revoke_clears_all_hashes_and_bumps_epoch() {
        let f = enabled();
        let offer = f.svc.start_pairing().unwrap();
        let token = f.svc.pair(ip(LAN), &offer.pin).unwrap();

        f.svc.revoke_all().unwrap();

        assert!(f.repo.load().remote_token_hashes.is_empty());
        assert!(!f.svc.verify_token(&token));
        assert_eq!(f.revoker.0.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn disabled_rejects_pairing_and_pin_creation() {
        let f = fixture_with(AppConfig::default());
        assert!(!f.svc.is_enabled());
        assert!(matches!(
            f.svc.start_pairing(),
            Err(DomainError::RemoteDisabled)
        ));
        assert!(matches!(
            f.svc.pair(ip(LAN), "123456"),
            Err(DomainError::RemoteDisabled)
        ));
    }

    #[test]
    fn disable_revokes_sessions_keeps_hashes_and_reenable_restores_tokens() {
        let f = enabled();
        let offer = f.svc.start_pairing().unwrap();
        let token = f.svc.pair(ip(LAN), &offer.pin).unwrap();

        f.svc.set_enabled(false).unwrap();
        assert!(!f.svc.is_enabled());
        assert!(!f.repo.load().remote_enabled);
        assert_eq!(f.revoker.0.load(Ordering::SeqCst), 1);
        assert_eq!(f.repo.load().remote_token_hashes.len(), 1);

        f.svc.set_enabled(true).unwrap();
        assert!(f.svc.is_enabled());
        assert!(f.svc.verify_token(&token));
        assert_eq!(
            f.revoker.0.load(Ordering::SeqCst),
            1,
            "enabling revokes nothing"
        );
    }

    #[test]
    fn disable_burns_the_active_pin() {
        let f = enabled();
        let offer = f.svc.start_pairing().unwrap();
        f.svc.set_enabled(false).unwrap();
        f.svc.set_enabled(true).unwrap();
        assert!(matches!(
            f.svc.pair(ip(LAN), &offer.pin),
            Err(DomainError::RemoteUnauthorized)
        ));
    }

    #[test]
    fn state_is_loaded_from_config() {
        let hash = hash_token("persisted");
        let f = fixture_with(AppConfig {
            remote_enabled: true,
            remote_token_hashes: vec![hash],
            ..AppConfig::default()
        });
        assert!(f.svc.is_enabled());
        assert!(f.svc.verify_token("persisted"));
        assert_eq!(f.svc.paired_devices(), 1);
    }
}
