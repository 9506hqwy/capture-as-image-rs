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
                .conflicts_with("list")
                .takes_value(true),
        )
        .arg(
            Arg::new("fullscreen")
                .short('f')
                .long("fullscreen")
                .help("Specify if full screen capture taking")
                .conflicts_with_all(&["window", "list"]),
        )
        .arg(
            Arg::new("window")
                .short('w')
                .long("window")
                .value_name("TITLE")
                .help("Specify target window title")
                .conflicts_with_all(&["fullscreen", "list"])
                .takes_value(true),
        )
        .arg(
            Arg::new("desktop")
                .short('d')
                .long("desktop")
                .help("Specify if desktop window taking")
                .conflicts_with_all(&["fullscreen", "list"]),
        )
        .arg(
            Arg::new("list")
                .short('l')
                .long("list")
                .help("List desktop window name")
                .conflicts_with_all(&["output", "fullscreen", "window"]),
        )
        .get_matches();

    if matches.is_present("list") {
        print_window_name()?;
        return Ok(());
    }

    let output = matches.value_of("output").unwrap();
    let fullscreen = matches.is_present("fullscreen");
    let window = matches.value_of("window");
    let is_desktop = matches.is_present("desktop");

    let bmp = capture_as_image(fullscreen, window, is_desktop)?;

    let format = image::ImageFormat::from_path(output).unwrap();
    let img = image::load_from_memory_with_format(&bmp, image::ImageFormat::Bmp).unwrap();
    img.save_with_format(output, format).unwrap();
    Ok(())
}
