use statsdump::{parse_args, setup_signals};

fn main() {
    setup_signals();
    let args = parse_args();
    args.run();
}
