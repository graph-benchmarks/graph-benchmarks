use proc_macro::TokenStream;
use quote::{format_ident, quote};
use serde::Deserialize;

#[derive(Deserialize)]
struct BuildConfig {
    drivers: Vec<String>,
    providers: Vec<String>
}

#[proc_macro]
pub fn include_drivers(_: TokenStream) -> TokenStream {
    let config: BuildConfig = toml::from_str(&std::fs::read_to_string("build.config.toml").unwrap()).unwrap();
    let mut drivers = Vec::new();
    let mut drivers_caps = Vec::new();

    for p in config.drivers {
        let d_cap: String = p
            .chars()
            .take(1)
            .flat_map(|f| f.to_uppercase())
            .chain(p.chars().skip(1))
            .collect();
        let d_name_caps = format_ident!("{}", d_cap);
        let d_name = format_ident!("{}_config", p);

        drivers.push(d_name);
        drivers_caps.push(d_name_caps);
    }

    quote !{
        pub const DRIVER_CONFIGS: &[&'static dyn DriverConfig] = &[#(&#drivers::#drivers_caps,)*];
    }.into()
}

#[proc_macro]
pub fn include_providers(_: TokenStream) -> TokenStream {
    let config: BuildConfig = toml::from_str(&std::fs::read_to_string("build.config.toml").unwrap()).unwrap();
    let mut provider = Vec::new();
    let mut provider_caps = Vec::new();

    for p in config.providers {
        let d_cap: String = p
            .chars()
            .take(1)
            .flat_map(|f| f.to_uppercase())
            .chain(p.chars().skip(1))
            .collect();
        let p_name_caps = format_ident!("{}", d_cap);
        let p_name = format_ident!("{}", p);

        provider.push(p_name);
        provider_caps.push(p_name_caps);
    }

    quote !{
        pub const PROVIDERS: &[&'static dyn Platform] = &[#(&#provider::#provider_caps,)*];
    }.into()
}