use clap::Parser as _;
use editorconfig_core::{
    MAX_VERSION, Options, Version, properties_with_options,
};

#[derive(clap::Parser)]
struct Cli {
    #[arg(short, long)]
    version: bool,

    /// An EditorConfig file path.
    #[arg(short = 'f')]
    ec_file_name: Option<String>,

    /// EditorConfig version to use.
    #[arg(short = 'b')]
    ec_version: Option<Version>,

    files: Vec<String>,
}

fn main() {
    let args = Cli::parse();

    if args.version {
        print_version();
        return;
    }

    for file in args.files.iter() {
        if 1 < args.files.len() {
            println!("[{file}]");
        }

        let mut options = Options::default();
        if let Some(file_name) = args.ec_file_name.as_ref() {
            options.file_name = file_name;
        }
        if let Some(version) = args.ec_version {
            options.version = version;
        }

        print_pairs(file, options);
    }
}

fn print_pairs(file: &str, options: Options) {
    let props = properties_with_options(file, options).unwrap();

    let mut props = props.iter().collect::<Vec<_>>();
    // The testing suite expects them to be sorted.
    props.sort_unstable_by_key(|&(key, _value)| key);

    for (key, value) in props {
        println!("{key}={value}");
    }
}

fn print_version() {
    println!("EditorConfig Rust Core Version {MAX_VERSION}");
}
