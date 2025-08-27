mod derive;

use proc_macro::TokenStream;

use crate::derive::handle_derive_bit;

#[proc_macro_derive(Bit)]
pub fn derive_bit(input: TokenStream) -> TokenStream {
    handle_derive_bit(input)
}
