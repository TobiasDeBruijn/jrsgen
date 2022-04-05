use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::quote;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use crate::class_tree::{ClassEntry, ClassType};
use crate::generator::class::{generate_class, generate_interface};
use crate::generator::method::generate_method;
use crate::JResult;

mod class;
mod method;

pub fn format_name(x: &str) -> &str {
    match x {
        "impl" => "impl_k",
        "move" => "move_k",
        "in" => "in_k",
        _ => x
    }
}

pub fn generate(tree: Vec<ClassEntry>) -> JResult<()> {
    let base_dir = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/output/src/bindings"));

    tree.into_iter()
        .try_for_each(|mut class| {
            let mut components = class.name.split('.').into_iter()
                .map(format_name)
                .collect::<Vec<_>>();

            let mut name = components.pop().unwrap().to_string();
            let mut dir = base_dir.join(components.join("/"));

            println!("Handling: {name}");

            // We're dealing with a subclass
            if name.contains('$') {
                let parts = name.split('$').collect::<Vec<_>>();

                let parent_class = parts.first().unwrap().to_string();
                let parent_class_as_package = format!("{}_d", parent_class.to_case(Case::Snake));

                dir.push(&parent_class_as_package);

                // The child class becomes the name
                name = parts.last().unwrap().to_string();

                // TODO this needs to be handled better
                // This is to quickly avoid nested Enums,
                // they cause trouble
                if name.parse::<i32>().is_ok() {
                    return Ok(());
                }

                // Fix the ident in the Class too
                class.name = class.name
                    .replace(&parent_class, &parent_class_as_package)
                    .replace('$', ".");
            }

            if !dir.exists() {
                fs::create_dir_all(&dir)?;
            }

            let path = dir.join(format!("{}.rs", name));
            let mut file = File::create(&path)?;

            let tokens = generate_entry(&class);
            let stringified = tokens.to_string();

            let formatted = format_tokens(stringified)?;
            file.write_all(formatted.as_bytes())?;

            Result::<(), anyhow::Error>::Ok(())
        })?;

    Ok(())
}

fn format_tokens(input: String) -> JResult<String> {
    let mut command = Command::new("rustfmt")
        .arg("--emit")
        .arg("stdout")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    let stdin = command.stdin.as_mut().unwrap();
    stdin.write_all(input.as_bytes())?;
    drop(stdin);

    let output = command.wait_with_output()?;
    let stdout = output.stdout;
    let stdout = String::from_utf8(stdout)?;
    Ok(stdout)
}

fn generate_entry(class: &ClassEntry) -> TokenStream {
    let (class_tokens, class_ident) = match class.class_type {
        ClassType::Class => generate_class(class),
        ClassType::Interface => generate_interface(class),
        ClassType::Annotation => return quote! {},
    };

    let methods = class.methods.iter()
        .map(generate_method)
        .collect::<Vec<_>>();

    quote! {
        #class_tokens

        impl<'a> #class_ident<'a> {
            #(#methods)*
        }
    }
}