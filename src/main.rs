use chronos::process::Process;
use chronos::{render, scheduler};
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "chronos", about = "A CPU scheduler simulator in Rust")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run a scheduling algorithm over a set of processes and print the
    /// Gantt chart and the metrics table.
    Run {
        /// Algorithm to run.
        #[arg(long, value_enum)]
        algo: Algo,
        /// Quantum for round-robin or the first MLFQ level. Ignored by
        /// algorithms that do not use one.
        #[arg(long, default_value_t = 2)]
        quantum: u32,
        /// Processes as "arrival:burst" or "arrival:burst:priority",
        /// comma-separated. Ids are assigned in the order given, starting
        /// at 0. Priority defaults to the process id (lower id runs first).
        #[arg(long)]
        procs: String,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum Algo {
    Fifo,
    Sjf,
    Rr,
    Priority,
    Mlfq,
}

fn parse_procs(spec: &str) -> Result<Vec<Process>, String> {
    let mut procs = Vec::new();
    for (id, entry) in spec.split(',').enumerate() {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        let fields: Vec<&str> = entry.split(':').collect();
        if fields.len() < 2 || fields.len() > 3 {
            return Err(format!(
                "invalid process spec '{}': expected arrival:burst or arrival:burst:priority",
                entry
            ));
        }
        let arrival: u32 = fields[0]
            .trim()
            .parse()
            .map_err(|_| format!("invalid arrival time in '{}'", entry))?;
        let burst: u32 = fields[1]
            .trim()
            .parse()
            .map_err(|_| format!("invalid burst time in '{}'", entry))?;
        let priority: i32 = if fields.len() == 3 {
            fields[2]
                .trim()
                .parse()
                .map_err(|_| format!("invalid priority in '{}'", entry))?
        } else {
            id as i32
        };
        procs.push(Process::new(id, arrival, burst, priority));
    }
    if procs.is_empty() {
        return Err("no processes given".to_string());
    }
    Ok(procs)
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Run {
            algo,
            quantum,
            procs,
        } => {
            let procs = match parse_procs(&procs) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("error: {}", e);
                    std::process::exit(1);
                }
            };
            let result = match algo {
                Algo::Fifo => scheduler::fifo(&procs),
                Algo::Sjf => scheduler::sjf(&procs),
                Algo::Rr => scheduler::round_robin(&procs, quantum),
                Algo::Priority => scheduler::priority(&procs),
                Algo::Mlfq => scheduler::mlfq(&procs),
            };
            println!("{}", render::gantt(&result));
            println!();
            print!("{}", render::metrics_table(&result));
        }
    }
}
