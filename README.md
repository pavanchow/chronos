<img src="docs/logo.svg" alt="Chronos logo" width="96">

# Chronos: a CPU scheduler simulator in Rust

Chronos is a from-scratch CPU scheduling simulator in Rust that runs the algorithms instead of drawing them. It runs FIFO, SJF, round-robin, priority, and a multi-level feedback queue over a set of processes and reports the actual turnaround, waiting, and response times with a Gantt chart, not the ones a textbook rounded off. It is a readable reference for learning operating-system CPU scheduling and getting exact metrics for a specific workload.

**[Live demo](https://pavanchow.github.io/chronos/)** · MIT licensed · pure Rust

## Algorithms

- **FIFO (first-come first-served)** runs each process to completion in arrival order.
- **SJF (shortest job first)**, non-preemptive, always picks the shortest burst among the processes that have arrived.
- **Round-robin** time-slices every ready process for a configurable quantum, cycling until each one finishes.
- **Priority**, non-preemptive, picks the lowest priority value among arrived processes, lower runs first.
- **MLFQ (multi-level feedback queue)** starts every process at the top level. A process that uses its whole quantum without finishing gets demoted a level. The bottom level runs to completion, first-come first-served.

Every scheduler is deterministic and pure. No workload panics it, including processes that all arrive at the same instant or a workload with idle gaps between arrivals.

## Usage

```
cargo run -- run --algo rr --quantum 2 --procs "0:5,1:3,2:8"
```

`--procs` takes a comma-separated list of processes as `arrival:burst` or `arrival:burst:priority`. Ids are assigned in the order given, starting at 0. Priority defaults to the process id when omitted.

```
cargo run -- run --algo fifo --procs "0:5,1:3,2:8"
cargo run -- run --algo sjf --procs "0:8,1:3,2:4"
cargo run -- run --algo priority --procs "0:5:2,1:3:1,2:8:0"
cargo run -- run --algo mlfq --procs "0:20,1:2"
```

Each run prints the Gantt chart as a labeled block sequence with time marks underneath, then a metrics table with completion, turnaround, waiting, and response time per process, plus the averages.

## Testing

```
cargo test
```

Tests assert exact metrics against hand-computed answers for every scheduler, including a traced round-robin run and an MLFQ run that demotes a long job across all three levels.

## License

MIT licensed. By Pavan Nallamothu.
