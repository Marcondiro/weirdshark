use std::sync::{Arc, Condvar, Mutex};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;
use crate::capturer::WorkerCommand;

#[derive(PartialEq)]
enum WriteSchedulerStatus {
    Start,
    Stop,
}

pub(super) struct WriteScheduler {
    status: Arc<(Mutex<WriteSchedulerStatus>, Condvar)>,
}

impl WriteScheduler {
    pub(super) fn new(interval: Duration, sender: Sender<WorkerCommand>) -> Self {
        let status = Arc::new((
            Mutex::new(WriteSchedulerStatus::Stop),
            Condvar::new())
        );

        let t_status = status.clone();
        thread::spawn(move || {
            loop {
                let (mutex, condvar) = t_status.as_ref();
                let guard = mutex.lock().unwrap();
                let _ = condvar.wait_while(guard, |g| *g != WriteSchedulerStatus::Start).unwrap();

                thread::sleep(interval);
                match sender.send(WorkerCommand::WriteFile) {
                    Ok(_) => continue,
                    Err(_) => break,
                }
            }
        });

        Self { status }
    }

    pub(super) fn start(&self) {
        let (mutex, condvar) = self.status.as_ref();
        let mut guard = mutex.lock().unwrap();
        if *guard == WriteSchedulerStatus::Start {
            panic!("Weirdshark: The file generation scheduler is already running");
        }

        *guard = WriteSchedulerStatus::Start;
        condvar.notify_one();
    }

    pub(super) fn stop(&self) {
        let (mutex, condvar) = self.status.as_ref();
        let mut guard = mutex.lock().unwrap();
        if *guard == WriteSchedulerStatus::Stop {
            panic!("Weirdshark: The file generation scheduler is already stopped");
        }

        *guard = WriteSchedulerStatus::Stop;
        condvar.notify_one();
    }
}
