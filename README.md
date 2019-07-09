# statsdump

Dump system stats (time, load, memory) to stdout as a csv.

## Usage

```
statsdump 0.1
dumps system stats to stdout

USAGE:
    statsdump [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -i, --id <ID>                     identifier of the stats, use hostname or similar
    -s, --interval-secs <interval>    interval in seconds between writes
```

## Examples

### Print to stdout

```
statsdump --id $(hostname) --interval-secs 1
```

### Dump to a file

```
statsdump --id $(hostname) --interval-secs 1 > stats.csv
```

`Ctrl-C` to stop

### Sample Output

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

## License

MIT

