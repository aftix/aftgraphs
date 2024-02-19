use crate::block_on;
use async_rwlock::RwLock;
use clap::{crate_version, Args, Command};
use lazy_static::lazy_static;
use std::{num::NonZeroU32, path::PathBuf};

lazy_static! {
    pub static ref ARGUMENTS: RwLock<Arguments> = RwLock::new(Arguments::default());
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct HeadlessArgs {
    pub out_file: PathBuf,
    pub in_file: PathBuf,
    pub size: (Option<u32>, Option<u32>),
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct Arguments {
    pub headless: Option<HeadlessArgs>,
    pub render_imgui: bool,
}

#[derive(Args)]
#[clap(version, long_about = None)]
struct MyArgs {
    #[clap(long, action, name = "render-imgui")]
    render_imgui: bool,
    #[clap(long, short, requires = "output")]
    render: Option<PathBuf>,
    #[clap(long, short, requires = "render")]
    output: Option<PathBuf>,
    #[clap(long, short = 'W', requires = "render")]
    width: Option<NonZeroU32>,
    #[clap(long, short = 'H', requires = "render")]
    height: Option<NonZeroU32>,
}

pub fn parse_cli(name: &str, description: Option<&str>, author: Option<&str>) {
    let cmd = command(name, description, author);
    let matches = cmd.get_matches();

    let in_file: Option<PathBuf> = matches.get_one("render").cloned();
    let out_file: Option<PathBuf> = matches.get_one("output").cloned();
    let width: Option<NonZeroU32> = matches.get_one("width").copied();
    let height: Option<NonZeroU32> = matches.get_one("height").copied();

    let size = (width.map(Into::<u32>::into), height.map(Into::<u32>::into));

    let headless = if let (Some(in_file), Some(out_file)) = (in_file, out_file) {
        Some(HeadlessArgs {
            out_file,
            in_file,
            size,
        })
    } else {
        None
    };

    let render_imgui = if let Some(&f) = matches.get_one::<bool>("render-imgui") {
        f
    } else {
        false
    };

    block_on(async move {
        let mut args = ARGUMENTS.write().await;
        *args = Arguments {
            headless,
            render_imgui,
        };
    });
}

pub fn command(name: &str, description: Option<&str>, author: Option<&str>) -> Command {
    let name = Box::leak(Box::new(name.to_owned()));
    let description = Box::leak(Box::new(description.map(ToOwned::to_owned)));
    let author = Box::leak(Box::new(author.map(ToOwned::to_owned)));

    let mut cmd = MyArgs::augment_args(
        Command::new(name.as_str())
            .bin_name(name.as_str())
            .version(crate_version!()),
    );

    if let Some(description) = description.as_ref().map(String::as_str) {
        cmd = cmd.about(description);
    }

    if let Some(author) = author.as_ref().map(String::as_str) {
        cmd = cmd.author(author);
    }

    cmd
}
