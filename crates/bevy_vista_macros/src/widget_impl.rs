use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, Ident, LitStr, Path, parse::Parse, parse_macro_input};

pub fn widget_derive_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let Some(attr) = input.attrs.iter().find(|a| a.path().is_ident("widget")) else {
        panic!("Expected #[widget(\"category/name\")]");
    };

    let widget_info = attr.parse_args::<WidgetAttrInfo>().unwrap();
    let path = widget_info.path.value();
    let segs = path.split('/').collect::<Vec<&str>>();
    if segs.len() != 2 {
        panic!("widget attribute expected \"category/name\"");
    }
    let category = segs[0];
    let name = segs[1];

    let Some(attr) = input.attrs.iter().find(|a| a.path().is_ident("builder")) else {
        panic!("Expected #[builder(BuilderType)]");
    };
    let builder_info = attr.parse_args::<WidgetBuilderAttrInfo>().unwrap();
    let builder_path = builder_info.builder;

    let impl_widget_trait = impl_widget_trait(&input, category, name);
    let impl_get_widget_registration_trait =
        impl_get_widget_registration_trait(&input, category, name, &builder_path);
    let auto_register = auto_widget_registration(&input.ident);

    quote! {
        #impl_widget_trait

        #impl_get_widget_registration_trait

        #auto_register
    }
    .into()
}

struct WidgetAttrInfo {
    path: LitStr,
}

impl Parse for WidgetAttrInfo {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            path: input.parse::<LitStr>()?,
        })
    }
}

struct WidgetBuilderAttrInfo {
    builder: Path,
}

impl Parse for WidgetBuilderAttrInfo {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            builder: input.parse::<Path>()?,
        })
    }
}

fn impl_widget_trait(input: &DeriveInput, category: &str, name: &str) -> proc_macro2::TokenStream {
    let struct_ident = &input.ident;
    let generics = &input.generics;

    quote! {
        impl #generics bevy_vista::widget::Widget for #struct_ident #generics {
            fn category() -> &'static str {
                #category
            }

            fn name() -> &'static str {
                #name
            }
        }
    }
}

fn impl_get_widget_registration_trait(
    input: &DeriveInput,
    category: &str,
    name: &str,
    builder_path: &Path,
) -> proc_macro2::TokenStream {
    let ty = &input.ident;
    let generics = &input.generics;
    quote! {
        impl #generics bevy_vista::widget::GetWidgetRegistration for #ty #generics {
            fn get_widget_registration() -> bevy_vista::widget::WidgetRegistration {
                bevy_vista::widget::WidgetRegistration::of::<Self, #builder_path>(#category, #name)
            }
        }
    }
}

fn auto_widget_registration(ty: &Ident) -> proc_macro2::TokenStream {
    quote! {
        bevy_vista::widget::__macro_exports::inventory::submit! {
            bevy_vista::widget::__macro_exports::AutomaticWidgetRegistrations(
                <#ty as bevy_vista::widget::__macro_exports::RegisterForWidget>::__auto_register
            )
        }
    }
}
