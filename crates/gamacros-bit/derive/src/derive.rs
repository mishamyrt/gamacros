use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields};

pub fn handle_derive_bit(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    // Collect variant idents in declared order
    let variants: Vec<syn::Ident> = match input.data {
        Data::Enum(e) => e
            .variants
            .into_iter()
            .map(|v| match v.fields {
                Fields::Unit => v.ident,
                _ => panic!("Bit supports only fieldless enum variants"),
            })
            .collect(),
        _ => panic!("Bit can be derived only for enums"),
    };

    // Assign discriminants implicitly by index and generate a bit() method
    let arms = variants.iter().enumerate().map(|(i, v)| {
        let idx = i as u64;
        quote! { #name::#v => 1u64 << #idx }
    });

    let expanded = quote! {
        use gamacros_bit_mask::Bitable;
        impl Bitable for #name {
            #[inline]
            fn bit(&self) -> u64 {
                match self { #( #arms, )* }
            }

            #[inline]
            fn index(&self) -> u32 { self.bit().trailing_zeros() }
        }
    };

    TokenStream::from(expanded)
}
