use std::sync::{Arc, Mutex, Condvar, atomic::{AtomicBool, Ordering}};

#[derive(PartialEq, Clone, Copy, Debug)] // Keep Debug for TaskState
pub enum TaskState {
    Running,
    Paused,
}

// The control block passed from the GUI
pub struct TaskController { // Removed #[derive(Debug)] from here
    state: Mutex<TaskState>,
    cvar: Condvar,
    is_cancelled: AtomicBool,
}

impl TaskController {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(TaskState::Running),
            cvar: Condvar::new(),
            is_cancelled: AtomicBool::new(false),
        })
    }

    pub fn pause(&self) {
        *self.state.lock().unwrap() = TaskState::Paused;
    }

    pub fn resume(&self) {
        *self.state.lock().unwrap() = TaskState::Running;
        self.cvar.notify_all();
    }

    pub fn cancel(&self) {
        self.is_cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.is_cancelled.load(Ordering::SeqCst)
    }

    // This is the key function the iterator will use to check for pause/resume signals
    pub fn check_and_wait_if_paused(&self) {
        let mut state_guard = self.state.lock().unwrap();
        while *state_guard == TaskState::Paused {
            state_guard = self.cvar.wait(state_guard).unwrap();
        }
    }
}