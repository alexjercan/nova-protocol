//! Procedural macros for the Nova Protocol event engine.
//!
//! Nova owns this crate because it owns the event engine it serves
//! (`nova_events::engine`); both are vendored into this workspace so the engine
//! and its derive stay together. `nova_events` is its only consumer - the derive expands to `impl EventKind for #name`, so the
//! trait must be in scope at the derive site, and `nova_events::prelude`
//! re-exports the two together.
#![warn(missing_docs)]

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// Derive `EventKind` for an event marker type.
///
/// The event name defaults to the lowercased struct name and can be overridden
/// with `#[event_name("...")]`; the payload type defaults to `()` and can be
/// overridden with `#[event_info(MyPayload)]`.
#[proc_macro_derive(EventKind, attributes(event_name, event_info))]
pub fn derive_event_kind(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let name_str = name.to_string().to_lowercase();

    let mut event_name = quote! { #name_str };
    // NOTE: `()` is the default payload for an event with no `#[event_info(...)]`:
    // it satisfies the `EventKind::Info` bounds (Serialize + Default + Clone + Debug)
    // and needs no import at the derive site. Do not name a concrete type here -- the
    // original default named one that neither resolved nor implemented Serialize, so
    // the attribute-less derive never compiled (guarded by
    // `attribute_less_derive_defaults_to_no_payload` in `nova_events::engine`).
    let mut event_info = quote! { () };

    // A malformed attribute is a compile error, never a silent fallback: an
    // `#[event_name = "x"]` typo used to expand to the LOWERCASED IDENT, and
    // because dispatch matches itself nothing broke loudly - only every
    // literal-name consumer quietly stopped matching.
    for attr in &input.attrs {
        if attr.path().is_ident("event_name") {
            match attr.parse_args() {
                Ok(syn::Lit::Str(s)) => {
                    let s = s.value();
                    event_name = quote! { #s };
                }
                _ => {
                    return syn::Error::new_spanned(
                        attr,
                        "expected #[event_name(\"the-event-name\")]",
                    )
                    .to_compile_error()
                    .into()
                }
            }
        } else if attr.path().is_ident("event_info") {
            match attr.parse_args() {
                Ok(syn::TypePath { path, .. }) => event_info = quote! { #path },
                _ => {
                    return syn::Error::new_spanned(attr, "expected #[event_info(PayloadType)]")
                        .to_compile_error()
                        .into()
                }
            }
        }
    }

    let expanded = quote! {
        impl EventKind for #name {
            type Info = #event_info;

            fn name() -> &'static str {
                #event_name
            }
        }
    };

    TokenStream::from(expanded)
}
