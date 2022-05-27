use convert_case::{Case, Casing};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use crate::class_tree::ClassEntry;
use crate::formatter::rename;

pub fn generate_interface(class: &ClassEntry) -> (TokenStream, Ident) {
    let name_ident = format_ident!("{}", class.name.split(".").last().unwrap());

    let tokens = quote! {
        pub trait #name_ident {}
    };

    (tokens, name_ident)
}

fn generate_interface_impl(name_ident: &Ident, interface: &Ident) -> TokenStream {
    quote! {
        impl<'a> #interface for #name_ident<'a> {}
    }
}

fn generate_struct(name_ident: &Ident) -> TokenStream {
    quote! {
        pub struct #name_ident<'a> {
            env: &'a jni::JNIEnv<'a>,
            obj: ejni::Object<'a>,
        }
    }
}

fn generate_struct_trait_impls(name_ident: &Ident, fully_qualified_class_name: &str) -> TokenStream {
    quote! {
        impl<'a> crate::ClassName for #name_ident<'a> {
            fn class_name() -> &'static str {
                #fully_qualified_class_name
            }
        }

        impl<'a> crate::FromRaw<'a> for #name_ident<'a> {
            fn from_raw(env: &'a jni::JNIEnv<'a>, obj: ejni::Object<'a>) -> Self {
                Self {
                    env,
                    obj
                }
            }
        }

        impl<'a> Into<jni::JValue<'a>> for #name_ident<'a> {
            fn into(self) -> jni::JValue<'a> {
                self.obj.into()
            }
        }
    }
}

pub fn generate_class(class: &ClassEntry) -> (TokenStream, Ident) {
    println!("{}", class.name);

    let compatible_name = rename(&class.name);

    let name_ident = format_ident!("{}", compatible_name.split('.').last().unwrap());
    let fully_qualified_class_path = class.name.replace('.', "/");

    let gen_struct = generate_struct(&name_ident);
    let trait_impls = generate_struct_trait_impls(&name_ident, &fully_qualified_class_path);
    let interfaces = class.interfaces.iter()
        .map(|x| {
            let name_compatible = rename(x);
            let mut name = name_compatible.split('.').last().unwrap();

            format_ident!("{}", name)
        })
        .map(|x| generate_interface_impl(&name_ident, &x))
        .collect::<Vec<_>>();

    let tokens = quote! {
        #gen_struct

        #trait_impls

        #(#interfaces)*
    };

    (tokens, name_ident)
}
