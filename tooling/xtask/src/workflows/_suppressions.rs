//! Places step-scoped `zizmor` suppressions in rendered workflow text.
//!
//! The YAML serializer cannot emit comments, so a step carries its
//! suppressions as typed data (`Step::zizmor_ignore`) and this pass appends
//! them to the step's `id:` line after serialization.

use anyhow::{Result, bail};
use gh_workflow::Workflow;

/// What one step asks to have suppressed, and where to find it.
struct Directive {
    job: String,
    step_id: String,
    rules: Vec<String>,
    reasons: Vec<String>,
}

impl Directive {
    fn comment(&self) -> String {
        format!(
            " # zizmor: ignore[{}] {}",
            self.rules.join(","),
            self.reasons.join("; ")
        )
    }
}

fn directives(workflow: &Workflow) -> Result<Vec<Directive>> {
    let mut found = Vec::new();
    let Some(jobs) = workflow.jobs.as_ref() else {
        return Ok(found);
    };
    for (job, value) in jobs.iter() {
        for step in value.steps.iter().flatten() {
            if step.lint_ignores.is_empty() {
                continue;
            }
            let Some(step_id) = step.id.clone() else {
                let what = step
                    .name
                    .as_deref()
                    .or(step.uses.as_deref())
                    .or(step.run.as_deref())
                    .unwrap_or("<unnamed>");
                bail!(
                    "job `{job}`: step `{what}` has a zizmor suppression but no `id` to carry it"
                );
            };
            let mut rules: Vec<String> = Vec::new();
            let mut reasons: Vec<String> = Vec::new();
            for ignore in &step.lint_ignores {
                if !rules.contains(&ignore.rule) {
                    rules.push(ignore.rule.clone());
                }
                if !reasons.contains(&ignore.reason) {
                    reasons.push(ignore.reason.clone());
                }
            }
            found.push(Directive {
                job: job.clone(),
                step_id,
                rules,
                reasons,
            });
        }
    }
    Ok(found)
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// Whether `line` is the `id:` line for `step_id`, however the serializer
/// chose to quote it and whether or not it opens the step's list entry.
fn declares_id(line: &str, step_id: &str) -> bool {
    let trimmed = line.trim_start();
    let key = trimmed.strip_prefix("- ").unwrap_or(trimmed);
    [
        format!("id: {step_id}"),
        format!("id: \"{step_id}\""),
        format!("id: '{step_id}'"),
    ]
    .iter()
    .any(|candidate| key == candidate)
}

/// Appends each step's suppression to its `id:` line in `content`, the
/// rendering of `workflow`.
pub(crate) fn place(workflow: &Workflow, content: &str) -> Result<String> {
    let directives = directives(workflow)?;
    if directives.is_empty() {
        return Ok(content.to_owned());
    }

    let mut lines: Vec<String> = content.lines().map(str::to_owned).collect();
    for directive in &directives {
        let Directive { job, step_id, .. } = directive;
        let header = format!("  {job}:");
        let Some(start) = lines.iter().position(|line| line.trim_end() == header) else {
            bail!("job `{job}`: no `{header}` block in the rendered workflow");
        };
        let end = lines[start + 1..]
            .iter()
            .position(|line| !line.trim().is_empty() && indent(line) <= 2)
            .map(|offset| start + 1 + offset)
            .unwrap_or(lines.len());

        let hits: Vec<usize> = (start + 1..end)
            .filter(|&at| declares_id(&lines[at], step_id))
            .collect();
        match hits.as_slice() {
            [at] => {
                let comment = directive.comment();
                lines[*at].push_str(&comment);
            }
            [] => bail!("job `{job}`: no step with `id: {step_id}` in the rendered workflow"),
            _ => bail!("job `{job}`: `id: {step_id}` appears on more than one step"),
        }
    }

    let mut placed = lines.join("\n");
    if content.ends_with('\n') {
        placed.push('\n');
    }
    Ok(placed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gh_workflow::{Job, Step, Workflow};

    fn cache_step() -> Step<gh_workflow::Use> {
        Step::new("Cache")
            .uses("actions", "cache", "v5")
            .id("cache")
    }

    fn render(workflow: &Workflow) -> String {
        workflow.to_string().unwrap()
    }

    #[test]
    fn a_directive_lands_on_its_steps_id_line() {
        let workflow = Workflow::new("t").add_job(
            "build",
            Job::new("build")
                .add_step(cache_step().zizmor_ignore("cache-poisoning", "restores only")),
        );
        let placed = place(&workflow, &render(&workflow)).unwrap();

        let tagged: Vec<_> = placed.lines().filter(|l| l.contains("zizmor")).collect();
        assert_eq!(
            tagged,
            ["      - id: cache # zizmor: ignore[cache-poisoning] restores only"]
        );
    }

    #[test]
    fn two_rules_on_a_step_are_comma_joined() {
        let workflow = Workflow::new("t").add_job(
            "build",
            Job::new("build").add_step(
                cache_step()
                    .zizmor_ignore("cache-poisoning", "restores only")
                    .zizmor_ignore("artipacked", "no credentials"),
            ),
        );
        let placed = place(&workflow, &render(&workflow)).unwrap();

        assert!(placed.contains(
            "id: cache # zizmor: ignore[cache-poisoning,artipacked] restores only; no credentials"
        ));
    }

    #[test]
    fn a_directive_on_a_step_without_an_id_names_the_job() {
        let workflow = Workflow::new("t").add_job(
            "build",
            Job::new("build").add_step(
                Step::new("Cache")
                    .uses("actions", "cache", "v5")
                    .zizmor_ignore("cache-poisoning", "restores only"),
            ),
        );
        let error = place(&workflow, &render(&workflow))
            .unwrap_err()
            .to_string();

        assert!(error.contains("job `build`"), "{error}");
        assert!(error.contains("no `id`"), "{error}");
    }

    #[test]
    fn an_id_missing_from_the_rendering_fails() {
        let workflow = Workflow::new("t").add_job(
            "build",
            Job::new("build")
                .add_step(cache_step().zizmor_ignore("cache-poisoning", "restores only")),
        );
        let other = Workflow::new("t").add_job("build", Job::new("build"));
        let error = place(&workflow, &render(&other)).unwrap_err().to_string();

        assert!(error.contains("id: cache"), "{error}");
    }

    #[test]
    fn an_id_shared_by_two_jobs_is_placed_in_each() {
        let tagged = || cache_step().zizmor_ignore("cache-poisoning", "restores only");
        let workflow = Workflow::new("t")
            .add_job("first", Job::new("first").add_step(tagged()))
            .add_job("second", Job::new("second").add_step(cache_step()))
            .add_job("third", Job::new("third").add_step(tagged()));
        let placed = place(&workflow, &render(&workflow)).unwrap();

        let mut job = "";
        let mut tagged_in = Vec::new();
        for line in placed.lines() {
            if let Some(name) = line.strip_prefix("  ").and_then(|l| l.strip_suffix(':'))
                && !name.starts_with(' ')
            {
                job = name;
            }
            if line.contains("zizmor: ignore") {
                tagged_in.push(job.to_owned());
            }
        }
        assert_eq!(tagged_in, ["first", "third"]);
    }
}
