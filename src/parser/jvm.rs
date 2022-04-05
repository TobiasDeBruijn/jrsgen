use std::{fs, io};
use std::io::Write;
use std::ops::Deref;
use std::path::PathBuf;
use jni::{InitArgsBuilder, JavaVM, JNIVersion};
use log::{debug, trace};
use crate::JResult;

pub struct Jvm(JavaVM);

/// java-dependencies.jar
const JAVA_DEPENDENCIES: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/target/java-dependencies.jar"));



impl Deref for Jvm {
    type Target = JavaVM;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Jvm {
    fn write_java_dependencies_to_disk() -> io::Result<PathBuf> {
        let tmp = tempfile::tempdir()?;
        let path = tmp.path().to_path_buf();

        let jarpath = path.join("dependencies.jar");
        let mut f = fs::File::create(&jarpath)?;
        f.write_all(JAVA_DEPENDENCIES)?;

        // Prevent the jar from being deleted
        Box::leak(Box::new(tmp));

        Ok(PathBuf::from(jarpath))
    }

    pub fn new<S: AsRef<str>>(classpath: &[S]) -> JResult<Self> {
        let mut classpath = classpath.iter()
            .map(|x| x.as_ref())
            .collect::<Vec<_>>();

        // Add our own java dependencies to the classpath
        let path = Self::write_java_dependencies_to_disk()?;
        let path = path.to_string_lossy();
        classpath.push(&path);

        let classpath = classpath.join(":");
        trace!("Using classpath: {}", classpath);

        let args = InitArgsBuilder::new()
            .version(JNIVersion::V8)
            // .option("-Xcheck:jni")
            .option(&format!("-Djava.class.path={}", classpath));

        debug!("Launching JVM");
        let args = args.build()?;
        let vm = JavaVM::new(args)?;
        Ok(Self(vm))
    }
}