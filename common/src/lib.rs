pub mod provider;
pub mod config;
pub mod command;
pub mod driver_config;

#[macro_export]
macro_rules! exit {
    ($err:expr, $($arg:tt)*) => {
        {
            tracing::error!($($arg)*);
            anyhow::bail!($err)
        }
    };
}