use clap::Parser as _;
use tamlil_editorconfig::{EDITORCONFIG_VERSION, Options, Properties};

#[derive(clap::Parser)]
struct Cli {
    #[arg(short, long)]
    version: bool,

    #[arg(short = 'f')]
    ec_file_name: Option<String>,

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
            file_name: args.ec_file_name.as_deref().unwrap_or(".editorconfig"),
            allow_unset: false,
        };
        print_pairs(file, options);
    }
}

fn print_pairs(file: &str, options: Options) {
    let props = Properties::new_with_options(file, options).unwrap();

    for (key, value) in props.iter() {
        println!("{key}={value}");
    }
}

fn print_version() {
    println!(
        "EditorConfig Version {}.{}.{}",
        EDITORCONFIG_VERSION.major,
        EDITORCONFIG_VERSION.minor,
        EDITORCONFIG_VERSION.patch
    );
}
