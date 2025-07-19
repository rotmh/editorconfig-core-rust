use clap::Parser as _;
use editorconfigcore::{
    DEFAULT_FILE_NAME, MAX_VERSION, Options, Properties, Version,
};

#[derive(clap::Parser)]
struct Cli {
    #[arg(short, long)]
    version: bool,

    /// An EditorConfig file path.
    #[arg(short = 'f', default_value_t = DEFAULT_FILE_NAME.to_string())]
    ec_file_name: String,

    /// EditorConfig version to use.
    #[arg(short = 'b', default_value_t = MAX_VERSION)]
    ec_version: Version,

    files: Vec<String>,
}

fn main() {
    let args = Cli::parse();

    if args.version {
        print_version();
        std::process::exit(0);
    }

    for file in args.files.iter() {
        if 1 < args.files.len() {
            println!("[{file}]");
        }

        let options = Options {
            file_name: &args.ec_file_name,
            version: args.ec_version,
            ..Options::default()
        };
        print_pairs(file, options);
    }
}

fn print_pairs(file: &str, options: Options) {
    let props = Properties::new_with_options(file, options).unwrap();

    let mut props = props.iter().collect::<Vec<_>>();
    props.sort_unstable_by_key(|&(key, _value)| key);

    for (key, value) in props {
        println!("{key}={value}");
    }
}

fn print_version() {
    println!("EditorConfig Version {MAX_VERSION}");
}
