//! `render_plan_text`: the what-if counts and the greedy plan as plain text.

use std::fmt::Write;

use crate::census::models::PlanReport;

/// The what-if counts and the greedy plan, as text appended to `text`.
pub(crate) fn render_plan_text(text: &mut String, plan: &PlanReport) {
    let _ = writeln!(text);
    let _ = writeln!(
        text,
        "What-if and plan (upper bounds from the refusals collected; supporting a construct may \
         uncover others)"
    );
    let assumed = match plan.assumed.is_empty() {
        true => "nothing".to_owned(),
        false => plan.assumed.join(", "),
    };
    let _ = writeln!(
        text,
        "  assuming {assumed} supported: unit-testable at most {}, strict at most {} ({} possibly \
         incomplete)",
        plan.unit_testable_upper_bound, plan.strict_upper_bound, plan.possibly_incomplete
    );
    for next in &plan.next {
        let _ = writeln!(
            text,
            "  next: {:>6} files, {:>6} alone  {}: {}",
            next.files, next.sole, next.root.category, next.root.construct
        );
    }
    for (number, step) in plan.steps.iter().enumerate() {
        let roots: Vec<String> = step
            .roots
            .iter()
            .map(|root| format!("{}: {}", root.category, root.construct))
            .collect();
        let _ = writeln!(
            text,
            "  step {:>2}: unit-testable at most {:>6}, strict at most {:>6}  + {}",
            number + 1,
            step.unit_testable_upper_bound,
            step.strict_upper_bound,
            roots.join(" + ")
        );
    }
}
