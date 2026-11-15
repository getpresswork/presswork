#[derive(Debug, Default)]
pub struct Scheduler {
    running: bool,
    pending: bool,
}

impl Scheduler {
    pub fn request(&mut self) -> bool {
        if self.running {
            self.pending = true;
            false
        } else {
            self.running = true;
            true
        }
    }

    pub fn finish(&mut self) -> bool {
        if self.pending {
            self.pending = false;
            true
        } else {
            self.running = false;
            false
        }
    }

    #[must_use]
    pub fn is_running(&self) -> bool {
        self.running || self.pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_starts_a_compilation_when_none_runs() {
        let mut scheduler = Scheduler::default();

        assert!(scheduler.request());
        assert!(scheduler.is_running());
    }

    #[test]
    fn requests_during_a_compilation_wait_for_its_end() {
        let mut scheduler = Scheduler::default();
        scheduler.request();

        assert!(!scheduler.request());
        assert!(!scheduler.request());
        assert!(scheduler.finish());
        assert!(!scheduler.finish());
        assert!(!scheduler.is_running());
    }

    #[test]
    fn the_end_of_a_compilation_without_requests_stops() {
        let mut scheduler = Scheduler::default();
        scheduler.request();

        assert!(!scheduler.finish());
        assert!(!scheduler.is_running());
        assert!(scheduler.request());
    }
}
