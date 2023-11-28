pub mod command;
pub mod config;
pub mod driver_config;
pub mod provider;

#[macro_export]
macro_rules! exit {
    ($err:expr, $($arg:tt)*) => {
        {
            tracing::error!($($arg)*);
            anyhow::bail!($err)
        }
    };
}
