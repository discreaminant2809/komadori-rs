use komadori::prelude::*;
use komadori_rayon::prelude::*;

fn main() {}

macro_rules! for_method {
    ($method_name:ident($($len:expr)?)) => {
        fn $method_name() {
            use komadori_rayon::iter::ParCount;

            {
                let mut collector = ParCount::new();
                let (consumer, _) = collector.$method_name($($len)*);
                let _serial = consumer.into_collector();
                drop(collector);
            }

            {
                let mut collector = ParCount::new();
                let (consumer, _) = collector.$method_name($($len)*);
                let _output = consumer.into_collector().finish();
                drop(collector);
            }
        }
    };
}

for_method!(parts(0));
for_method!(take_parts(0));
for_method!(unindexed_parts());
for_method!(take_unindexed_parts());
