extern crate proc_macro;
#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use syn::{parse_macro_input, ItemFn};

/// Run a future as a test, this expands to calling the `async fn` via
/// `futures::executor::block_on`. Based off the `tokio-async-await-test`
/// crate.
#[proc_macro_attribute]
pub fn async_test(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemFn);
    let test_case_name = input.ident.clone();

    let expanded = quote! {
        #[test]
        fn #test_case_name () {
            use futures::{executor::block_on, future::{FutureExt, TryFutureExt}};

            #input

            block_on(#test_case_name()).unwrap();
        }
    };
    expanded.into()
}
