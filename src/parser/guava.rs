use ejni::{Class, JavaString, Object, Set};
use jni::JNIEnv;
use jni::objects::JValue;
use crate::JResult;

pub struct ClassPath<'a> {
    env: &'a JNIEnv<'a>,
    obj: Object<'a>,
}

impl<'a> ClassPath<'a> {
    pub fn new(env: &'a JNIEnv<'a>) -> JResult<Self> {
        let classloader = env.call_static_method("java/lang/ClassLoader", "getSystemClassLoader", "()Ljava/lang/ClassLoader;", &[])?.l()?;
        let classpath = env.call_static_method("com/google/common/reflect/ClassPath", "from", "(Ljava/lang/ClassLoader;)Lcom/google/common/reflect/ClassPath;", &[JValue::Object(classloader)])?.l()?;

        Ok(Self {
            env,
            obj: Object::new(env, classpath, Class::for_name(env, "com/google/common/reflect/ClassPath")?)
        })
    }

    pub fn get_all_classes(&self) -> JResult<Vec<Class<'a>>> {
        let classes_set = self.env.call_method(self.obj.inner, "getAllClasses", "()Lcom/google/common/collect/ImmutableSet;", &[])?.l()?;
        let set = Set::new(self.env, Object::new(self.env, classes_set, Class::for_name(self.env, "com/google/common/collect/ImmutableSet")?), Class::for_name(self.env, "com/google/common/reflect/ClassPath$ClassInfo")?);
        let as_vec = set.to_vec()?;

        let as_classes = as_vec.into_iter()
            .map(|object| {
                let name_obj = self.env.call_method(object.inner, "getName", "()Ljava/lang/String;", &[])?.l()?;
                let class_name = JavaString::new(self.env, Object::new(self.env, name_obj, Class::String(self.env)?)).into_rust()?;

                if class_name.ends_with("module-info") || class_name.starts_with("META-INF") {
                    return Ok(None)
                }

                let class = Class::for_name(self.env, class_name)?;
                Ok(Some(class))
            })
            .collect::<JResult<Vec<_>>>()?
            .into_iter()
            .filter_map(|x| x)
            .collect::<Vec<_>>();

        Ok(as_classes)
    }
}