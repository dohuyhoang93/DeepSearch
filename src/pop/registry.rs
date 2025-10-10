use std::collections::HashMap;
use super::context::Context;

/// Định nghĩa một Process là một hàm nhận vào Context và trả về một Context mới (hoặc lỗi).
pub type Process = fn(Context) -> anyhow::Result<Context>;

/// Registry chứa tất cả các Process và Workflow có sẵn trong ứng dụng.
pub struct Registry {
    processes: HashMap<String, Process>,
    workflows: HashMap<String, Vec<String>>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            processes: HashMap::new(),
            workflows: HashMap::new(),
        }
    }

    pub fn register_process(&mut self, name: &str, process: Process) {
        self.processes.insert(name.to_string(), process);
    }

    pub fn register_workflow(&mut self, name: &str, workflow: Vec<String>) {
        self.workflows.insert(name.to_string(), workflow);
    }

    pub fn get_process(&self, name: &str) -> Option<&Process> {
        self.processes.get(name)
    }

    pub fn get_workflow(&self, name: &str) -> Option<&Vec<String>> {
        self.workflows.get(name)
    }
}
