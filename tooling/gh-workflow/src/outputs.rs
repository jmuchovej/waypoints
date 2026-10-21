use crate::{NamedJob, Step};

#[derive(Clone)]
pub struct StepOutput {
    pub name: &'static str,
    step_id: String,
}

impl StepOutput {
    /// Asserts the step's command names the output, which only holds when the
    /// command writes it inline. A command that publishes its own outputs — as
    /// `cargo xtask` does — needs [`new_trusted`].
    #[allow(dead_code, reason = "for steps that write their outputs inline")]
    pub fn new<T>(step: &Step<T>, name: &'static str) -> Self {
        let step_id = step
            .value
            .id
            .clone()
            .expect("Steps with outputs must have an ID");

        assert!(
            step.value
                .run
                .as_ref()
                .is_none_or(|run_command| run_command.contains(name)),
            "Step output with name '{name}' must occur at least once in run command with ID:{step_id}!"
        );

        Self { name, step_id }
    }

    pub fn new_trusted<T>(step: &Step<T>, name: &'static str) -> Self {
        let step_id = step
            .value
            .id
            .clone()
            .expect("Steps with outputs must have an ID");

        Self { name, step_id }
    }

    pub fn expr(&self) -> String {
        format!("steps.{}.outputs.{}", self.step_id, self.name)
    }

    pub fn as_job_output(&self, job: &NamedJob) -> JobOutput {
        JobOutput {
            job_name: job.name.clone(),
            name: self.name,
        }
    }

    pub fn as_output(&self) -> (String, String) {
        (self.name.to_owned(), self.to_string())
    }
}

impl serde::Serialize for StepOutput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl std::fmt::Display for StepOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "${{{{ {} }}}}", self.expr())
    }
}

#[derive(Clone)]
pub struct JobOutput {
    pub job_name: String,
    pub name: &'static str,
}

impl JobOutput {
    #[allow(dead_code, reason = "for outputs not reached through a PrepareJob")]
    pub fn new(job_name: String, name: &'static str) -> Self {
        Self { job_name, name }
    }

    pub fn expr(&self) -> String {
        format!("needs.{}.outputs.{}", self.job_name, self.name)
    }
}

impl serde::Serialize for JobOutput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl std::fmt::Display for JobOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "${{{{ {} }}}}", self.expr())
    }
}
