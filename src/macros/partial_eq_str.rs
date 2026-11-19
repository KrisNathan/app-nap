#[macro_export]
macro_rules! partial_eq_str {
    ($name:ident) => {
        impl PartialEq<str> for $name {
            fn eq(&self, other: &str) -> bool {
                self.0 == other
            }
        }

        impl<'a> PartialEq<&'a str> for $name {
            fn eq(&self, other: &&'a str) -> bool {
                self.0 == *other
            }
        }

        impl PartialEq<$name> for str {
            fn eq(&self, other: &$name) -> bool {
                self == other.0
            }
        }

        impl<'a> PartialEq<$name> for &'a str {
            fn eq(&self, other: &$name) -> bool {
                *self == other.0
            }
        }
    };
}
