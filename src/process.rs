/// A single process to be scheduled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Process {
    pub id: usize,
    pub arrival: u32,
    pub burst: u32,
    pub priority: i32,
}

impl Process {
    pub fn new(id: usize, arrival: u32, burst: u32, priority: i32) -> Self {
        Process {
            id,
            arrival,
            burst,
            priority,
        }
    }
}

/// One contiguous slice of the Gantt timeline. `pid` is `None` for an idle slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slice {
    pub pid: Option<usize>,
    pub start: u32,
    pub end: u32,
}

/// Per-process outcome of a scheduling run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Metrics {
    pub id: usize,
    pub arrival: u32,
    pub burst: u32,
    pub completion: u32,
    pub turnaround: u32,
    pub waiting: u32,
    pub response: u32,
}

/// Aggregate averages over a set of per-process metrics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Averages {
    pub turnaround: f64,
    pub waiting: f64,
    pub response: f64,
}

/// The full result of running a scheduler: the Gantt timeline, per-process
/// metrics in process-id order, and the averages across them.
#[derive(Debug, Clone)]
pub struct RunResult {
    pub timeline: Vec<Slice>,
    pub metrics: Vec<Metrics>,
    pub averages: Averages,
}

/// Build metrics and averages from completion times and the first time each
/// process was ever given the CPU. `completions[i]` and `first_run[i]` line
/// up with `procs[i]`.
pub fn finalize(procs: &[Process], completions: &[u32], first_run: &[u32]) -> RunResult {
    let mut metrics: Vec<Metrics> = procs
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let completion = completions[i];
            let turnaround = completion - p.arrival;
            let waiting = turnaround - p.burst;
            let response = first_run[i] - p.arrival;
            Metrics {
                id: p.id,
                arrival: p.arrival,
                burst: p.burst,
                completion,
                turnaround,
                waiting,
                response,
            }
        })
        .collect();
    metrics.sort_by_key(|m| m.id);

    let n = metrics.len().max(1) as f64;
    let averages = Averages {
        turnaround: metrics.iter().map(|m| m.turnaround as f64).sum::<f64>() / n,
        waiting: metrics.iter().map(|m| m.waiting as f64).sum::<f64>() / n,
        response: metrics.iter().map(|m| m.response as f64).sum::<f64>() / n,
    };

    RunResult {
        timeline: Vec::new(),
        metrics,
        averages,
    }
}

/// Merge adjacent slices that run the same process (or are both idle) into
/// one, so the Gantt chart reads as continuous runs rather than 1-unit ticks.
pub fn coalesce(mut slices: Vec<Slice>) -> Vec<Slice> {
    let mut out: Vec<Slice> = Vec::with_capacity(slices.len());
    slices.retain(|s| s.start < s.end);
    for s in slices {
        if let Some(last) = out.last_mut() {
            if last.pid == s.pid && last.end == s.start {
                last.end = s.end;
                continue;
            }
        }
        out.push(s);
    }
    out
}
