extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

struct ApiQuery {
    method: syn::Ident,
    url: syn::Expr,
    get_json: syn::LitBool,
    data: Option<syn::Expr>,
}

impl syn::parse::Parse for ApiQuery {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let method: syn::Ident = input.parse()?;
        input.parse::<syn::Token![,]>()?;
        let url: syn::Expr = input.parse()?;
        input.parse::<syn::Token![,]>()?;
        let get_json: syn::LitBool = input.parse()?;
        let data: Option<syn::Expr> = if input.peek(syn::Token![,]) {
            input.parse::<syn::Token![,]>()?;
            Some(input.parse()?)
        } else {
            None
        };
        Ok(Self {
            method,
            url,
            get_json,
            data,
        })
    }
}

/// Make an API query.
/// Arguments:
/// - method: The HTTP method to use (get, post, put, delete)
/// - url: The URL to query
/// - `get_json`: Whether to parse the response as JSON and return it
/// - data: The data to send in the request body
#[proc_macro]
pub fn api_query(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ApiQuery);
    let method = input.method;
    let url = input.url;
    let get_json = input.get_json.value;
    let json_str = if get_json {
        quote! {
            Ok(response.json().await?)
        }
    } else {
        quote! {
        Ok(())
        }
    };
    let data_str = input
        .data
        .map_or_else(|| quote! {}, |data| quote! { .json(&#data) });
    let gen = quote! {
        {
            let url = #url;
            let url = url.parse::<reqwest::Url>().unwrap();
            let response = self.client().#method(url)
                #data_str
                .send()
                .await?;
            #json_str
        }
    };
    gen.into()
}
