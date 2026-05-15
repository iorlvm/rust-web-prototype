use proc_macro2::Ident;
use quote::format_ident;

pub fn proxy_struct_ident(struct_name: &str) -> Ident {
    format_ident!("{}Proxy", struct_name)
}

pub fn print_debug_info(expanded: &proc_macro2::TokenStream, extra: ExtraInfo) {
    if std::env::var("IOC_LITE_DEBUG_MACRO").is_ok() {
        let file = syn::parse_file(&expanded.to_string()).unwrap();
        let pretty = prettyplease::unparse(&file);
        if let Some(pre) = &extra.pre {
            eprintln!("{}", pre);
        }
        eprintln!("{}", pretty);
        if let Some(post) = &extra.post {
            eprintln!("{}", post);
        }
    }
}

pub struct ExtraInfo {
    pre: Option<String>,
    post: Option<String>,
}
impl ExtraInfo {
    pub fn new(pre: Option<String>, post: Option<String>) -> Self {
        Self { pre, post }
    }
}
