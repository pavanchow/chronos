# Design

## Process model

A process is four numbers.

```rust
pub struct Process {
    pub id: usize,
    pub arrival: u32,
    pub burst: u32,
    pub priority: i32,
}
```

`id` is assigned by position in the input list. `arrival` is the time the process becomes ready to run. `burst` is the total CPU time it needs. `priority` only matters to the priority scheduler, where a lower value runs first, and it defaults to the process id when the caller does not give one.

A scheduler run produces three things: a `timeline` of `Slice { pid, start, end }` entries (a `None` pid marks an idle gap), a `Vec<Metrics>` in process-id order, and an `Averages` struct over turnaround, waiting, and response time.

Completion, turnaround, waiting, and response are all derived once a process finishes:

```
turnaround = completion - arrival
waiting    = turnaround - burst
response   = first_run  - arrival
```

`first_run` is the timestamp the process was first given the CPU, which for a non-preemptive scheduler is the same instant it starts running to completion.

## FIFO

Processes are sorted by `(arrival, id)` and run back to back with no interruption. If the next process in that order has not arrived yet when the current one finishes, an idle slice fills the gap up to its arrival.

## SJF

Non-preemptive. At every decision point the scheduler looks at every process that has arrived and not yet run, and picks the one with the smallest `burst`, ties broken by `(arrival, id)`. It then runs that process to completion before picking again. If nothing has arrived yet, time jumps forward to the next arrival and an idle slice is recorded.

## Round-robin

A ready queue admits processes in arrival order as the clock reaches their arrival time. The scheduler pops the front of the queue, runs it for `min(quantum, remaining_burst)`, and advances the clock by that amount. Any process that arrived during that slice is admitted to the back of the queue before the just-run process is re-queued, which is what keeps round-robin fair between processes that arrive close together. A process with zero remaining burst after its slice is marked complete instead of being re-queued. An empty ready queue with processes still to arrive causes an idle slice up to the next arrival.

## Priority

Structurally identical to SJF, except the selection key is `(priority, arrival, id)` instead of `(burst, arrival, id)`. This implementation is non-preemptive: once a process starts, it runs to completion even if a higher-priority process arrives partway through. That keeps the algorithm free of starvation-avoidance machinery (aging) that would otherwise be needed for a preemptive version, at the cost of a lower-priority process being able to briefly block a higher-priority one that arrives mid-burst.

## MLFQ

Three levels: level 0 has quantum 4, level 1 has quantum 8, level 2 has no quantum and runs first-come first-served to completion. A new process always enters level 0. The scheduler always looks at the highest level with a non-empty queue and runs the process at the front of it. If that process's remaining burst is fully consumed within the quantum, it completes right there. If it uses the entire quantum and still has burst left, it is demoted one level and placed at the back of that level's queue. A process already at the bottom level stays there. This reproduces the textbook MLFQ behavior of favoring short jobs (which finish at level 0 before ever being demoted) while still guaranteeing a long job eventually gets CPU time, just at a coarser granularity as it sinks through the levels.

## Metrics and averages

`finalize` in `src/process.rs` takes the completion time and first-run time recorded per process during simulation and computes the four metrics plus the three averages in one pass, dividing by the process count (guarded to at least 1 so an empty workload never divides by zero).

`coalesce` merges consecutive timeline slices that belong to the same process (or are both idle) into a single slice, so a round-robin or MLFQ run that happens to use back-to-back quanta for the same process is rendered as one continuous block instead of an artificial seam.
