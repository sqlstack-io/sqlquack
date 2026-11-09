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
        "  assuming {assumed} supported: unit-testable at most {} ({} of them possibly \
         incomplete), strict at most {}",
        plan.unit_testable_upper_bound,
        plan.unit_testable_possibly_incomplete,
        plan.strict_upper_bound
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
            "  step {:>2}: unit-testable at most {:>6} ({:>5} possibly incomplete), strict at \
             most {:>6}  + {}",
            number + 1,
            step.unit_testable_upper_bound,
            step.unit_testable_possibly_incomplete,
            step.strict_upper_bound,
            roots.join(" + ")
        );
    }
}
