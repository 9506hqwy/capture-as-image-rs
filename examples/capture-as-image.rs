use capture_as_image::error::Error;
use capture_as_image::{capture_as_image, print_window_name};
use clap::{Arg, Command};

fn main() -> Result<(), Error> {
    env_logger::init();

    let matches = Command::new("capture-as-image")
        .version("0.1.0")
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("FILE")
                .help("Specify output filename")
                .required(true)
                .conflicts_with("list"),
        )
        .arg(
            Arg::new("fullscreen")
                .short('f')
                .long("fullscreen")
                .help("Specify if full screen capture taking")
                .conflicts_with_all(["window", "list"])
                .num_args(0),
        )
        .arg(
            Arg::new("clipping")
                .short('c')
                .long("clipping")
                .help("Specify if clipping from full screen")
                .conflicts_with_all(["fullscreen", "list"])
                .num_args(0),
        )
        .arg(
            Arg::new("window")
                .short('w')
                .long("window")
                .value_name("TITLE")
                .help("Specify target window title")
                .conflicts_with_all(["fullscreen", "list"]),
        )
        .arg(
            Arg::new("desktop")
                .short('d')
                .long("desktop")
                .help("Specify if desktop window taking")
                .conflicts_with_all(["fullscreen", "list"])
                .num_args(0),
        )
        .arg(
            Arg::new("list")
                .short('l')
                .long("list")
                .help("List desktop window name")
                .conflicts_with_all(["output", "fullscreen", "window"])
                .num_args(0),
        )
        .get_matches();

    if matches.get_flag("list") {
        print_window_name()?;
        return Ok(());
    }

    let output = matches.get_one::<String>("output").unwrap();
    let fullscreen = matches.get_flag("fullscreen");
    let window = matches.get_one::<String>("window");
    let is_desktop = matches.get_flag("desktop");
    let is_clipping = matches.get_flag("clipping");

    let bmp = capture_as_image(
        fullscreen,
        window.map(|x| x.as_str()),
        is_desktop,
        is_clipping,
    )?;

    let format = image::ImageFormat::from_path(output).unwrap();
    let img = image::load_from_memory_with_format(&bmp, image::ImageFormat::Bmp).unwrap();
    img.save_with_format(output, format).unwrap();
    Ok(())
}
