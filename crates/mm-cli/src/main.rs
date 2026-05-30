use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "mm")]
#[command(version)]
struct Cli {}

fn main() {
    let _cli = Cli::parse();
}
