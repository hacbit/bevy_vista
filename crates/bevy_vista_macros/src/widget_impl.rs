use proc_macro::TokenStream;
use quote::quote;
use syn::{
    DeriveInput, Ident, LitStr, Path, Token, parse::Parse, parse::ParseStream, parse_macro_input,
};

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
    let impl_get_widget_registration_trait = impl_get_widget_registration_trait(
        &input,
        category,
        name,
        &builder_path,
        true,
        widget_info.children.as_deref(),
        widget_info.slots.as_deref(),
    );
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
    children: Option<String>,
    slots: Option<String>,
}

impl Parse for WidgetAttrInfo {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path = input.parse::<LitStr>()?;
        let mut children = None;
        let mut slots = None;

        while input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            let key = input.parse::<Ident>()?;
            input.parse::<Token![=]>()?;
            let value = input.parse::<LitStr>()?.value();
            match key.to_string().as_str() {
                "children" => {
                    children = Some(value);
                }
                "slots" => {
                    slots = Some(value);
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        key,
                        "unsupported widget metadata key, expected `children` or `slots`",
                    ));
                }
            }
        }

        Ok(Self {
            path,
            children,
            slots,
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
    supports_inspector: bool,
    children: Option<&str>,
    slots: Option<&str>,
) -> proc_macro2::TokenStream {
    let ty = &input.ident;
    let generics = &input.generics;
    let base_registration = if supports_inspector {
        quote! {
            bevy_vista::widget::WidgetRegistration::of_with_inspector::<Self, #builder_path>(#category, #name)
        }
    } else {
        quote! {
            bevy_vista::widget::WidgetRegistration::of::<Self, #builder_path>(#category, #name)
        }
    };

    let child_rule = children
        .map(parse_child_rule_tokens)
        .unwrap_or_else(|| quote! { bevy_vista::widget::WidgetChildRule::Any });

    let slots_tokens = slots
        .map(parse_slots_tokens)
        .unwrap_or_else(|| quote! { &[] });

    let registration = quote! {
        #base_registration
            .child_rule_config(#child_rule)
            .child_slots(#slots_tokens)
    };
    quote! {
        impl #generics bevy_vista::widget::GetWidgetRegistration for #ty #generics {
            fn get_widget_registration() -> bevy_vista::widget::WidgetRegistration {
                #registration
            }
        }
    }
}

fn parse_child_rule_tokens(raw: &str) -> proc_macro2::TokenStream {
    let value = raw.trim().to_ascii_lowercase();
    if value == "any" {
        return quote! { bevy_vista::widget::WidgetChildRule::Any };
    }
    if let Some(inner) = value
        .strip_prefix("exact(")
        .and_then(|s| s.strip_suffix(')'))
    {
        let n = inner
            .trim()
            .parse::<usize>()
            .unwrap_or_else(|_| panic!("invalid children metadata, expected exact(<usize>)"));
        return quote! { bevy_vista::widget::WidgetChildRule::Exact(#n) };
    }
    if let Some(inner) = value.strip_prefix("max(").and_then(|s| s.strip_suffix(')')) {
        let n = inner
            .trim()
            .parse::<usize>()
            .unwrap_or_else(|_| panic!("invalid children metadata, expected max(<usize>)"));
        return quote! { bevy_vista::widget::WidgetChildRule::Range { max: Some(#n) } };
    }

    panic!("invalid children metadata, use one of: any | exact(<usize>) | max(<usize>)");
}

fn parse_slots_tokens(raw: &str) -> proc_macro2::TokenStream {
    let items = raw
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    let lits = items
        .iter()
        .map(|slot| LitStr::new(slot, proc_macro2::Span::call_site()))
        .collect::<Vec<_>>();
    quote! { &[#(#lits),*] }
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
