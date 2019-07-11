use clap::{App, Arg, SubCommand};
use libc;
use std::io;
use std::thread;
use std::time::{Duration, SystemTime};

use csv::{self, WriterBuilder};
use procfs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct SysInfo {
    id: String,
    time_ms: Option<u128>,
    mem_total: Option<u64>,
    mem_free: Option<u64>,
    mem_buffers: Option<u64>,
    mem_cached: Option<u64>,
    load_avg_1: Option<f32>,
    load_avg_5: Option<f32>,
    load_avg_15: Option<f32>,
}

impl SysInfo {
    pub fn new(id: String) -> SysInfo {
        let (mem_total, mem_free, mem_buffers, mem_cached) = match procfs::meminfo() {
            Ok(mi) => (
                Some(mi.mem_total),
                Some(mi.mem_free),
                Some(mi.buffers),
                Some(mi.cached),
            ),
            Err(err) => {
                eprintln!("Error loading meminfo: {}", err);
                (None, None, None, None)
            }
        };

        let (load_avg_1, load_avg_5, load_avg_15) = match procfs::LoadAverage::new() {
            Ok(la) => (Some(la.one), Some(la.five), Some(la.fifteen)),
            Err(err) => {
                eprintln!("Error loading load avg: {}", err);
                (None, None, None)
            }
        };

        let time_ms = timestamp();

        SysInfo {
            id,
            time_ms,
            mem_total,
            mem_free,
            mem_buffers,
            mem_cached,
            load_avg_1,
            load_avg_5,
            load_avg_15,
        }
    }

    pub fn write_stdout(&self, has_headers: bool) -> Result<(), csv::Error> {
        let stdout = io::stdout();
        let handle = stdout.lock();
        let mut wtr = WriterBuilder::new()
            .has_headers(has_headers)
            .from_writer(handle);
        wtr.serialize(self)
    }
}

fn timestamp() -> Option<u128> {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => Some(n.as_millis()),
        Err(err) => {
            eprintln!("Error getting time: {}", err);
            None
        }
    }
}

pub fn sys_stats_loop(id: &str, interval: &Duration) {
    let mut has_headers = true;

    loop {
        let sys_info = SysInfo::new(String::from(id));
        match sys_info.write_stdout(has_headers) {
            Ok(_) => {}
            Err(err) => {
                eprintln!("Error writing stats: {}", err);
            }
        }

        thread::sleep(*interval);
        has_headers = false;
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ProcInfo {
    time_ms: Option<u128>,
    pid: i32,
    owner: u32,
    open_fd_count: i64,
    num_threads: i64,
    starttime: i64,
    utime: u64,
    stime: u64,
    cmdline: String,
}

impl ProcInfo {
    pub fn new(time_ms: Option<u128>, proc: &procfs::Process) -> ProcInfo {
        let open_fd_count = match proc.fd() {
            Ok(fds) => fds.len() as i64,
            Err(_) => -1,
        };

        let cmdline = match proc.cmdline() {
            Ok(items) => {
                if items.len() == 0 {
                    String::from("?")
                } else {
                    items.join(" ")
                }
            }
            Err(_) => String::from("?"),
        };

        ProcInfo {
            time_ms,
            pid: proc.stat.pid,
            owner: proc.owner,
            open_fd_count,
            num_threads: proc.stat.num_threads,
            starttime: proc.stat.starttime,
            utime: proc.stat.utime,
            stime: proc.stat.stime,
            cmdline: cmdline,
        }
    }
}

pub fn fd_stats_loop(interval: &Duration) {
    loop {
        let time_ms = timestamp();
        let stdout = io::stdout();
        let handle = stdout.lock();
        let mut wtr = WriterBuilder::new().has_headers(true).from_writer(handle);
        for process in procfs::all_processes() {
            let proc_info = ProcInfo::new(time_ms, &process);
            match wtr.serialize(proc_info) {
                Ok(_) => {}
                Err(err) => {
                    eprintln!("Error serializing proc_info: {}", err);
                }
            }
        }

        match wtr.flush() {
            Ok(_) => {}
            Err(_) => {}
        }
        thread::sleep(*interval);
    }
}

pub fn setup_signals() {
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

pub enum AppOptions {
    SysStats { id: String, interval: Duration },
    FdStats { interval: Duration },
    Stop,
}

impl AppOptions {
    pub fn run(&self) {
        match self {
            AppOptions::SysStats { id, interval } => sys_stats_loop(id, interval),
            AppOptions::FdStats { interval } => fd_stats_loop(interval),
            AppOptions::Stop => {
                eprintln!("nothing to do");
            }
        }
    }
}

pub fn parse_args() -> AppOptions {
    let matches = App::new("statsdump")
        .version("0.2")
        .author("Mariano Guerra <mariano@instadeq.com>")
        .about("dumps system stats to stdout")
        .subcommand(
            SubCommand::with_name("sys")
                .about("Collect system information (CPU, Memory)")
                .arg(
                    Arg::with_name("id")
                        .short("i")
                        .long("id")
                        .value_name("ID")
                        .help("identifier of the stats, use hostname or similar")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("interval")
                        .short("s")
                        .long("interval-secs")
                        .takes_value(true)
                        .help("interval in seconds between writes"),
                ),
        )
        .subcommand(
            SubCommand::with_name("proc")
                .about("Collect process information (pid, fd, cmd etc)")
                .arg(
                    Arg::with_name("interval")
                        .short("s")
                        .long("interval-secs")
                        .takes_value(true)
                        .help("interval in seconds between writes"),
                ),
        )
        .get_matches();

    match matches.subcommand_name() {
        Some("sys") => {
            let cmatches = matches.subcommand_matches("sys").unwrap();
            let id = cmatches.value_of("id").unwrap_or("localhost");
            let interval_secs = match cmatches.value_of("interval").unwrap_or("5").parse::<u64>() {
                Ok(n) => n,
                Err(err) => {
                    eprintln!("Invalid interval ({}), using default of 5 seconds", err);
                    5
                }
            };

            AppOptions::SysStats {
                id: String::from(id),
                interval: Duration::from_secs(interval_secs),
            }
        }
        Some("proc") => {
            let cmatches = matches.subcommand_matches("proc").unwrap();
            let interval_secs = match cmatches.value_of("interval").unwrap_or("5").parse::<u64>() {
                Ok(n) => n,
                Err(err) => {
                    eprintln!("Invalid interval ({}), using default of 5 seconds", err);
                    5
                }
            };
            AppOptions::FdStats {
                interval: Duration::from_secs(interval_secs),
            }
        }

        None | Some(_) => AppOptions::Stop,
    }
}
