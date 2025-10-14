use ipgrep::cli::Args;
use ipgrep::core::run;

// Measurements (ipgrep vs. grep):
//
// # time ./target/release/ipgrep 10.101.10.0/24 /etc/* -r >/dev/null
// (0major+709minor)
// real    0m0.068s
// user    0m0.044s
// sys     0m0.024s // slightly fewer syscalls (27049)
//
// # time grep 10.101.10.0/24 /etc/* -r >/dev/null
// (0major+245minor)
// real    0m0.042s
// user    0m0.012s
// sys     0m0.030s // slightly more syscalls (31372)
//
fn main() -> std::process::ExitCode {
    let args = Args::parse();
    let params = args.into_parameters();

    match run(&params) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("ipgrep: {e}");
            std::process::ExitCode::from(2)
        }
    }
}
