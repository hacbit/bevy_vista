use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Data, DeriveInput, Expr, Field, Fields, LitBool, LitFloat, LitStr, Result, Token, parse::Parse,
    parse::ParseStream, parse_macro_input,
};

pub fn show_in_inspector_derive_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ty = &input.ident;

    let Data::Struct(data) = &input.data else {
        panic!("ShowInInspector can only be derived for structs");
    };
    let Fields::Named(fields) = &data.fields else {
        panic!("ShowInInspector requires named fields");
    };

    let metadata_entries = fields
        .named
        .iter()
        .filter_map(field_metadata_tokens)
        .collect::<Vec<_>>();

    quote! {
        impl bevy_vista::inspector::ShowInInspector for #ty {
            fn inspector_fields() -> ::std::vec::Vec<bevy_vista::inspector::InspectorFieldMetadata> {
                vec![
                    #(#metadata_entries),*
                ]
            }
        }
    }
    .into()
}

fn field_metadata_tokens(field: &Field) -> Option<proc_macro2::TokenStream> {
    let ident = field.ident.as_ref()?;
    let property_attr = field
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("property"))?;
    let property = property_attr
        .parse_args::<PropertyArgs>()
        .unwrap_or_else(|err| panic!("invalid #[property(...)] on `{ident}`: {err}"));
    if property.header.is_none() && property.header_default_open.is_some() {
        panic!("`default_open` requires `header = \"...\"` on `{ident}`");
    }

    let field_name = ident.to_string();
    let mut options = quote! { bevy_vista::inspector::InspectorFieldOptions::default() };

    if let Some(label) = property.label {
        options = quote! { #options.label(#label) };
    }
    if let Some(editor) = property.editor {
        let editor_tokens = editor_tokens(&editor.value());
        options = quote! { #options.editor(#editor_tokens) };
    }
    if property.hidden {
        options = quote! { #options.hidden(true) };
    }
    if let Some(min) = property.min {
        options = quote! { #options.numeric_min(#min) };
    }
    if let Some(header) = property.header {
        let title = header.value();
        let default_open = property.header_default_open.unwrap_or(true);
        options = quote! {
            #options.header_with_options(
                bevy_vista::inspector::InspectorHeaderOptions::new(#title)
                    .default_open(#default_open)
            )
        };
    }
    if property.end_header {
        options = quote! { #options.end_header(true) };
    }

    Some(quote! {
        bevy_vista::inspector::InspectorFieldMetadata {
            field_name: #field_name,
            options: #options,
        }
    })
}

fn editor_tokens(editor: &str) -> proc_macro2::TokenStream {
    match editor {
        "f32" => quote! {
            bevy_vista::inspector::InspectorResolvedEditor::Number(
                bevy_vista::inspector::InspectorNumberAdapter::F32
            )
        },
        "val_px" => quote! {
            bevy_vista::inspector::InspectorResolvedEditor::Val(
                bevy_vista::inspector::InspectorValAdapter::Val
            )
        },
        "ui_rect_all" => quote! {
            bevy_vista::inspector::InspectorResolvedEditor::Number(
                bevy_vista::inspector::InspectorNumberAdapter::UiRectAll
            )
        },
        "bool" => quote! {
            bevy_vista::inspector::InspectorResolvedEditor::Bool(
                bevy_vista::inspector::InspectorBoolAdapter::Bool
            )
        },
        "visibility" => quote! {
            bevy_vista::inspector::InspectorResolvedEditor::Bool(
                bevy_vista::inspector::InspectorBoolAdapter::Visibility
            )
        },
        "unit_enum" => quote! {
            bevy_vista::inspector::InspectorResolvedEditor::Choice(
                bevy_vista::inspector::InspectorChoiceAdapter::UnitEnum
            )
        },
        "color_preset" => quote! {
            bevy_vista::inspector::InspectorResolvedEditor::Choice(
                bevy_vista::inspector::InspectorChoiceAdapter::ColorPreset
            )
        },
        other => panic!("unknown inspector editor `{other}`"),
    }
}

#[derive(Default)]
struct PropertyArgs {
    label: Option<LitStr>,
    editor: Option<LitStr>,
    min: Option<Expr>,
    header: Option<LitStr>,
    header_default_open: Option<bool>,
    hidden: bool,
    end_header: bool,
}

impl Parse for PropertyArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut args = PropertyArgs::default();
        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            if ident == "hidden" {
                args.hidden = true;
            } else if ident == "end_header" {
                args.end_header = true;
            } else {
                input.parse::<Token![=]>()?;
                match ident.to_string().as_str() {
                    "label" => args.label = Some(input.parse::<LitStr>()?),
                    "editor" => args.editor = Some(input.parse::<LitStr>()?),
                    "header" => args.header = Some(input.parse::<LitStr>()?),
                    "default_open" => {
                        args.header_default_open = Some(input.parse::<LitBool>()?.value)
                    }
                    "min" => {
                        let value = input.parse::<LitFloat>()?;
                        args.min = Some(syn::parse_quote!(#value));
                    }
                    _ => return Err(input.error("unsupported property attribute")),
                }
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(args)
    }
}
