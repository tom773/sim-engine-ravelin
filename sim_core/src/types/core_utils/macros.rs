#[macro_export]
macro_rules! good_id {
    ($slug:literal) => {
        $crate::goods::CATALOGUE
            .get_good_id_by_slug($slug)
            .expect(concat!("unknown good slug: ", $slug))
    };
}

#[macro_export]
macro_rules! recipe_id {
    ($name:literal) => {
        sim_types::goods::CATALOGUE
            .get_recipe_id_by_name($name)
            .expect(concat!("unknown recipe name: ", $name))
    };
}

#[macro_export]
macro_rules! pserde {
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