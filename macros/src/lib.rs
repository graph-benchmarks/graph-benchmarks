use proc_macro::TokenStream;
use proc_macro2::TokenStream as Ts;
use quote::{quote, format_ident};

#[proc_macro]
pub fn include_driver_config(_: TokenStream) -> TokenStream {
    let drivers = std::fs::read_to_string("build-drivers").unwrap();
    let mut drivers_ident = Vec::new();
    let mut drivers_ident_caps = Vec::new();

    let mut stream = Ts::new();
    for d in drivers.split('\n').map(|x| x.trim().to_string()).collect::<Vec<String>>() {
        let driver_path = format!("../../drivers/{d}/config.rs");
        let d_name = format_ident!("{}", d);

        let d_cap: String = d.chars().take(1).flat_map(|f| f.to_uppercase()).chain(d.chars().skip(1)).collect();
        let d_name_caps = format_ident!("{}", d_cap);
        drivers_ident.push(d_name.clone());
        drivers_ident_caps.push(d_name_caps);

        stream.extend::<Ts>(quote! {
            mod #d_name {
                include!(#driver_path);
            }
        });
    }

    stream.extend::<Ts>(quote!{
        pub const DRIVER_CONFIGS: &[&'static dyn DriverConfig] = &[&#(#drivers_ident::#drivers_ident_caps,)*];
    });

    stream.into()
}
