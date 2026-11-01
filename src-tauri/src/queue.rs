//! Очередь прогонов: идёт один, ждёт только самый свежий.

use std::sync::{Condvar, Mutex};
use vtracer::CancelToken;

#[derive(Default)]
struct Slots {
    latest: u64,
    /// Растёт при отмене всего: номера прогонов назначает фронтенд, поэтому
    /// сдвигать `latest` здесь нельзя.
    generation: u64,
    current: Option<CancelToken>,
}

#[derive(Default)]
pub struct Queue {
    slots: Mutex<Slots>,
    freed: Condvar,
}

impl Queue {
    /// Ждёт своей очереди. `None` значит, что запрос вытеснен более новым.
    pub fn enter(&self, run_id: u64) -> Option<CancelToken> {
        let mut slots = self.slots.lock().unwrap();
        slots.latest = slots.latest.max(run_id);
        if let Some(token) = &slots.current {
            token.cancel();
        }
        self.freed.notify_all();
        let generation = slots.generation;
        loop {
            if slots.latest != run_id || slots.generation != generation {
                return None;
            }
            if slots.current.is_none() {
                let token = CancelToken::new();
                slots.current = Some(token.clone());
                return Some(token);
            }
            slots = self.freed.wait(slots).unwrap();
        }
    }

    pub fn leave(&self) {
        self.slots.lock().unwrap().current = None;
        self.freed.notify_all();
    }

    /// Отменяет текущий прогон и всё, что ждёт очереди.
    pub fn cancel_all(&self) {
        let mut slots = self.slots.lock().unwrap();
        slots.generation += 1;
        if let Some(token) = &slots.current {
            token.cancel();
        }
        self.freed.notify_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn next_run_works_after_repeated_cancel() {
        let queue = Queue::default();
        assert!(queue.enter(1).is_some());
        queue.leave();
        queue.cancel_all();
        queue.cancel_all();
        assert!(queue.enter(2).is_some());
    }

    #[test]
    fn newer_run_evicts_waiting_one_and_cancels_current() {
        let queue = Arc::new(Queue::default());
        let first = queue.enter(1).unwrap();

        let q = queue.clone();
        let waiting = std::thread::spawn(move || q.enter(2).is_some());
        while !first.is_cancelled() {
            std::thread::yield_now();
        }

        let q = queue.clone();
        let newest = std::thread::spawn(move || {
            let token = q.enter(3);
            q.leave();
            token.is_some()
        });
        assert!(!waiting.join().unwrap(), "запрос 2 должен быть вытеснен");
        queue.leave();
        assert!(newest.join().unwrap());
    }
}
