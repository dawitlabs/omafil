use super::error::{AirDropError, ErrorCode};
use serde::Serialize;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    AwaitingConsent,
    Transferring,
    Verifying,
    Completed,
    Rejected,
    Cancelled,
    Failed,
    TimedOut,
}

impl Phase {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Rejected | Self::Cancelled | Self::Failed | Self::TimedOut
        )
    }
}

/// Local state only; the future transport must bind consent to an authenticated session.
pub struct Session {
    phase: Phase,
    bytes: u64,
    total: u64,
    deadline: Instant,
}

impl Session {
    pub fn new(total: u64, now: Instant, timeout: Duration) -> Result<Self, AirDropError> {
        if timeout.is_zero() {
            return Err(ErrorCode::InvalidMetadata.into());
        }
        let deadline = now.checked_add(timeout).ok_or(ErrorCode::InvalidMetadata)?;
        Ok(Self {
            phase: Phase::AwaitingConsent,
            bytes: 0,
            total,
            deadline,
        })
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }
    pub fn completed_bytes(&self) -> u64 {
        self.bytes
    }

    pub fn expire(&mut self, now: Instant) -> bool {
        if !self.phase.is_terminal() && now >= self.deadline {
            self.phase = Phase::TimedOut;
            return true;
        }
        false
    }

    fn active(&mut self, now: Instant) -> Result<(), AirDropError> {
        if self.expire(now) || self.phase == Phase::TimedOut {
            return Err(ErrorCode::TimedOut.into());
        }
        if self.phase.is_terminal() {
            return Err(ErrorCode::InvalidState.into());
        }
        Ok(())
    }

    pub fn consent(&mut self, accept: bool, now: Instant) -> Result<(), AirDropError> {
        self.active(now)?;
        if self.phase != Phase::AwaitingConsent {
            return Err(ErrorCode::InvalidState.into());
        }
        self.phase = if accept {
            Phase::Transferring
        } else {
            Phase::Rejected
        };
        Ok(())
    }

    pub fn advance(&mut self, count: u64, now: Instant) -> Result<(), AirDropError> {
        self.active(now)?;
        if self.phase != Phase::Transferring {
            return Err(ErrorCode::InvalidState.into());
        }
        let next = self.bytes.checked_add(count).filter(|n| *n <= self.total);
        let Some(next) = next else {
            self.phase = Phase::Failed;
            return Err(ErrorCode::InvalidMetadata.into());
        };
        self.bytes = next;
        Ok(())
    }

    pub fn verify(&mut self, now: Instant) -> Result<(), AirDropError> {
        self.active(now)?;
        if self.phase != Phase::Transferring {
            return Err(ErrorCode::InvalidState.into());
        }
        if self.bytes != self.total {
            self.phase = Phase::Failed;
            return Err(ErrorCode::Incomplete.into());
        }
        self.phase = Phase::Verifying;
        Ok(())
    }

    /// Call only after archive/integrity verification and successful publication.
    pub fn complete(&mut self, now: Instant) -> Result<(), AirDropError> {
        self.active(now)?;
        if self.phase != Phase::Verifying {
            return Err(ErrorCode::InvalidState.into());
        }
        self.phase = Phase::Completed;
        Ok(())
    }

    pub fn cancel(&mut self, now: Instant) -> Result<(), AirDropError> {
        if self.phase == Phase::Cancelled {
            return Ok(());
        }
        self.active(now)?;
        self.phase = Phase::Cancelled;
        Ok(())
    }

    pub fn fail(&mut self, now: Instant) -> Result<(), AirDropError> {
        self.active(now)?;
        self.phase = Phase::Failed;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn session() -> (Session, Instant) {
        let now = Instant::now();
        (Session::new(4, now, Duration::from_secs(10)).unwrap(), now)
    }

    #[test]
    fn consent_and_verification_are_required() {
        let (mut s, now) = session();
        assert!(s.advance(1, now).is_err());
        assert!(s.complete(now).is_err());
        s.consent(true, now).unwrap();
        s.advance(4, now).unwrap();
        assert!(s.complete(now).is_err());
        s.verify(now).unwrap();
        s.complete(now).unwrap();
        assert_eq!(s.phase(), Phase::Completed);
        assert!(s.cancel(now).is_err());
        assert!(!s.expire(now + Duration::from_secs(20)));
    }

    #[test]
    fn expired_offer_cannot_be_accepted() {
        let (mut s, now) = session();
        assert_eq!(
            s.consent(true, now + Duration::from_secs(10))
                .unwrap_err()
                .code,
            ErrorCode::TimedOut
        );
        assert_eq!(s.phase(), Phase::TimedOut);
    }

    #[test]
    fn rejection_and_cancellation_are_terminal() {
        let (mut s, now) = session();
        s.consent(false, now).unwrap();
        assert!(s.consent(true, now).is_err());
        for phase in 0..3 {
            let (mut s, now) = session();
            if phase > 0 {
                s.consent(true, now).unwrap();
            }
            if phase > 1 {
                s.advance(4, now).unwrap();
                s.verify(now).unwrap();
            }
            s.cancel(now).unwrap();
            s.cancel(now).unwrap();
            assert!(s.advance(0, now).is_err());
            assert!(s.complete(now).is_err());
        }
    }

    #[test]
    fn partial_and_excess_transfers_fail() {
        let (mut s, now) = session();
        s.consent(true, now).unwrap();
        s.advance(3, now).unwrap();
        assert_eq!(s.verify(now).unwrap_err().code, ErrorCode::Incomplete);
        let (mut s, now) = session();
        s.consent(true, now).unwrap();
        assert!(s.advance(u64::MAX, now).is_err());
        assert_eq!(s.phase(), Phase::Failed);
        assert_eq!(s.completed_bytes(), 0);
    }
}
