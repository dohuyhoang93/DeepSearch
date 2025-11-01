use std::collections::HashMap;
use super::context::Context;
use crate::processes::live_search;
use crate::processes::scan;

/// Defines a Process as a function that takes a Context and returns a new Context (or an error).
pub type Process = fn(Context) -> anyhow::Result<Context>;

/// The Registry holds all available Processes and Workflows in the application.
pub struct Registry {
    processes: HashMap<String, Process>,
    workflows: HashMap<String, Vec<String>>,
}

impl Registry {
    pub fn new() -> Self {
        let mut registry = Self {
            processes: HashMap::new(),
            workflows: HashMap::new(),
        };
        registry.register_process("scan_directory_streaming", scan::scan_directory_streaming);
        registry.register_process("live_search_and_stream_results", live_search::live_search_and_stream_results);
        registry
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
