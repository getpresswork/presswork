use std::collections::HashMap;
use std::hash::Hash;
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use axum::http::HeaderMap;
use sha2::{Digest, Sha256};

const WINDOW: Duration = Duration::from_mins(15);
const LOCK: Duration = Duration::from_mins(15);
const BUSY: Duration = Duration::from_mins(1);
const EMAIL_FAILURES: u32 = 5;
const ADDRESS_FAILURES: u32 = 20;
const PRUNE_SIZE: usize = 10_000;
const MAX_ENTRIES: usize = 100_000;
const FORWARDED_FOR: &str = "x-forwarded-for";

type EmailKey = [u8; 32];

#[derive(Debug, Default)]
struct Counter {
    failures: u32,
    first: Option<Instant>,
    locked_until: Option<Instant>,
    pending: u32,
}

impl Counter {
    fn forget_old(&mut self, now: Instant) {
        let lock_ended = self.locked_until.is_some_and(|until| until <= now);
        let window_ended = self.locked_until.is_none()
            && self
                .first
                .is_some_and(|first| now.saturating_duration_since(first) >= WINDOW);
        if lock_ended || window_ended {
            self.failures = 0;
            self.first = None;
            self.locked_until = None;
        }
    }

    fn wait(&self, now: Instant) -> Option<Duration> {
        self.locked_until
            .map(|until| until.saturating_duration_since(now))
            .filter(|wait| !wait.is_zero())
    }

    fn idle(&mut self, now: Instant) -> bool {
        self.forget_old(now);
        self.pending == 0 && self.failures == 0 && self.locked_until.is_none()
    }
}

#[derive(Debug)]
struct Counters<K> {
    limit: u32,
    next_prune: usize,
    entries: HashMap<K, Counter>,
}

impl<K: Eq + Hash + Copy> Counters<K> {
    fn new(limit: u32) -> Self {
        Self {
            limit,
            next_prune: PRUNE_SIZE,
            entries: HashMap::new(),
        }
    }

    fn check(&mut self, key: &K, now: Instant) -> Result<(), Duration> {
        let Some(counter) = self.entries.get_mut(key) else {
            return Ok(());
        };
        counter.forget_old(now);
        if let Some(wait) = counter.wait(now) {
            return Err(wait);
        }
        if counter.failures + counter.pending >= self.limit {
            return Err(BUSY);
        }
        Ok(())
    }

    fn start(&mut self, key: K, now: Instant) -> bool {
        if !self.entries.contains_key(&key) && !self.make_room(now) {
            return false;
        }
        self.entries.entry(key).or_default().pending += 1;
        true
    }

    fn make_room(&mut self, now: Instant) -> bool {
        if self.entries.len() >= self.next_prune {
            self.entries.retain(|_, counter| !counter.idle(now));
            self.next_prune = (self.entries.len() * 2).clamp(PRUNE_SIZE, MAX_ENTRIES);
        }
        self.entries.len() < MAX_ENTRIES
    }

    fn finish(&mut self, key: &K, failed: bool, now: Instant) -> bool {
        let Some(counter) = self.entries.get_mut(key) else {
            return false;
        };
        counter.pending = counter.pending.saturating_sub(1);
        if !failed {
            return false;
        }
        counter.forget_old(now);
        if counter.locked_until.is_some() {
            return false;
        }
        counter.first.get_or_insert(now);
        counter.failures += 1;
        if counter.failures < self.limit {
            return false;
        }
        counter.locked_until = Some(now + LOCK);
        true
    }

    fn forgive(&mut self, key: &K) {
        if let Some(counter) = self.entries.get_mut(key)
            && counter.locked_until.is_none()
        {
            counter.failures = 0;
            counter.first = None;
        }
    }
}

#[derive(Debug)]
pub(crate) struct SignInLimits {
    emails: Mutex<Counters<EmailKey>>,
    addresses: Mutex<Counters<IpAddr>>,
}

impl Default for SignInLimits {
    fn default() -> Self {
        Self {
            emails: Mutex::new(Counters::new(EMAIL_FAILURES)),
            addresses: Mutex::new(Counters::new(ADDRESS_FAILURES)),
        }
    }
}

impl SignInLimits {
    pub(crate) fn start(
        self: &Arc<Self>,
        email: &str,
        address: Option<IpAddr>,
        now: Instant,
    ) -> Result<Attempt, Duration> {
        let email = email_key(email);
        let address = address.map(network);
        let mut emails = lock(&self.emails);
        let mut addresses = lock(&self.addresses);
        let email_wait = emails.check(&email, now).err();
        let address_wait = address.and_then(|address| addresses.check(&address, now).err());
        if let Some(wait) = email_wait.max(address_wait) {
            return Err(wait);
        }
        let email = emails.start(email, now).then_some(email);
        let address = address.filter(|address| addresses.start(*address, now));
        Ok(Attempt {
            limits: Arc::clone(self),
            email,
            address,
            failed: None,
            succeeded: false,
        })
    }
}

#[derive(Debug)]
pub(crate) struct Attempt {
    limits: Arc<SignInLimits>,
    email: Option<EmailKey>,
    address: Option<IpAddr>,
    failed: Option<Instant>,
    succeeded: bool,
}

impl Attempt {
    pub(crate) fn failed(mut self, now: Instant) {
        self.failed = Some(now);
    }

    pub(crate) fn succeeded(mut self) {
        self.succeeded = true;
    }
}

impl Drop for Attempt {
    fn drop(&mut self) {
        let failed = self.failed.is_some();
        let now = self.failed.unwrap_or_else(Instant::now);
        if let Some(email) = &self.email {
            let mut emails = lock(&self.limits.emails);
            if emails.finish(email, failed, now) {
                tracing::warn!("Sign-ins for one email address are refused for 15 minutes");
            }
            if self.succeeded {
                emails.forgive(email);
            }
        }
        if let Some(address) = &self.address
            && lock(&self.limits.addresses).finish(address, failed, now)
        {
            tracing::warn!(%address, "Sign-ins from one address are refused for 15 minutes");
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn email_key(email: &str) -> EmailKey {
    Sha256::digest(email.trim().to_lowercase().as_bytes()).into()
}

fn network(address: IpAddr) -> IpAddr {
    match address.to_canonical() {
        IpAddr::V6(address) => IpAddr::V6(Ipv6Addr::from(u128::from(address) & (u128::MAX << 64))),
        address @ IpAddr::V4(_) => address,
    }
}

pub(crate) fn client_address(
    headers: &HeaderMap,
    connection: Option<SocketAddr>,
    trust_forwarded: bool,
) -> Option<IpAddr> {
    trust_forwarded
        .then(|| last_forwarded(headers))
        .flatten()
        .or_else(|| connection.map(|connection| connection.ip()))
}

fn last_forwarded(headers: &HeaderMap) -> Option<IpAddr> {
    let last = headers
        .get_all(FORWARDED_FOR)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .last()?
        .trim();
    last.parse()
        .ok()
        .or_else(|| last.parse::<SocketAddr>().ok().map(|address| address.ip()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ANN: &str = "ann@example.com";

    fn ip(text: &str) -> IpAddr {
        text.parse().unwrap()
    }

    fn fail(limits: &Arc<SignInLimits>, email: &str, from: Option<IpAddr>, now: Instant) {
        limits.start(email, from, now).unwrap().failed(now);
    }

    fn user(index: usize) -> String {
        format!("user{index}@example.com")
    }

    #[test]
    fn five_failures_refuse_an_email_address_for_fifteen_minutes() {
        let limits = Arc::new(SignInLimits::default());
        let now = Instant::now();
        for _ in 0..4 {
            fail(&limits, ANN, None, now);
        }
        assert!(limits.start(ANN, None, now).is_ok());

        fail(&limits, ANN, None, now);

        assert_eq!(limits.start(ANN, None, now).unwrap_err(), LOCK);
        let later = now + Duration::from_mins(1);
        assert_eq!(
            limits.start(ANN, None, later).unwrap_err(),
            Duration::from_mins(14)
        );
        assert!(limits.start(ANN, None, now + LOCK).is_ok());
        assert!(limits.start("bob@example.com", None, now).is_ok());
    }

    #[test]
    fn the_case_and_the_spaces_of_an_email_address_do_not_change_its_count() {
        let limits = Arc::new(SignInLimits::default());
        let now = Instant::now();
        for _ in 0..5 {
            fail(&limits, " Ann@Example.COM ", None, now);
        }

        assert!(limits.start(ANN, None, now).is_err());
    }

    #[test]
    fn failures_older_than_fifteen_minutes_do_not_count() {
        let limits = Arc::new(SignInLimits::default());
        let now = Instant::now();
        for _ in 0..4 {
            fail(&limits, ANN, None, now);
        }
        let later = now + WINDOW;
        for _ in 0..4 {
            fail(&limits, ANN, None, later);
        }

        assert!(limits.start(ANN, None, later).is_ok());
    }

    #[test]
    fn a_sign_in_with_the_right_password_forgives_the_failures() {
        let limits = Arc::new(SignInLimits::default());
        let now = Instant::now();
        for _ in 0..4 {
            fail(&limits, ANN, None, now);
        }
        limits.start(ANN, None, now).unwrap().succeeded();
        for _ in 0..4 {
            fail(&limits, ANN, None, now);
        }

        assert!(limits.start(ANN, None, now).is_ok());
    }

    #[test]
    fn twenty_failures_refuse_a_client_address() {
        let limits = Arc::new(SignInLimits::default());
        let now = Instant::now();
        let from = Some(ip("203.0.113.7"));
        for index in 0..20 {
            fail(&limits, &user(index), from, now);
        }

        assert_eq!(limits.start(ANN, from, now).unwrap_err(), LOCK);
        assert!(limits.start(ANN, Some(ip("203.0.113.8")), now).is_ok());
        assert!(limits.start(ANN, None, now).is_ok());
    }

    #[test]
    fn the_addresses_of_one_ipv6_network_share_a_count() {
        let limits = Arc::new(SignInLimits::default());
        let now = Instant::now();
        for index in 0..20 {
            let from = Some(ip(&format!("2001:db8::{index:x}")));
            fail(&limits, &user(index), from, now);
        }

        assert!(
            limits
                .start(ANN, Some(ip("2001:db8::ffff:1")), now)
                .is_err()
        );
        assert!(limits.start(ANN, Some(ip("2001:db8:0:1::1")), now).is_ok());
        assert_eq!(network(ip("::ffff:203.0.113.7")), ip("203.0.113.7"));
    }

    #[test]
    fn attempts_in_progress_count_against_the_limit() {
        let limits = Arc::new(SignInLimits::default());
        let now = Instant::now();
        let attempts: Vec<Attempt> = (0..5)
            .map(|_| limits.start(ANN, None, now).unwrap())
            .collect();

        assert_eq!(limits.start(ANN, None, now).unwrap_err(), BUSY);

        drop(attempts);
        assert!(limits.start(ANN, None, now).is_ok());
    }

    #[test]
    fn idle_counts_are_removed() {
        let limits = Arc::new(SignInLimits::default());
        let now = Instant::now();
        for index in 0..PRUNE_SIZE {
            fail(&limits, &user(index), None, now);
        }

        fail(&limits, ANN, None, now + WINDOW);

        assert_eq!(lock(&limits.emails).entries.len(), 1);
    }

    #[test]
    fn a_full_table_does_not_grow() {
        let limits = Arc::new(SignInLimits::default());
        let now = Instant::now();
        for index in 0..MAX_ENTRIES {
            fail(&limits, &user(index), None, now);
        }

        fail(&limits, ANN, None, now);

        assert_eq!(lock(&limits.emails).entries.len(), MAX_ENTRIES);
    }

    #[test]
    fn the_client_address_is_the_address_of_the_connection() {
        let connection = Some("192.0.2.4:50000".parse().unwrap());
        let mut headers = HeaderMap::new();
        headers.append(FORWARDED_FOR, "198.51.100.1, 203.0.113.9".parse().unwrap());

        assert_eq!(
            client_address(&headers, connection, false),
            Some(ip("192.0.2.4"))
        );
        assert_eq!(
            client_address(&headers, connection, true),
            Some(ip("203.0.113.9"))
        );
        headers.append(FORWARDED_FOR, "198.51.100.2".parse().unwrap());
        assert_eq!(
            client_address(&headers, connection, true),
            Some(ip("198.51.100.2"))
        );
        assert_eq!(
            client_address(&HeaderMap::new(), connection, true),
            Some(ip("192.0.2.4"))
        );
        assert_eq!(client_address(&HeaderMap::new(), None, false), None);
    }
}
