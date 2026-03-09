use std::fs;

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Attribute, Ident, LitInt, LitStr, Token, parse::Parse, parse_macro_input, parse_str,
    punctuated::Punctuated,
};

pub fn generate_icons_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as GenerateIconsInput);
    let mut struct_meta = input.meta;
    let Some(icons_data_attr) = struct_meta.pop_if(|a| a.path().is_ident("icons_data")) else {
        panic!("Expected #[icons_data(\"...\")]");
    };
    let file_path = icons_data_attr
        .parse_args::<LitStr>()
        .expect("Expected str literal")
        .value();
    let fields = match fs::read_to_string(file_path) {
        Ok(content) => parse_str::<Fields>(&content).unwrap(),
        Err(e) => panic!("read file error: {}", e),
    };

    let mgr_id = input.mgr_ident;
    let icons_id = input.icons_ident;
    let enum_meta = input.meta2;

    let names = fields
        .fields
        .iter()
        .map(|f| f.name.clone())
        .collect::<Vec<Ident>>();
    let const_strs = fields.fields.iter().map(|f| {
        let name = &f.name;
        let data = &f.str;
        quote! {
            const #name: &str = #data;
        }
    });

    quote! {
        #(#struct_meta)*
        pub struct #mgr_id {
            handles: std::collections::HashMap<#icons_id, bevy::asset::Handle<bevy::image::Image>>,
        }

        #( #const_strs )*

        #[derive(bevy::reflect::Reflect)]
        #(#enum_meta)*
        pub enum #icons_id {
            #( #names ),*
        }

        impl #icons_id {
            fn to_raw_data(self) -> &'static str {
                match self {
                    #(
                        Self::#names => #names
                    ),*
                }
            }

            pub fn reflected_variants() -> Vec<(&'static str, Self)> {
                let bevy::reflect::TypeInfo::Enum(enum_info) =
                    <Self as bevy::reflect::Typed>::type_info()
                else {
                    return Vec::new();
                };

                enum_info
                    .iter()
                    .enumerate()
                    .filter_map(|(index, variant)| {
                        if variant.variant_type() != bevy::reflect::VariantType::Unit {
                            return None;
                        }

                        let mut dynamic = bevy::reflect::DynamicEnum::new_with_index(
                            index,
                            variant.name(),
                            bevy::reflect::DynamicVariant::Unit,
                        );
                        dynamic.set_represented_type(Some(<Self as bevy::reflect::Typed>::type_info()));

                        <Self as bevy::reflect::FromReflect>::from_reflect(&dynamic)
                            .map(|value| (variant.name(), value))
                    })
                    .collect()
            }
        }
    }
    .into()
}

struct GenerateIconsInput {
    meta: Vec<Attribute>,
    _struct: Token![struct],
    mgr_ident: Ident,
    meta2: Vec<Attribute>,
    _enum: Token![enum],
    icons_ident: Ident,
    // fields: Punctuated<Field, Token![,]>,
    // others for enum
}

impl Parse for GenerateIconsInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            meta: Attribute::parse_outer(input)?,
            _struct: input.parse::<Token![struct]>()?,
            mgr_ident: input.parse::<Ident>()?,
            meta2: Attribute::parse_outer(input)?,
            _enum: input.parse::<Token![enum]>()?,
            icons_ident: input.parse::<Ident>()?,
        })
    }
}

struct Fields {
    fields: Punctuated<Field, Token![,]>,
}

impl Parse for Fields {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            fields: input.parse_terminated(Field::parse, Token![,])?,
        })
    }
}

struct Field {
    name: Ident,
    _width: LitInt,
    _height: LitInt,
    str: LitStr,
}

impl Parse for Field {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            name: input.parse::<Ident>()?,
            _width: input.parse::<LitInt>()?,
            _height: input.parse::<LitInt>()?,
            str: input.parse::<LitStr>()?,
        })
    }
}
