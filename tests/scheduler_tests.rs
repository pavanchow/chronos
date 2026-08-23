use chronos::process::Process;
use chronos::scheduler;

fn procs(spec: &[(usize, u32, u32, i32)]) -> Vec<Process> {
    spec.iter()
        .map(|&(id, arrival, burst, priority)| Process::new(id, arrival, burst, priority))
        .collect()
}

#[test]
fn fifo_orders_by_arrival_and_matches_hand_computed_turnaround() {
    // P0 arrives 0 burst 5, P1 arrives 1 burst 3, P2 arrives 2 burst 8.
    // FIFO runs strictly in arrival order: 0-5, 5-8, 8-16.
    let ps = procs(&[(0, 0, 5, 0), (1, 1, 3, 0), (2, 2, 8, 0)]);
    let result = scheduler::fifo(&ps);

    assert_eq!(result.timeline.len(), 3);
    assert_eq!(result.timeline[0].pid, Some(0));
    assert_eq!((result.timeline[0].start, result.timeline[0].end), (0, 5));
    assert_eq!(result.timeline[1].pid, Some(1));
    assert_eq!((result.timeline[1].start, result.timeline[1].end), (5, 8));
    assert_eq!(result.timeline[2].pid, Some(2));
    assert_eq!((result.timeline[2].start, result.timeline[2].end), (8, 16));

    // completion, turnaround, waiting, response by hand:
    // P0: completion 5, turnaround 5, waiting 0, response 0
    // P1: completion 8, turnaround 7, waiting 4, response 4
    // P2: completion 16, turnaround 14, waiting 6, response 6
    assert_eq!(result.metrics[0].completion, 5);
    assert_eq!(result.metrics[0].turnaround, 5);
    assert_eq!(result.metrics[0].waiting, 0);
    assert_eq!(result.metrics[0].response, 0);

    assert_eq!(result.metrics[1].completion, 8);
    assert_eq!(result.metrics[1].turnaround, 7);
    assert_eq!(result.metrics[1].waiting, 4);
    assert_eq!(result.metrics[1].response, 4);

    assert_eq!(result.metrics[2].completion, 16);
    assert_eq!(result.metrics[2].turnaround, 14);
    assert_eq!(result.metrics[2].waiting, 6);
    assert_eq!(result.metrics[2].response, 6);

    let avg_turnaround = (5.0 + 7.0 + 14.0) / 3.0;
    let avg_waiting = (0.0 + 4.0 + 6.0) / 3.0;
    let avg_response = (0.0 + 4.0 + 6.0) / 3.0;
    assert!((result.averages.turnaround - avg_turnaround).abs() < 1e-9);
    assert!((result.averages.waiting - avg_waiting).abs() < 1e-9);
    assert!((result.averages.response - avg_response).abs() < 1e-9);
}

#[test]
fn fifo_handles_all_same_arrival_without_panic_and_orders_by_id() {
    let ps = procs(&[(0, 0, 4, 0), (1, 0, 2, 0), (2, 0, 1, 0)]);
    let result = scheduler::fifo(&ps);
    // same arrival, so FIFO falls back to id order: 0, 1, 2.
    assert_eq!(result.timeline[0].pid, Some(0));
    assert_eq!(result.timeline[1].pid, Some(1));
    assert_eq!(result.timeline[2].pid, Some(2));
}

#[test]
fn fifo_produces_idle_slice_for_gap_before_first_arrival() {
    let ps = procs(&[(0, 3, 2, 0)]);
    let result = scheduler::fifo(&ps);
    assert_eq!(result.timeline[0].pid, None);
    assert_eq!((result.timeline[0].start, result.timeline[0].end), (0, 3));
    assert_eq!(result.timeline[1].pid, Some(0));
    assert_eq!((result.timeline[1].start, result.timeline[1].end), (3, 5));
}

#[test]
fn round_robin_quantum_two_produces_expected_gantt_slices() {
    // Hand-traced with quantum 2:
    // P0(0,5) P1(1,3) P2(2,8)
    // 0-2 P0, 2-4 P1, 4-6 P2, 6-8 P0, 8-9 P1(done), 9-11 P2, 11-12 P0(done), 12-16 P2(done)
    let ps = procs(&[(0, 0, 5, 0), (1, 1, 3, 0), (2, 2, 8, 0)]);
    let result = scheduler::round_robin(&ps, 2);

    let expected: Vec<(Option<usize>, u32, u32)> = vec![
        (Some(0), 0, 2),
        (Some(1), 2, 4),
        (Some(2), 4, 6),
        (Some(0), 6, 8),
        (Some(1), 8, 9),
        (Some(2), 9, 11),
        (Some(0), 11, 12),
        (Some(2), 12, 16),
    ];
    let actual: Vec<(Option<usize>, u32, u32)> = result
        .timeline
        .iter()
        .map(|s| (s.pid, s.start, s.end))
        .collect();
    assert_eq!(actual, expected);

    // average waiting time by hand: P0 waiting 7, P1 waiting 5, P2 waiting 6
    assert_eq!(result.metrics[0].waiting, 7);
    assert_eq!(result.metrics[1].waiting, 5);
    assert_eq!(result.metrics[2].waiting, 6);
    let avg_waiting = (7.0 + 5.0 + 6.0) / 3.0;
    assert!((result.averages.waiting - avg_waiting).abs() < 1e-9);
}

#[test]
fn round_robin_never_panics_on_idle_gap_between_bursts() {
    let ps = procs(&[(0, 0, 2, 0), (1, 10, 2, 0)]);
    let result = scheduler::round_robin(&ps, 2);
    assert_eq!(result.metrics.len(), 2);
    assert_eq!(result.metrics[0].completion, 2);
    assert_eq!(result.metrics[1].completion, 12);
}

#[test]
fn sjf_selects_shortest_job_among_arrived_processes() {
    // P0 arrives 0 burst 8. By the time P0 would finish, P1 (arrival 1, burst 3)
    // and P2 (arrival 2, burst 4) have both arrived; SJF must still run P0 to
    // completion first (non-preemptive), then pick the shorter of the two: P1.
    let ps = procs(&[(0, 0, 8, 0), (1, 1, 3, 0), (2, 2, 4, 0)]);
    let result = scheduler::sjf(&ps);

    let order: Vec<Option<usize>> = result.timeline.iter().map(|s| s.pid).collect();
    assert_eq!(order, vec![Some(0), Some(1), Some(2)]);
    assert_eq!(result.metrics[0].completion, 8);
    assert_eq!(result.metrics[1].completion, 11);
    assert_eq!(result.metrics[2].completion, 15);
}

#[test]
fn sjf_picks_shorter_ready_job_over_longer_one_that_arrived_earlier() {
    // P0 arrives 0 burst 5. P1 arrives 0 burst 2. Both ready at time 0,
    // SJF must pick P1 (shorter burst) first even though ids tie-break by id.
    let ps = procs(&[(0, 0, 5, 0), (1, 0, 2, 0)]);
    let result = scheduler::sjf(&ps);
    assert_eq!(result.timeline[0].pid, Some(1));
    assert_eq!(result.timeline[1].pid, Some(0));
}

#[test]
fn priority_scheduling_picks_lowest_priority_value_first() {
    // Lower priority number = runs first. P2 (priority 0) beats P1 (priority 1)
    // once both have arrived, even though P1 arrived earlier.
    let ps = procs(&[(0, 0, 5, 2), (1, 1, 3, 1), (2, 2, 8, 0)]);
    let result = scheduler::priority(&ps);
    let order: Vec<Option<usize>> = result.timeline.iter().map(|s| s.pid).collect();
    assert_eq!(order, vec![Some(0), Some(2), Some(1)]);
}

#[test]
fn mlfq_demotes_a_long_job_across_levels() {
    // Levels: quantum 4, quantum 8, then FCFS. P0 (burst 20) never finishes
    // inside its quantum at level 0 or level 1, so it must be demoted twice:
    // level 0 -> level 1 -> level 2. P1 (burst 2, arrives at 1) finishes
    // inside its very first quantum at level 0.
    let ps = procs(&[(0, 0, 20, 0), (1, 1, 2, 0)]);
    let result = scheduler::mlfq(&ps);

    // First slice: P0 runs its level-0 quantum (4) and gets demoted.
    assert_eq!(result.timeline[0].pid, Some(0));
    assert_eq!((result.timeline[0].start, result.timeline[0].end), (0, 4));

    // P1 preempts at level 0 next since it is still at the top level.
    assert_eq!(result.timeline[1].pid, Some(1));
    assert_eq!((result.timeline[1].start, result.timeline[1].end), (4, 6));
    assert_eq!(result.metrics[1].completion, 6);

    // P0 finishes off its remaining 16 units across level 1 (quantum 8,
    // demoted again) and level 2 (FCFS, runs to completion): 6-22 total.
    assert_eq!(result.metrics[0].completion, 22);
    let p0_total_run: u32 = result
        .timeline
        .iter()
        .filter(|s| s.pid == Some(0))
        .map(|s| s.end - s.start)
        .sum();
    assert_eq!(p0_total_run, 20);
}

#[test]
fn no_scheduler_panics_on_empty_or_degenerate_workloads() {
    let empty: Vec<Process> = Vec::new();
    let _ = scheduler::fifo(&empty);
    let _ = scheduler::sjf(&empty);
    let _ = scheduler::round_robin(&empty, 3);
    let _ = scheduler::priority(&empty);
    let _ = scheduler::mlfq(&empty);

    let single = procs(&[(0, 0, 1, 0)]);
    let _ = scheduler::fifo(&single);
    let _ = scheduler::round_robin(&single, 100);
    let _ = scheduler::mlfq(&single);

    let all_same_arrival = procs(&[(0, 5, 3, 0), (1, 5, 1, 0), (2, 5, 2, 0)]);
    let _ = scheduler::fifo(&all_same_arrival);
    let _ = scheduler::sjf(&all_same_arrival);
    let _ = scheduler::round_robin(&all_same_arrival, 1);
    let _ = scheduler::priority(&all_same_arrival);
    let _ = scheduler::mlfq(&all_same_arrival);
}
