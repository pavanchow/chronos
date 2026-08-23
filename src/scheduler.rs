use crate::process::{coalesce, finalize, Process, RunResult, Slice};
use std::collections::VecDeque;

/// Processes ordered by arrival time, ties broken by id. Used by every
/// scheduler as the admission order.
fn arrival_order(procs: &[Process]) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..procs.len()).collect();
    idx.sort_by_key(|&i| (procs[i].arrival, procs[i].id));
    idx
}

/// First-come first-served. Runs each process to completion in arrival
/// order, oldest arrival (then lowest id) first.
pub fn fifo(procs: &[Process]) -> RunResult {
    if procs.is_empty() {
        return empty_result();
    }
    let order = arrival_order(procs);
    let mut time: u32 = 0;
    let mut timeline = Vec::new();
    let mut completions = vec![0u32; procs.len()];
    let mut first_run = vec![0u32; procs.len()];

    for i in order {
        let p = &procs[i];
        if p.arrival > time {
            timeline.push(Slice {
                pid: None,
                start: time,
                end: p.arrival,
            });
            time = p.arrival;
        }
        let start = time;
        let end = start + p.burst;
        first_run[i] = start;
        completions[i] = end;
        timeline.push(Slice {
            pid: Some(p.id),
            start,
            end,
        });
        time = end;
    }

    let mut result = finalize(procs, &completions, &first_run);
    result.timeline = coalesce(timeline);
    result
}

/// Shortest job first, non-preemptive. At every decision point the process
/// with the smallest total burst among those that have arrived is chosen,
/// ties broken by arrival then id.
pub fn sjf(procs: &[Process]) -> RunResult {
    non_preemptive_select(procs, |a, b| (a.burst, a.arrival, a.id).cmp(&(b.burst, b.arrival, b.id)))
}

/// Priority scheduling, non-preemptive. Lower `priority` value runs first.
/// Ties broken by arrival then id.
pub fn priority(procs: &[Process]) -> RunResult {
    non_preemptive_select(procs, |a, b| {
        (a.priority, a.arrival, a.id).cmp(&(b.priority, b.arrival, b.id))
    })
}

/// Shared engine for non-preemptive schedulers: repeatedly pick the best
/// arrived, unfinished process by `cmp` and run it to completion.
fn non_preemptive_select<F>(procs: &[Process], cmp: F) -> RunResult
where
    F: Fn(&Process, &Process) -> std::cmp::Ordering,
{
    if procs.is_empty() {
        return empty_result();
    }
    let mut remaining: Vec<usize> = (0..procs.len()).collect();
    let mut time: u32 = 0;
    let mut timeline = Vec::new();
    let mut completions = vec![0u32; procs.len()];
    let mut first_run = vec![0u32; procs.len()];

    while !remaining.is_empty() {
        let available: Vec<usize> = remaining
            .iter()
            .copied()
            .filter(|&i| procs[i].arrival <= time)
            .collect();

        if available.is_empty() {
            let next_arrival = remaining.iter().map(|&i| procs[i].arrival).min().unwrap();
            timeline.push(Slice {
                pid: None,
                start: time,
                end: next_arrival,
            });
            time = next_arrival;
            continue;
        }

        let chosen = *available
            .iter()
            .min_by(|&&a, &&b| cmp(&procs[a], &procs[b]))
            .unwrap();

        let p = &procs[chosen];
        let start = time;
        let end = start + p.burst;
        first_run[chosen] = start;
        completions[chosen] = end;
        timeline.push(Slice {
            pid: Some(p.id),
            start,
            end,
        });
        time = end;
        remaining.retain(|&i| i != chosen);
    }

    let mut result = finalize(procs, &completions, &first_run);
    result.timeline = coalesce(timeline);
    result
}

/// Round robin with a fixed quantum. Newly arrived processes join the back
/// of the ready queue in arrival order; a process that does not finish
/// within its quantum is re-queued behind them.
pub fn round_robin(procs: &[Process], quantum: u32) -> RunResult {
    if procs.is_empty() {
        return empty_result();
    }
    let quantum = quantum.max(1);
    let order = arrival_order(procs);
    let mut ptr = 0usize;
    let n = procs.len();

    let mut remaining_burst: Vec<u32> = procs.iter().map(|p| p.burst).collect();
    let mut completions = vec![0u32; n];
    let mut first_run: Vec<Option<u32>> = vec![None; n];
    let mut ready: VecDeque<usize> = VecDeque::new();
    let mut time: u32 = 0;
    let mut timeline = Vec::new();
    let mut finished = 0usize;

    let admit = |time: u32, ptr: &mut usize, ready: &mut VecDeque<usize>| {
        while *ptr < n && procs[order[*ptr]].arrival <= time {
            ready.push_back(order[*ptr]);
            *ptr += 1;
        }
    };

    admit(time, &mut ptr, &mut ready);

    while finished < n {
        if ready.is_empty() {
            let next_arrival = procs[order[ptr]].arrival;
            timeline.push(Slice {
                pid: None,
                start: time,
                end: next_arrival,
            });
            time = next_arrival;
            admit(time, &mut ptr, &mut ready);
            continue;
        }

        let pid_idx = ready.pop_front().unwrap();
        let run = quantum.min(remaining_burst[pid_idx]);
        let start = time;
        let end = start + run;
        if first_run[pid_idx].is_none() {
            first_run[pid_idx] = Some(start);
        }
        remaining_burst[pid_idx] -= run;
        time = end;
        timeline.push(Slice {
            pid: Some(procs[pid_idx].id),
            start,
            end,
        });

        admit(time, &mut ptr, &mut ready);

        if remaining_burst[pid_idx] == 0 {
            completions[pid_idx] = end;
            finished += 1;
        } else {
            ready.push_back(pid_idx);
        }
    }

    let first_run: Vec<u32> = first_run.into_iter().map(|f| f.unwrap_or(0)).collect();
    let mut result = finalize(procs, &completions, &first_run);
    result.timeline = coalesce(timeline);
    result
}

/// Multi-level feedback queue with three levels: quantum 4, quantum 8, and a
/// bottom level run first-come first-served to completion. A process starts
/// at level 0. If it uses its whole quantum without finishing it is demoted
/// one level; if it finishes within the quantum it completes on the spot.
/// Higher levels are always drained before a lower level runs.
pub fn mlfq(procs: &[Process]) -> RunResult {
    const LEVELS: usize = 3;
    const QUANTA: [u32; LEVELS] = [4, 8, u32::MAX];

    if procs.is_empty() {
        return empty_result();
    }
    let order = arrival_order(procs);
    let mut ptr = 0usize;
    let n = procs.len();

    let mut remaining_burst: Vec<u32> = procs.iter().map(|p| p.burst).collect();
    let mut completions = vec![0u32; n];
    let mut first_run: Vec<Option<u32>> = vec![None; n];
    let mut queues: [VecDeque<usize>; LEVELS] = Default::default();
    let mut time: u32 = 0;
    let mut timeline = Vec::new();
    let mut finished = 0usize;

    let admit = |time: u32, ptr: &mut usize, queues: &mut [VecDeque<usize>; LEVELS]| {
        while *ptr < n && procs[order[*ptr]].arrival <= time {
            queues[0].push_back(order[*ptr]);
            *ptr += 1;
        }
    };

    admit(time, &mut ptr, &mut queues);

    while finished < n {
        let level = queues.iter().position(|q| !q.is_empty());
        let level = match level {
            Some(l) => l,
            None => {
                let next_arrival = procs[order[ptr]].arrival;
                timeline.push(Slice {
                    pid: None,
                    start: time,
                    end: next_arrival,
                });
                time = next_arrival;
                admit(time, &mut ptr, &mut queues);
                continue;
            }
        };

        let pid_idx = queues[level].pop_front().unwrap();
        let quantum = QUANTA[level];
        let run = quantum.min(remaining_burst[pid_idx]);
        let start = time;
        let end = start + run;
        if first_run[pid_idx].is_none() {
            first_run[pid_idx] = Some(start);
        }
        remaining_burst[pid_idx] -= run;
        time = end;
        timeline.push(Slice {
            pid: Some(procs[pid_idx].id),
            start,
            end,
        });

        admit(time, &mut ptr, &mut queues);

        if remaining_burst[pid_idx] == 0 {
            completions[pid_idx] = end;
            finished += 1;
        } else {
            let next_level = (level + 1).min(LEVELS - 1);
            queues[next_level].push_back(pid_idx);
        }
    }

    let first_run: Vec<u32> = first_run.into_iter().map(|f| f.unwrap_or(0)).collect();
    let mut result = finalize(procs, &completions, &first_run);
    result.timeline = coalesce(timeline);
    result
}

fn empty_result() -> RunResult {
    RunResult {
        timeline: Vec::new(),
        metrics: Vec::new(),
        averages: crate::process::Averages {
            turnaround: 0.0,
            waiting: 0.0,
            response: 0.0,
        },
    }
}
