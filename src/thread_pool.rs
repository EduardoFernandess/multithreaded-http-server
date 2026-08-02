use std::sync::{mpsc, Arc, Mutex};
use std::thread;

type Job = Box<dyn FnOnce() + Send + 'static>;

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

impl ThreadPool {
    /// # Panics
    ///
    /// Panics if `size` is zero.
    pub fn new(size: usize) -> Self {
        assert!(size > 0, "thread pool size must be > 0");

        let (sender, receiver) = mpsc::channel::<Job>();
        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);
        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        Self {
            workers,
            sender: Some(sender),
        }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        if let Some(sender) = &self.sender {
            sender.send(job).expect("all workers disconnected");
        }
    }

    pub fn size(&self) -> usize {
        self.workers.len()
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());
        for worker in &mut self.workers {
            if let Some(handle) = worker.thread.take() {
                let _ = handle.join();
            }
        }
    }
}

struct Worker {
    _id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Self {
        let thread = thread::spawn(move || loop {
            let job = {
                let guard = receiver.lock().expect("mutex poisoned");
                guard.recv()
            };

            match job {
                Ok(job) => job(),
                Err(_) => break,
            }
        });

        Self {
            _id: id,
            thread: Some(thread),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    #[test]
    fn executes_jobs_concurrently() {
        let pool = ThreadPool::new(4);
        let counter = Arc::new(AtomicUsize::new(0));

        for _ in 0..40 {
            let counter = Arc::clone(&counter);
            pool.execute(move || {
                counter.fetch_add(1, Ordering::SeqCst);
            });
        }

        drop(pool);
        assert_eq!(counter.load(Ordering::SeqCst), 40);
    }

    #[test]
    #[should_panic(expected = "thread pool size must be > 0")]
    fn rejects_zero_size() {
        let _ = ThreadPool::new(0);
    }

    #[test]
    fn reports_size() {
        let pool = ThreadPool::new(3);
        assert_eq!(pool.size(), 3);
        thread::sleep(Duration::from_millis(1));
    }
}
