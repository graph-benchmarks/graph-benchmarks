use common::driver_config::DriverConfig;

pub fn get_driver_config(name: &str) -> Option<&'static dyn DriverConfig> {
    DRIVER_CONFIGS.iter().find(|x| x.name() == name).copied()
}

macros::include_drivers!();
