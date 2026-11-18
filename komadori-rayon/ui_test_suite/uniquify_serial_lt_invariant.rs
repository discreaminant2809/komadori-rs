use komadori::prelude::*;
use komadori_rayon::collector::plumbing::{DefineSerial, DefineUnindexedSerial};
use komadori_rayon::iter::ParCount;

fn main() {}

type Serial<'a> = <ParCount as DefineSerial<'a>>::Serial;
type UnindexedSerial<'a> = <ParCount as DefineUnindexedSerial<'a>>::UnindexedSerial;
type Output<'a> = <Serial<'a> as CollectorBase>::Output;
type UnindexedOutput<'a> = <UnindexedSerial<'a> as CollectorBase>::Output;

macro_rules! for_type {
    ($TyName:ident) => {
        const _: () = {
            fn covariant<'a>(x: $TyName<'static>) -> $TyName<'a> {
                x
            }

            fn contravariant<'a>(x: $TyName<'a>) -> $TyName<'static> {
                x
            }
        };
    };
}

for_type!(Serial);
for_type!(UnindexedSerial);
for_type!(Output);
for_type!(UnindexedOutput);
