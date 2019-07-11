use clap::{App, Arg};
use libc;
use std::io;
use std::thread;
use std::time::{Duration, SystemTime};

use csv::{self, WriterBuilder};
use procfs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Row {
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

impl Row {
    pub fn new(id: String) -> Row {
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

        let time_ms = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
            Ok(n) => Some(n.as_millis()),
            Err(err) => {
                eprintln!("Error getting time: {}", err);
                None
            }
        };

        Row {
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

pub fn sys_stats_loop(id: &str, interval: &Duration) {
    let mut has_headers = true;

    loop {
        let row = Row::new(String::from(id));
        match row.write_stdout(has_headers) {
            Ok(_) => {}
            Err(err) => {
                eprintln!("Error writing stats: {}", err);
            }
        }

        thread::sleep(*interval);
        has_headers = false;
    }
}

pub fn setup_signals() {
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

pub enum AppOptions {
    SysStats { id: String, interval: Duration },
}

impl AppOptions {
    pub fn run(&self) {
        match self {
            AppOptions::SysStats { id, interval } => sys_stats_loop(id, interval),
        }
    }
}

pub fn parse_args() -> AppOptions {
    let matches = App::new("statsdump")
        .version("0.1")
        .author("Mariano Guerra <mariano@instadeq.com>")
        .about("dumps system stats to stdout")
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
        )
        .get_matches();

    let id = matches.value_of("id").unwrap_or("localhost");
    let interval_secs = match matches.value_of("interval").unwrap_or("5").parse::<u64>() {
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
