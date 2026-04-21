use super::context::Context;
use super::registry::{Process, Registry};

/// The Engine is responsible for executing a Workflow.
pub struct Engine {
    registry: Registry,
}

impl Engine {
    pub fn new(registry: Registry) -> Self {
        Self { registry }
    }

    /// Executes a workflow sequentially.
    pub fn run_workflow(&self, workflow_name: &str, mut context: Context) -> anyhow::Result<Context> {
        let workflow = self.registry.get_workflow(workflow_name)
            .ok_or_else(|| anyhow::anyhow!("Workflow '{workflow_name}' not found"))?;

        for process_name in workflow {
            let process: &Process = self.registry.get_process(process_name)
                .ok_or_else(|| anyhow::anyhow!("Process '{process_name}' not found in registry"))?;
            
            // Execute the process and update the context
            context = process(context)?;
        }

        Ok(context)
    }
}
