use thiserror::Error;

#[derive(Error, Debug)]
pub enum DisplayError {
    #[error("GPIO error: {0}")]
    Gpio(String),

    #[error("SPI error: {0}")]
    Spi(String),

    #[error("I2C error: {0}")]
    I2c(String),

    #[error("Display busy timeout")]
    BusyTimeout,

    #[error("Rendering error: {0}")]
    Render(String),
}
