use clap::arg;
use convert_case::{Case, Casing};
use crate::class_tree::{ArgumentType, ClassEntry, ClassType, MethodEntry};
use crate::config::Config;
use crate::formatter::{escape_keywords, rename_class_fq};

#[derive(Debug)]
pub struct FormattedClassEntry {
    pub name: String,
    pub methods: Vec<FormattedMethodEntry>,
    pub class_type: ClassType,
    pub interfaces: Vec<String>,
}

impl From<ClassEntry> for FormattedClassEntry {
    fn from(original: ClassEntry) -> Self {
        let name = rename_class_fq(&original.name);
        let methods = original.methods.into_iter()
            .map(FormattedMethodEntry::from)
            .collect::<Vec<_>>();

        let interfaces = original.interfaces.into_iter()
            .map(|x| rename_class_fq(&x))
            .collect::<Vec<_>>();

        Self {
            name,
            methods,
            class_type: original.class_type,
            interfaces
        }
    }
}

#[derive(Debug)]
pub struct FormattedMethodEntry {
    pub rust_name: String,
    pub java_name: String,
    pub is_static: bool,
    pub arguments: Vec<ArgumentType>,
    pub jni_signature: String,
    pub return_type: Option<ArgumentType>,
    pub declaring_class_rust: String,
    pub declaring_class_java: String,
}

impl From<MethodEntry> for FormattedMethodEntry {
    fn from(original: MethodEntry) -> Self {
        // Casing and keywords are the only thing in need of adjustment
        let name_cased = original.name.to_case(Case::Snake);
        let rust_name = escape_keywords(&name_cased).to_string();

        let declaring_class_rust = rename_class_fq(&original.declaring_class);

        let jni_args = ArgumentType::to_jni_signature(&original.arguments);
        let jni_ret = original.return_type.as_ref().map(|x| ArgumentType::to_jni_signature(&[x.clone()]));
        let jni_signature = format!("({}){}", jni_args, jni_ret.unwrap_or("V".to_string()));

        let arguments = original.arguments.into_iter()
            .map(|x| x.format_to_rust())
            .collect::<Vec<_>>();
        let return_type = original.return_type.map(|x| x.format_to_rust());

        Self {
            rust_name,
            java_name: original.name,
            is_static: original.is_static,
            arguments,
            return_type,
            jni_signature,
            declaring_class_rust,
            declaring_class_java: original.declaring_class,
        }
    }
}

impl ArgumentType {
    fn format_to_rust(self) -> Self {
        match self {
            ArgumentType::Array(argument_type) => {
                match argument_type {
                    ArgumentType::Object(mut class_fq) => {
                        if class_fq.starts_with("[L") && class_fq.ends_with(';') {
                            class_fq.remove(0); // Remove the '['
                            class_fq.remove(0); // Remove the 'L'
                            class_fq.pop();
                        }

                        if class_fq.starts_with('[') {
                            class_fq.remove(0);
                        }

                        ArgumentType::Array(Box::new(ArgumentType::Object(class_fq)))
                    },
                    _ => ArgumentType::Array(argument_type)
                }
            },
            _ => self
        }
    }

    pub fn to_jni_signature(this: &[Self]) -> String {
        this.iter()
            .map(|this| {
                if this.is_primitive() {
                    return this.primitive_to_jni_signature();
                }

                match this {
                    Self::Object(class_fq) | Self::Array(class_fq) => {
                        let slashed = class_fq.replace('.', "/");
                        let mut cleaned = slashed;

                        if cleaned.starts_with("[L") && cleaned.ends_with(';') {
                            cleaned.remove(0);
                            cleaned.pop();

                            format!("[L{};", cleaned)
                        } else if cleaned.starts_with('[') {
                            cleaned
                        } else {
                            unreachable!();
                        }
                    },
                    Self::Array(class_fq) => {
                        Self::to_jni_signature(&[**class_fq])
                    },
                    _ => unreachable!(),
                }
            })
            .collect::<String>()
    }

    fn is_primitive(&self) -> bool {
        match self {
            Self::Object(_) | Self::Array(_) => false,
            _ => true,
        }
    }

    fn primitive_to_jni_signature(&self) -> String {
        match self {
            Self::Int => "I".into(),
            Self::Byte => "B".into(),
            Self::Double => "D".into(),
            Self::Float => "F".into(),
            Self::Short => "S".into(),
            Self::Char => "C".into(),
            Self::Boolean => "Z".into(),
            Self::Long => "J".into(),
            Self::Object(_) | Self::Array(_) => panic!("Not a primitive")
        }
    }

    pub fn to_rust_type(&self, config: &Config) -> String {
        match self {
            Self::Int => "i32".into(),
            Self::Byte => "u8".into(),
            Self::Double => "f64".into(),
            Self::Float => "f32".into(),
            Self::Short => "i16".into(),
            Self::Char => "u16".into(),
            Self::Boolean => "bool".into(),
            Self::Long => "i64".into(),
            Self::Object(class_fq) => {
                // Remove charactes Java puts in
                let class_fq = class_fq
                    .replace('[', "")
                    .replace("[L", "")
                    .replace(';', "");

                // Name is now rust safe
                let renamed = rename_class_fq(&class_fq);

                // Convert to a Rust type path
                let type_path = renamed.replace('.', "::");

                // Try to map the path to a configured mapping
                let mapped = if let Some(mapping) = config.generator.mappings.get(&type_path) {
                    mapping.to_owned()
                } else {
                    type_path
                };

                match self {
                    Self::Object(_) => mapped,
                    Self::Array(_) => format!("Vec<{}>", mapped),
                    _ => unreachable!()
                }
            },
            Self::Array(argument_type) => {
                argument_type.to_rust_type(config)
            },
         }
    }
}