use crate::process::RunResult;

/// Render the Gantt timeline as a single line of labeled blocks, one block
/// per slice, each sized (roughly) to its duration.
pub fn gantt(result: &RunResult) -> String {
    if result.timeline.is_empty() {
        return "(no slices)".to_string();
    }
    let mut top = String::new();
    let mut bottom = String::new();
    for slice in &result.timeline {
        let label = match slice.pid {
            Some(pid) => format!("P{}", pid),
            None => "idle".to_string(),
        };
        let width = (label.len() + 2).max(4);
        top.push('|');
        top.push_str(&center(&label, width - 1));
        bottom.push_str(&format!("{:<width$}", slice.start, width = width));
    }
    top.push('|');
    bottom.push_str(&format!("{}", result.timeline.last().unwrap().end));
    format!("{}\n{}", top, bottom)
}

fn center(text: &str, width: usize) -> String {
    if text.len() >= width {
        return text.to_string();
    }
    let total_pad = width - text.len();
    let left = total_pad / 2;
    let right = total_pad - left;
    format!("{}{}{}", " ".repeat(left), text, " ".repeat(right))
}

/// Render the per-process metrics table plus the trailing averages row.
pub fn metrics_table(result: &RunResult) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{:<5}{:<9}{:<7}{:<11}{:<12}{:<9}{:<9}\n",
        "PID", "Arrival", "Burst", "Completion", "Turnaround", "Waiting", "Response"
    ));
    for m in &result.metrics {
        out.push_str(&format!(
            "{:<5}{:<9}{:<7}{:<11}{:<12}{:<9}{:<9}\n",
            m.id, m.arrival, m.burst, m.completion, m.turnaround, m.waiting, m.response
        ));
    }
    out.push_str(&format!(
        "\nAverages: turnaround={:.2} waiting={:.2} response={:.2}\n",
        result.averages.turnaround, result.averages.waiting, result.averages.response
    ));
    out
}
