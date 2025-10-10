use super::context::Context;
use super::registry::{Process, Registry};

/// Engine chịu trách nhiệm thực thi một Workflow.
pub struct Engine {
    registry: Registry,
}

impl Engine {
    pub fn new(registry: Registry) -> Self {
        Self { registry }
    }

    /// Thực thi một workflow tuần tự.
    pub fn run_workflow(&self, workflow_name: &str, mut context: Context) -> anyhow::Result<Context> {
        let workflow = self.registry.get_workflow(workflow_name)
            .ok_or_else(|| anyhow::anyhow!("Workflow '{}' not found", workflow_name))?;

        for process_name in workflow {
            let process: &Process = self.registry.get_process(process_name)
                .ok_or_else(|| anyhow::anyhow!("Process '{}' not found in registry", process_name))?;
            
            // Thực thi process và cập nhật context
            context = process(context)?;
        }

        Ok(context)
    }
}
