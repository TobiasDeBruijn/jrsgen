use ejni::{Class, JavaString, Object};
use jni::JNIEnv;
use jni::objects::JValue;
use log::trace;
use crate::JResult;
use crate::parser::guava::ClassPath;

#[derive(Debug)]
pub struct ClassEntry {
    pub name: String,
    pub class_type: ClassType,
    pub methods: Vec<MethodEntry>,
    pub interfaces: Vec<String>,
}

#[derive(Debug)]
pub enum ClassType {
    Class,
    Interface,
    Annotation
}

impl ClassType {
    fn new(env: &JNIEnv<'_>, class: &Class<'_>) -> JResult<Self> {
        let is_interface = env.call_method(class.class.into_inner(), "isInterface", "()Z", &[])?.z()?;
        let is_annotation = env.call_method(class.class.into_inner(), "isAnnotation", "()Z", &[])?.z()?;

        if is_interface {
            Ok(Self::Interface)
        } else if is_annotation {
            Ok(Self::Annotation)
        } else {
            Ok(Self::Class)
        }
    }
}

pub fn build(env: &JNIEnv<'_>, root: String) -> JResult<Vec<ClassEntry>> {
    let classpath = ClassPath::new(env)?;
    let classes = classpath.get_all_classes()?;
    let classes = classes.into_iter()
        .map(|x| Ok((x.get_name()?, x)))
        .collect::<JResult<Vec<_>>>()?
        .into_iter()
        .filter(|(x, _)| x.starts_with(&root))
        .map(|(_, x)| x)
        .collect::<Vec<_>>();

    trace!("Found {} classes as subclass of package {}", classes.len(), root);

    let class_entries = classes.into_iter()
        .map(|class| {
            let name = class.get_name()?;
            trace!("Exploring class {}", name);
            let class_type = ClassType::new(env, &class)?;

            let methods = get_methods(env, &class)?;
            trace!("Found {} methods for {}", methods.len(), name);

            let interfaces = env.call_method(class.class.into_inner(), "getInterfaces", "()[Ljava/lang/Class;", &[])?.l()?;
            let len = env.get_array_length(interfaces.into_inner())?;
            let interfaces = (0..len).into_iter()
                .map(|idx| Ok(env.get_object_array_element(interfaces.into_inner(), idx)?))
                .collect::<JResult<Vec<_>>>()?
                .into_iter()
                .map(|object| Ok(env.call_method(object, "getName", "()Ljava/lang/String;", &[])?.l()?))
                .collect::<JResult<Vec<_>>>()?
                .into_iter()
                .map(|string_object| Ok(JavaString::new(env, Object::new(env, string_object, Class::String(env)?)).into_rust()?))
                .collect::<JResult<Vec<_>>>()?;

            Ok(ClassEntry {
                name,
                class_type,
                methods,
                interfaces,
            })
        })
        .collect::<JResult<Vec<_>>>()?;

    Ok(class_entries)
}

#[derive(Debug)]
pub struct MethodEntry {
    pub name: String,
    pub is_static: bool,
    pub arguments: Vec<ArgumentType>,
    pub return_type: Option<ArgumentType>,
    pub declaring_class: String,
}

impl MethodEntry {
    pub fn new(env: &JNIEnv<'_>, method: Object<'_>) -> JResult<Self> {
        let name = env.call_method(method.inner, "getName", "()Ljava/lang/String;", &[])?.l()?;
        let name = JavaString::new(env, Object::new(env, name, Class::String(env)?)).into_rust()?;

        trace!("Analyzing method {}", name);

        let modifiers = env.call_method(method.inner, "getModifiers", "()I", &[])?.i()?;
        let is_static = env.call_static_method("java/lang/reflect/Modifier", "isStatic", "(I)Z", &[JValue::Int(modifiers)])?.z()?;

        let parameter_classes_array = env.call_method(method.inner, "getParameterTypes", "()[Ljava/lang/Class;", &[])?.l()?;
        let len = env.get_array_length(parameter_classes_array.into_inner())?;
        let arguments = (0..len).into_iter()
            .map(|idx| Ok(env.get_object_array_element(parameter_classes_array.into_inner(), idx)?))
            .collect::<JResult<Vec<_>>>()?
            .into_iter()
            .map(|object| {
                let class_name = env.call_method(object, "getName", "()Ljava/lang/String;", &[])?.l()?;
                let class_name = JavaString::new(env, Object::new(env, class_name, Class::String(env)?)).into_rust()?;
                let argument = ArgumentType::new(&env, class_name)?;
                Ok(argument)
            })
            .collect::<JResult<Vec<_>>>()?;

        trace!("Found {} arguments for method {}", arguments.len(), name);

        let declaring_class = env.call_method(method.inner, "getDeclaringClass", "()Ljava/lang/Class;", &[])?.l()?;
        let declaring_class = env.call_method(declaring_class, "getName", "()Ljava/lang/String;", &[])?.l()?;
        let declaring_class = JavaString::new(env, Object::new(env, declaring_class, Class::String(env)?)).into_rust()?;

        let return_type = env.call_method(method.inner, "getReturnType", "()Ljava/lang/Class;", &[])?.l()?;
        let ret_name = env.call_method(return_type, "getName", "()Ljava/lang/String;", &[])?.l()?;
        let ret_name = JavaString::new(env, Object::new(env, ret_name, Class::String(env)?)).into_rust()?;

        let return_type = match ret_name.as_str() {
            "boolean" => Some(ArgumentType::Boolean),
            "int" => Some(ArgumentType::Int),
            "float" => Some(ArgumentType::Float),
            "short" => Some(ArgumentType::Short),
            "double" => Some(ArgumentType::Double),
            "byte" => Some(ArgumentType::Byte),
            "long" => Some(ArgumentType::Long),
            "char" => Some(ArgumentType::Char),
            "void" => None,
            _ => {
                let class = Class::for_name(env, &ret_name)?;
                let is_array = env.call_method(class.class.into_inner(), "isArray", "()Z", &[])?.z()?;
                if is_array {
                    Some(ArgumentType::Array(Box::new(ArgumentType::Object(ret_name))))
                } else {
                    Some(ArgumentType::Object(ret_name))
                }
            }
        };

        Ok(Self {
            name,
            is_static,
            arguments,
            return_type,
            declaring_class,
        })
    }
}

#[derive(Debug, Clone)]
pub enum ArgumentType {
    Boolean,
    Byte,
    Char,
    Short,
    Int,
    Long,
    Float,
    Double,
    Object(String),
    Array(Box<ArgumentType>),
}

impl ArgumentType {
    pub fn from_signature_type(i: &str) -> Option<Self> {
        match i {
            "B" => Some(Self::Byte),
            "Z" => Some(Self::Boolean),
            "J" => Some(Self::Long),
            "I" => Some(Self::Int),
            "F" => Some(Self::Float),
            "D" => Some(Self::Double),
            "S" => Some(Self::Short),
            "C" => Some(Self::Char),
            _ => None
        }
    }

    pub fn new(env: &JNIEnv<'_>, name: String) -> JResult<ArgumentType> {
        match name.as_str() {
            "boolean" => Ok(Self::Boolean),
            "byte" => Ok(Self::Byte),
            "char" => Ok(Self::Char),
            "short" => Ok(Self::Short),
            "int" => Ok(Self::Int),
            "long" => Ok(Self::Long),
            "float" => Ok(Self::Float),
            "double" => Ok(Self::Double),
            _ => {
                let class = Class::for_name(env, &name)?;
                let is_array = env.call_method(class.class.into_inner(), "isArray", "()Z", &[])?.z()?;
                if is_array {
                    let this = Self::from_signature_type(&name).unwrap_or(ArgumentType::Object(name));
                    Ok(Self::Array(Box::new(this)))
                } else {
                    Ok(Self::Object(name))
                }
            }
        }
    }
}

fn get_methods(env: &JNIEnv<'_>, class: &Class<'_>) -> JResult<Vec<MethodEntry>> {
    let methods = env.call_method(class.class.into_inner(), "getDeclaredMethods", "()[Ljava/lang/reflect/Method;", &[])?.l()?;
    let len = env.get_array_length(methods.into_inner())?;
    let methods = (0..len).into_iter()
        .map(|idx| Ok(env.get_object_array_element(methods.into_inner(), idx)?))
        .collect::<JResult<Vec<_>>>()?
        .into_iter()
        .map(|object| Ok(Object::new(env, object, Class::Method(env)?)))
        .collect::<JResult<Vec<_>>>()?
        .into_iter()
        .map(|object| Ok(MethodEntry::new(env, object)?))
        .collect::<JResult<Vec<_>>>()?;

    Ok(methods)
}