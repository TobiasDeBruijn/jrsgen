use convert_case::{Case, Casing};
use const_format::formatcp;

mod class;

pub use class::*;

const KEYWORD_SUFFIX: &str = "_k";
const SUBCLASS_PARENT_SUFFIX: &str = "_p";

/// Rename keywords. This will add the `KEYWORD_SUFFIX` if a keyword is used.
/// This function should be called on individual class name components. E.g. `com`
pub fn escape_keywords(x: &str) -> &str {
    match x {
        "impl" => formatcp!("impl{}", KEYWORD_SUFFIX),
        "move" => formatcp!("move{}", KEYWORD_SUFFIX),
        "in" => formatcp!("in{}", KEYWORD_SUFFIX),
        _ => x
    }
}

/// Rename the parent class of a subclass. This will:
/// - Rename keywords
/// - Convert to snake case
/// - Append the `SUBCLASS_PARENT_SUFFIX` suffix
///
/// This function should be called on individual class name components. E.g. `com`
fn rename_parent_class(input: &str) -> String {
    let keyword_renamed = escape_keywords(input);
    let case_adjusted = keyword_renamed.to_string().to_case(Case::Snake);

    format!("{case_adjusted}{SUBCLASS_PARENT_SUFFIX}")
}

/// Rename a fully qualified class name. This will:
/// - Fix casing on prefixing name components
/// - Correctly fix subclasses
/// - Adjust for keywords
///
/// This function should be called only for the entire class name path. E.g. `com.foo.Example`
pub fn rename_class_fq(input: &str) -> String {
    // Input format: com.foo.Bar$Baz

    // First, rename all components containing keywords
    let mut components = input.split('.').into_iter()
        .map(escape_keywords)
        .map(|x| x.to_string())
        .collect::<Vec<_>>();
    let class_name = components.pop().unwrap();

    // Correct the casing of the class name components that are not the class name itself
    let case_adjusted_components = components.into_iter()
        .map(|x| x.to_case(Case::Snake))
        .collect::<Vec<_>>();

    let mut class_fully_qualified = case_adjusted_components;

    // Handle subclasses
    let mut tmp = class_name;
    while tmp.contains('$') {
        // Unwrap is safe due to the condition in the while loop
        let (prefix, suffix) = tmp.split_once('$').unwrap();

        let parent_class = prefix;
        class_fully_qualified.push(rename_parent_class(parent_class));

        // Could be multiple, if multiple layers of nesting are present
        let child_classes = suffix;
        tmp = child_classes.to_string();
    }

    // tmp is now the name of the final subclass
    class_fully_qualified.push(tmp);

    class_fully_qualified.join(".")
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn simple_class() {
        let input = "com.foo.example.Bar";
        let output = rename_class_fq(input);

        assert_eq!("com.foo.example.Bar", &output);
    }

    #[test]
    fn with_keywords() {
        let input = "com.foo.impl.Bar";
        let output = rename_class_fq(input);

        assert_eq!(format!("com.foo.impl{KEYWORD_SUFFIX}.Bar"), output);
    }

    #[test]
    fn one_subclass() {
        let input = "com.foo.example.Bar$Baz";
        let output = rename_class_fq(input);

        assert_eq!(format!("com.foo.example.bar{SUBCLASS_PARENT_SUFFIX}.Baz"), output);
    }

    #[test]
    fn two_subclasses() {
        let input = "com.foo.example.Bar$Baz$Quix";
        let output = rename_class_fq(input);

        assert_eq!(format!("com.foo.example.bar{SUBCLASS_PARENT_SUFFIX}.baz{SUBCLASS_PARENT_SUFFIX}.Quix"), output);
    }

    #[test]
    fn three_subclasses() {
        let input = "com.foo.example.Bar$Baz$Quix$Example";
        let output = rename_class_fq(input);

        assert_eq!(format!("com.foo.example.bar{SUBCLASS_PARENT_SUFFIX}.baz{SUBCLASS_PARENT_SUFFIX}.quix{SUBCLASS_PARENT_SUFFIX}.Example"), output);
    }

    #[test]
    fn case_adjusting() {
        let input = "com.Foo.example.Bar";
        let output = rename_class_fq(input);

        assert_eq!("com.foo.example.Bar", &output);
    }

}