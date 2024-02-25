extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

#[proc_macro_derive(ReadApi)]
pub fn read_api_derive(input: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();
    let name = &ast.ident;
    let gen = quote! {
        impl ReadApi for #name {
            async fn get_watchers(&self) -> Result<BTreeMap<String, Watcher>, ApiError> {
                api_query!(get, format!("{}/watchers", self.base_url), true)
            }
            async fn get_watcher(&self, name: &str) -> Result<Watcher, ApiError> {
                api_query!(get, format!("{}/watchers/{}", self.base_url, name), true)
            }
            async fn get_watcher_spec(&self, name: &str) -> Result<Spec, ApiError> {
                api_query!(get, format!("{}/watchers/{}/spec", self.base_url, name), true)
            }
            async fn get_watcher_statuses(&self, name: &str) -> Result<Vec<(DateTime<Local>, Status)>, ApiError> {
                api_query!(get, format!("{}/watchers/{}/statuses", self.base_url, name), true)
            }
            async fn get_watcher_status(&self, name: &str, n: u32) -> Result<Status, ApiError> {
                api_query!(get, format!("{}/watchers/{}/statuses/{}", self.base_url, name, n), true)
            }
        }
    };
    gen.into()
}

#[proc_macro_derive(WriteApi)]
pub fn write_api_derive(input: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();
    let name = &ast.ident;
    let gen = quote! {
        impl WriteApi for #name {
            async fn delete_watcher(&self, name: &str) -> Result<(), ApiError> {
                api_query!(delete, format!("{}/watchers/{}", self.base_url, name), false)
            }
            async fn post_watcher_spec(&self, name: &str, spec: Spec) -> Result<(), ApiError> {
                api_query!(post, format!("{}/watchers/{}/spec", self.base_url, name), false, spec)
            }
            async fn put_watcher_spec(&self, name: &str, spec: Spec) -> Result<(), ApiError> {
                api_query!(put, format!("{}/watchers/{}/spec", self.base_url, name), false, spec)
            }
            async fn post_watcher_status(&self, name: &str, status: Status) -> Result<(), ApiError> {
                api_query!(post, format!("{}/watchers/{}/statuses", self.base_url, name), false, status)
            }
        }
    };
    gen.into()
}

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
        Ok(ApiQuery {
            method,
            url,
            data,
            get_json,
        })
    }
}

/// Make an API query.
/// Arguments:
/// - method: The HTTP method to use (get, post, put, delete)
/// - url: The URL to query
/// - get_json: Whether to parse the response as JSON and return it
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
        .map(|data| quote! { .json(&#data) })
        .unwrap_or(quote! {});
    let gen = quote! {
        {
            let url = #url;
            let url = url.parse::<reqwest::Url>().unwrap();
            let response = self.client.#method(url)
                #data_str
                .send()
                .await?;
            #json_str
        }
    };
    gen.into()
}
