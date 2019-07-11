use clap::{App, Arg, SubCommand};
use libc;
use std::ffi::{CString, OsString};
use std::io;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, SystemTime};

use csv::{self, WriterBuilder};
use proc_mounts::{self, MountIter};
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

#[derive(Debug, Serialize, Deserialize)]
pub struct SwapInfo {
    time_ms: Option<u128>,
    source: String,
    kind: String,
    size: usize,
    used: usize,
    priority: isize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MountInfo {
    time_ms: Option<u128>,
    source: String,
    dest: String,
    fstype: String,
    options: String,
    dump: i32,
    pass: i32,
    used: u64,
    available: u64,
    total: u64,
    use_pc: u32,
}

impl SwapInfo {
    pub fn new(
        time_ms: Option<u128>,
        source: PathBuf,
        kind: OsString,
        size: usize,
        used: usize,
        priority: isize,
    ) -> SwapInfo {
        SwapInfo {
            time_ms,
            source: String::from(source.to_string_lossy()),
            kind: String::from(kind.to_string_lossy()),
            size,
            used,
            priority,
        }
    }
}

pub fn statvfs(mount_point: &str) -> Option<libc::statvfs> {
    unsafe {
        let mountp = CString::new(mount_point).unwrap();
        let mut stats: libc::statvfs = std::mem::zeroed();
        if libc::statvfs(mountp.as_ptr(), &mut stats) != 0 {
            None
        } else {
            Some(stats)
        }
    }
}

pub fn fs_usage(mount_point: &str) -> (u64, u64, u64, u32) {
    match statvfs(mount_point) {
        Some(stats) => {
            let total = stats.f_blocks * stats.f_frsize / 1024;
            let available = stats.f_bavail * stats.f_frsize / 1024;
            let free = stats.f_bfree * stats.f_frsize / 1024;
            let used = total - free;
            let u100 = used * 100;
            let nonroot_total = used + available;
            let pct = if nonroot_total == 0 {
                0
            } else {
                u100 / nonroot_total
            };

            (used, available, total, pct as u32)
        }
        None => (0, 0, 0, 100),
    }
}

impl MountInfo {
    pub fn new(
        time_ms: Option<u128>,
        source: PathBuf,
        dest: PathBuf,
        fstype: &str,
        options: &Vec<String>,
        dump: i32,
        pass: i32,
    ) -> MountInfo {
        let dest_str = dest.to_string_lossy();
        let (used, available, total, use_pc) = fs_usage(&dest_str);
        MountInfo {
            time_ms,
            source: String::from(source.to_string_lossy()),
            dest: String::from(dest_str),
            fstype: String::from(fstype),
            options: options.join(";"),
            dump,
            pass,
            used,
            available,
            total,
            use_pc,
        }
    }
}

pub fn mount_stats_loop(interval: &Duration) {
    loop {
        let time_ms = timestamp();
        let stdout = io::stdout();
        let handle = stdout.lock();
        let mut wtr = WriterBuilder::new().has_headers(true).from_writer(handle);
        match MountIter::new() {
            Ok(mount_iter) => {
                for mount in mount_iter {
                    match mount {
                        Ok(proc_mounts::MountInfo {
                            source,
                            dest,
                            fstype,
                            options,
                            dump,
                            pass,
                        }) => {
                            let mount_info = MountInfo::new(
                                time_ms, source, dest, &fstype, &options, dump, pass,
                            );

                            match wtr.serialize(mount_info) {
                                Ok(_) => {}
                                Err(err) => {
                                    eprintln!("Error writing mount info: {}", err);
                                }
                            }
                        }
                        Err(err) => {
                            eprintln!("Error reading mount info: {}", err);
                        }
                    }
                }
            }
            Err(err) => {
                eprintln!("Error reading mount info: {}", err);
            }
        }

        /*match SwapIter::new() {
            Ok(swap_iter) => {
                for swap in swap_iter {
                    match swap {
                        Ok(proc_mounts::SwapInfo {
                            source,
                            kind,
                            size,
                            used,
                            priority,
                        }) => {
                            let swap_info =
                                SwapInfo::new(time_ms, source, kind, size, used, priority);
                            match wtr.serialize(swap_info) {
                                Ok(_) => {}
                                Err(err) => {
                                    eprintln!(
                                        "Error writing swap mount info: {} ({:?})",
                                        err, swap_info
                                    );
                                }
                            }
                        }
                        Err(err) => {
                            eprintln!("Error reading swap mount info: {}", err);
                        }
                    }
                }
            }
            Err(err) => {
                eprintln!("Error reading swap mount info: {}", err);
            }
        }*/

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
    MountStats { interval: Duration },
    Stop,
}

impl AppOptions {
    pub fn run(&self) {
        match self {
            AppOptions::SysStats { id, interval } => sys_stats_loop(id, interval),
            AppOptions::FdStats { interval } => fd_stats_loop(interval),
            AppOptions::MountStats { interval } => mount_stats_loop(interval),
            AppOptions::Stop => {
                eprintln!("nothing to do");
            }
        }
    }
}

pub fn parse_args() -> AppOptions {
    let matches = App::new("statsdump")
        .version("0.3")
        .author("Mariano Guerra <mariano@instadeq.com>")
        .about("dumps system stats")
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
        .subcommand(
            SubCommand::with_name("mount")
                .about("Collect mounted fs information")
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
        Some("mount") => {
            let cmatches = matches.subcommand_matches("mount").unwrap();
            let interval_secs = match cmatches.value_of("interval").unwrap_or("5").parse::<u64>() {
                Ok(n) => n,
                Err(err) => {
                    eprintln!("Invalid interval ({}), using default of 5 seconds", err);
                    5
                }
            };
            AppOptions::MountStats {
                interval: Duration::from_secs(interval_secs),
            }
        }

        None | Some(_) => AppOptions::Stop,
    }
}
