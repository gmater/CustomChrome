pub mod device;
pub mod protocol;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("No AULA F99 MAX keyboard is connected.")]
    NotConnected,
    #[error("The keyboard is on the wireless dongle. Plug in the USB cable and switch the keyboard to wired mode to change the screen.")]
    WiredRequired,
    #[error("\"{0}\" uses the same USB ID as the F99 MAX but is a different keyboard, so CustomChrome won't change its settings.")]
    UnsupportedModel(String),
    #[error("That lighting effect isn't available.")]
    UnsupportedEffect,
    #[error("That key or assignment can't be changed.")]
    InvalidKey,
    #[error("That macro is empty, has an invalid step, or doesn't fit in the keyboard's macro memory.")]
    InvalidMacro,
    #[error("That setting isn't valid.")]
    InvalidSetting,
    #[error("The keyboard's {0} interface is missing or not what CustomChrome expects.")]
    UnexpectedInterface(&'static str),
    #[error("The keyboard did not reply in time.")]
    Timeout,
    #[error("The keyboard didn't store the change correctly. Try again.")]
    WriteNotVerified,
    #[error("The keyboard did not report how many screen frames it can store.")]
    UnknownFrameLimit,
    #[error("Invalid image data: {0}")]
    InvalidImage(String),
    #[error("The keyboard is busy with another task. Try again when it finishes.")]
    Busy,
    #[error("The upload was interrupted.")]
    Interrupted,
    #[error("USB error: {0}")]
    Hid(#[from] hidapi::HidError),
}

impl serde::Serialize for Error {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
