use std::str::FromStr;
use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use crate::class_tree::{ArgumentType, MethodEntry};
use crate::formatter::rename;

pub fn generate_method(method: &MethodEntry) -> TokenStream {
    // Filter out lamdas and other things
    if method.name.contains("lambda$") || method.name.contains('$') {
        return quote! {};
    }

    for arg in &method.arguments {
        match arg {
            ArgumentType::Object(object) => {
                if object.contains("lambda$") {
                    return quote! {};
                }
            },
            _ => {}
        }
    }

    if method.is_static {
        generate_static(method)
    } else {
        generate_associated(method)
    }
}

fn generate_static(method: &MethodEntry) -> TokenStream {
    let name_snake = method.name.to_case(Case::Snake);
    let formatted = rename(&name_snake);

    let name_snake_ident = format_ident!("{}", formatted);
    let arguments = generate_rust_arguments(method);
    let return_type = generate_return_type(&method.return_type);
    let jvalues = generate_jvalue_arguments(method, false);

    let java_name = &method.name;
    let class_name = method.declaring_class.replace('.', "/");

    let method_signature = generate_signature(method);
    let jvalue_array = generate_jvalue_array(method);
    let return_handler = generate_return_handler(method);

    quote! {
        pub fn #name_snake_ident(env: &'a jni::JNIEnv<'a>, #arguments) -> #return_type {
            #jvalues
            let jvalue = env.call_static_method(#class_name, #java_name, #method_signature, #jvalue_array)?;
            #return_handler
        }
    }
}

fn generate_associated(method: &MethodEntry) -> TokenStream {
    let name_snake = method.name.to_case(Case::Snake);
    let name_formatted = format_name(&name_snake);

    let name_snake_ident = format_ident!("{}", name_formatted);
    let arguments = generate_rust_arguments(method);
    let return_type = generate_return_type(&method.return_type);
    let jvalues = generate_jvalue_arguments(method, true);

    let java_name = &method.name;
    let method_signature = generate_signature(method);
    let jvalue_array = generate_jvalue_array(method);
    let return_handler = generate_return_handler(method);

    quote! {
        pub fn #name_snake_ident(&self, #arguments) -> #return_type {
            #jvalues
            let jvalue = self.env.call_method(self.obj.inner, #java_name, #method_signature, #jvalue_array)?;
            #return_handler
        }
    }
}

fn generate_return_handler(method: &MethodEntry) -> TokenStream {
    if let Some(return_type) = &method.return_type {
        let value = match return_type {
            ArgumentType::Boolean => quote! {
                let value = jvalue.z()?;
            },
            ArgumentType::Byte => quote! {
                let value = jvalue.b()?;
            },
            ArgumentType::Char => quote! {
                let value = jvalue.c()?;
            },
            ArgumentType::Short => quote! {
                let value = jvalue.s()?;
            },
            ArgumentType::Int => quote! {
                let value = jvalue.i()?;
            },
            ArgumentType::Long => quote! {
                let value = jvalue.j()?;
            },
            ArgumentType::Float => quote! {
                let value = jvalue.f()?;
            },
            ArgumentType::Double => quote! {
                let value = jvalue.d()?;
            },
            ArgumentType::Object(object) => quote! {
                let value = jvalue.l()?;
                let value = object.
            },
            ArgumentType::Array(class_name) => quote! {
                todo!("Array type for class {}", #class_name);
            },
        };

        quote! {
            #value
            Ok(value)
        }
    } else {
        quote! {
            Ok(())
        }
    }
}

fn generate_jvalue_array(method: &MethodEntry) -> TokenStream {
    let tokens = method.arguments.iter().enumerate()
        .map(|(idx, _)| {
            let ident = format_ident!("arg{}", idx);
            quote! { #ident }
        })
        .collect::<Vec<_>>();

    quote! {
        &[ #(#tokens),* ]
    }
}

fn argument_type_to_signature(argument_type: &ArgumentType) -> String {
    match argument_type {
        ArgumentType::Boolean => "Z".to_string(),
        ArgumentType::Byte => "B".to_string(),
        ArgumentType::Char => "C".to_string(),
        ArgumentType::Short => "S".to_string(),
        ArgumentType::Int => "I".to_string(),
        ArgumentType::Long => "J".to_string(),
        ArgumentType::Float => "F".to_string(),
        ArgumentType::Double => "D".to_string(),
        ArgumentType::Object(name) => {
            let name = name.split('.')
                .map(format_name)
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(".");

            let name_slashed = name.replace('.', "/");
            format!("L{};", name_slashed)
        },
        ArgumentType::Array(name) => {
            let name = name.split('.')
                .into_iter()
                .map(format_name)
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(".");

            let name_slashed = name.replace('.', "/");
            format!("[L{};", name_slashed)
        }
    }
}

fn generate_signature(method: &MethodEntry) -> String {
    let arguments = method.arguments.iter()
        .map(argument_type_to_signature)
        .collect::<String>();
    let ret = if let Some(ret) = &method.return_type {
        argument_type_to_signature(ret)
    } else {
        "V".to_string()
    };

    format!("({}){}", arguments, ret)
}

fn generate_jvalue_arguments(method: &MethodEntry, associated_method: bool) -> TokenStream {
    let env = if associated_method {
        quote! {
            let env = self.env;
        }
    } else {
        quote! {}
    };

    let tokens = method.arguments.iter().enumerate()
        .map(|(idx, argument_type)| {
            let arg_name = format_ident!("arg{}", idx);

            match argument_type {
                ArgumentType::Byte => quote! {
                    let #arg_name = jni::JValue::Byte(#arg_name as i8);
                },
                ArgumentType::Boolean => quote! {
                    let #arg_name = jni::JValue::Bool(if #arg_name { 1 } else { 0 });
                },
                ArgumentType::Int => quote! {
                    let #arg_name = jni::JValue::Int(#arg_name);
                },
                ArgumentType::Long => quote! {
                    let #arg_name = jni::JValue::Long(#arg_name);
                },
                ArgumentType::Double => quote! {
                    let #arg_name = jni::JValue::Double(#arg_name);
                },
                ArgumentType::Float => quote! {
                    let #arg_name = jni::JValue::Float(#arg_name);
                },
                ArgumentType::Short => quote! {
                    let #arg_name = jni::JValue::Short(#arg_name);
                },
                ArgumentType::Char => quote! {
                    let #arg_name = jni::JValue::Char(#arg_name);
                },
                ArgumentType::Object(_) => quote! {
                    let #arg_name = #arg_name.into();
                },
                ArgumentType::Array(class_name) => quote! {
                    todo!("Yet to generate array of {}", #class_name);
                }
            }
        })
        .collect::<Vec<_>>();

    quote! {
        #env
        #(#tokens)*
    }
}

fn generate_return_type(return_type: &Option<ArgumentType>) -> TokenStream {
    if let Some(return_type) = &return_type {
        let return_type = generate_argument_type(return_type);
        quote! {
            crate::JResult<#return_type>
        }
    } else {
        quote! {
            crate::JResult<()>
        }
    }
}

fn generate_argument_type(argument_type: &ArgumentType) -> TokenStream {
    match argument_type {
        ArgumentType::Int => quote!(i32),
        ArgumentType::Char => quote!(u16),
        ArgumentType::Double => quote!(f64),
        ArgumentType::Float => quote!(f32),
        ArgumentType::Byte => quote!(u8),
        ArgumentType::Long => quote!(i64),
        ArgumentType::Short => quote!(i16),
        ArgumentType::Boolean => quote!(bool),
        ArgumentType::Array(object) => {
            match object.as_str() {
                "[Z" => quote! {
                    Vec<bool>
                },
                "[B" => quote! {
                    Vec<u8>
                },
                "[C" => quote! {
                    Vec<u16>
                },
                "[S" => quote! {
                    Vec<i16>
                },
                "[I" => quote! {
                    Vec<i32>
                },
                "[J" => quote! {
                    Vec<i64>
                },
                "[F" => quote! {
                    Vec<f32>
                },
                "[D" => quote! {
                    Vec<f64>
                },
                _ => {
                    // We're dealing with a 2D array
                    if object.contains("[[") {
                        match object.as_str() {
                            "[[I" => return quote! {
                                Vec<Vec<i32>>
                            },
                            "[[B" => return quote! {
                                Vec<Vec<u8>>
                            },
                            _ => {}
                        }
                    }

                    let object = object
                        .replace("[L", "")
                        .replace("[", "")
                        .replace(';', "")
                        .replace('.', "::")
                        .replace('$', "::");

                    let object = object.split("::")
                        .into_iter()
                        .map(format_name)
                        .map(|x| x.to_string())
                        .collect::<Vec<_>>()
                        .join("::");

                    println!("{object}");
                    let tokens = TokenStream::from_str(&object).unwrap();

                    let name_as_path = quote! {
                        crate::bindings::#tokens
                    };

                    quote! {
                        Vec<#name_as_path>
                    }
                }
            }
        }
        ArgumentType::Object(object) => {
            let object = object
                .split('.')
                .map(format_name)
                .collect::<Vec<_>>()
                .join(".")
                .replace('.', "::")
                .replace("$", "::");
            let tokens = TokenStream::from_str(&object).unwrap();

            quote! {
                crate::bindings::#tokens
            }
        }
    }
}

fn generate_rust_arguments(method: &MethodEntry) -> TokenStream {
    let tokens = method.arguments.iter().enumerate()
        .map(|(idx, arg)| {
            let ident = format_ident!("arg{}", idx);
            let ty = generate_argument_type(arg);

            quote! {
                #ident: #ty
            }
        })
        .collect::<Vec<_>>();

    quote! {
        #(#tokens),*
    }
}