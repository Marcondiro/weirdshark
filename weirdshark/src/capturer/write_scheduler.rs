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

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::time::{Duration, Instant};
    use crate::capturer::WorkerCommand;
    use crate::capturer::write_scheduler::WriteScheduler;

    #[test]
    fn ws_sends_write_command_after_one_interval() {
        let interval = Duration::from_millis(200);
        let (sender, receiver) = mpsc::channel();

        let ws = WriteScheduler::new(interval, sender);
        ws.start();
        let start_time = Instant::now();

        let result = receiver.recv_timeout(interval.mul_f32(1.1)); // 10% tolerance
        let wakeup_time = Instant::now();
        assert!(result.is_ok(), "The file generation scheduler didn't work, timeout reached");
        match result.unwrap() {
            WorkerCommand::WriteFile => assert!(wakeup_time - start_time >= interval),
            c => panic!("File scheduler generated an unexpected command: {:?}", c),
        }
    }

    #[test]
    fn ws_sends_write_command_after_many_intervals() {
        let interval = Duration::from_millis(100);
        let (sender, receiver) = mpsc::channel();

        let ws = WriteScheduler::new(interval, sender);
        ws.start();

        for i in 0..20 {
            let start_time = Instant::now();
            let result = receiver.recv_timeout(interval.mul_f32(1.1)); // 10% tolerance
            let wakeup_time = Instant::now();
            assert!(result.is_ok(), "The file generation scheduler didn't work, timeout reached");
            match result.unwrap() {
                WorkerCommand::WriteFile => assert!(wakeup_time - start_time >= interval),
                c => panic!("File scheduler generated an unexpected command: {:?}", c),
            }
        }
    }

    #[test]
    fn ws_stop_stops() {
        let interval = Duration::from_millis(200);
        let (sender, receiver) = mpsc::channel();

        let ws = WriteScheduler::new(interval, sender);
        ws.start();
        ws.stop();
        // ignore one eventual write caused by the first start
        let _ = receiver.recv_timeout(interval.mul_f32(1.1)); // 10% tolerance

        let result = receiver.recv_timeout(interval * 20);
        assert!(result.is_err(), "The file generation scheduler pause didn't work, message received");
    }
}
