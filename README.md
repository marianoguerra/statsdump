# statsdump

Dump system stats (time, load, memory) to stdout as a csv.

## Usage

```
statsdump 0.3
dumps system stats

USAGE:
    statsdump [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    help     Prints this message or the help of the given subcommand(s)
    mount    Collect mounted fs information
    proc     Collect process information (pid, fd, cmd etc)
    sys      Collect system information (CPU, Memory)

```

## Examples

### Print system stats to stdout

```
statsdump sys --id $(hostname) --interval-secs 1
```

### Dump system stats to a file

```
statsdump sys --id $(hostname) --interval-secs 1 > stats.csv
```

### Print process stats to stdout

```
statsdump proc --interval-secs 1

```

### Print mount stats to stdout

```
statsdump mount --interval-secs 1

```

`Ctrl-C` to stop

### Sample System Stats Output

```csv
id,time_ms,mem_total,mem_free,mem_buffers,mem_cached,load_avg_1,load_avg_5,load_avg_15
ganesha,1562673069611,8268804096,611717120,236593152,2151419904,0.65,1.05,0.84
ganesha,1562673070612,8268804096,614203392,236601344,2152075264,0.65,1.05,0.84
ganesha,1562673071612,8268804096,614273024,236601344,2151960576,0.65,1.05,0.84
ganesha,1562673072613,8268804096,602386432,236634112,2146725888,0.6,1.04,0.83
ganesha,1562673073613,8268804096,601354240,236666880,2146148352,0.6,1.04,0.83
ganesha,1562673074613,8268804096,599031808,236716032,2146627584,0.6,1.04,0.83
ganesha,1562673075614,8268804096,599031808,236740608,2146447360,0.6,1.04,0.83
```

### Sample Process Stats Output

```csv
time_ms,pid,owner,open_fd_count,num_threads,starttime,utime,stime,cmdline
1562834946950,1,0,-1,1,20,1605,13005,/sbin/init splash
...
```

### Sample Process Stats Output

```csv
time_ms,source,dest,fstype,options,dump,pass
1562837513947,sysfs,/sys,sysfs,rw;nosuid;nodev;noexec;relatime,0,0
1562837513947,proc,/proc,proc,rw;nosuid;nodev;noexec;relatime,0,0
1562837513947,udev,/dev,devtmpfs,rw;nosuid;relatime;size=4013116k;nr_inodes=1003279;mode=755,0,0
...
```

## License

MIT

