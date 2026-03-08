use proc_macro::TokenStream;

mod gen_icons_impl;
mod inspector_impl;
mod widget_impl;

#[proc_macro]
pub fn generate_icons(input: TokenStream) -> TokenStream {
    gen_icons_impl::generate_icons_impl(input)
}

#[proc_macro_derive(Widget, attributes(widget, builder))]
pub fn widget_derive(input: TokenStream) -> TokenStream {
    widget_impl::widget_derive_impl(input)
}

#[proc_macro_derive(ShowInInspector, attributes(property))]
pub fn show_in_inspector_derive(input: TokenStream) -> TokenStream {
    inspector_impl::show_in_inspector_derive_impl(input)
}
