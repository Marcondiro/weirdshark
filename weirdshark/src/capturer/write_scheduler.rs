use std::sync::{Arc, Condvar, Mutex};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;
use crate::capturer::WorkerCommand;

pub(super) struct WriteScheduler {
    is_running: Arc<(Mutex<bool>, Condvar)>,
}

impl WriteScheduler {
    pub(super) fn new(interval: Duration, sender: Sender<WorkerCommand>) -> Self {
        let is_running = Arc::new((
            Mutex::new(false),
            Condvar::new())
        );

        let t_is_running = is_running.clone();
        thread::spawn(move || {
            loop {
                {
                    let (is_running_mutex, is_running_condvar) = t_is_running.as_ref();
                    let guard = is_running_mutex.lock().unwrap();
                    let _ = is_running_condvar.wait_while(guard, |g| !(*g)).unwrap();
                }

                thread::sleep(interval);
                match sender.send(WorkerCommand::WriteFile) {
                    Ok(_) => continue,
                    Err(_) => break,
                }
            }
        });

        Self { is_running }
    }

    pub(super) fn start(&self) {
        let (mutex, condvar) = self.is_running.as_ref();
        let mut guard = mutex.lock().unwrap();
        if *guard {
            panic!("Weirdshark: The file generation scheduler is already running");
        }

        *guard = true;
        condvar.notify_one();
    }

    pub(super) fn stop(&self) {
        let (mutex, condvar) = self.is_running.as_ref();
        let mut guard = mutex.lock().unwrap();
        if !(*guard) {
            panic!("Weirdshark: The file generation scheduler is already stopped");
        }

        *guard = false;
        condvar.notify_one();
    }
}
