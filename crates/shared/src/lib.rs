#[macro_export]
macro_rules! prep_serde_as {
    ($outer:ty, $inner:ty) => {
        impl std::fmt::Display for $outer {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
        
        impl std::str::FromStr for $outer {
            type Err = <$inner as std::str::FromStr>::Err;
            
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self(s.parse::<$inner>()?))
            }
        }
    };
}

#[allow(unused)]
#[allow(ambiguous_glob_reexports)]
pub mod fin_sys;
pub use fin_sys::*;

pub mod types;
pub use types::*;

pub mod validation;
pub use validation::*;

pub mod behaviour;
pub use behaviour::*;
