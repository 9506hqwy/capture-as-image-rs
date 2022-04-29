#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Win32(windows::core::Error),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<windows::core::Error> for Error {
    fn from(err: windows::core::Error) -> Error {
        Error::Win32(err)
    }
}
