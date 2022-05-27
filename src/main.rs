use log::{debug, trace};
use clap::Parser;
use parser::class_tree;
use crate::formatter::FormattedClassEntry;
use crate::parser::jvm::Jvm;

mod parser;
//mod generator;
mod config;
mod formatter;

pub type JResult<T> = std::result::Result<T, anyhow::Error>;

#[derive(Parser, Debug)]
#[clap(author, version)]
struct Args {
    #[clap(short, long)]
    classpath: Vec<String>
}

fn main() {
    env_logger::init();
    debug!("Parsing arguments");
    let args = Args::parse();

    debug!("Creating JVM");
    let jvm = Jvm::new(&args.classpath).expect("Creating JVM");
    let env = jvm.attach_current_thread().expect("Attaching thread");

    debug!("Building class tree");
    let class_tree = class_tree::build(&env, "com.itextpdf.".into()).expect("Failed to build tree");

    trace!("Built tree:");
    trace!("{:#?}", class_tree);

    debug!("Formatting");
    let formatted = class_tree.into_iter()
        .map(FormattedClassEntry::from)
        .collect::<Vec<_>>();

    trace!("{:#?}", formatted);

    debug!("Generating code");
    //generator::generate(class_tree).expect("Failed to generate code");
}