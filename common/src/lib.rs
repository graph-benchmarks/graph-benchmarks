use std::collections::VecDeque;

use anyhow::Result;

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

pub fn traverse_yaml_mut<'a>(
    mut input: &'a mut serde_yaml::Value,
    path: &str,
) -> Result<Option<&'a mut serde_yaml::Value>> {
    let mut paths = path.split('.').into_iter().collect::<VecDeque<&str>>();
    while paths.len() > 0 {
        let path = paths.pop_front().unwrap();

        if input.is_mapping() {
            let v = input.as_mapping_mut().unwrap().get_mut(path);
            if v.is_none() {
                return Ok(None);
            }

            input = v.unwrap();
        } else if input.is_sequence() {
            let idx: usize = path.parse()?;
            let v = input.as_sequence_mut().unwrap();
            if idx >= v.len() {
                return Ok(None);
            }

            input = &mut v[idx];
        }
    }
    Ok(Some(input))
}
