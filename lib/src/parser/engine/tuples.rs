use super::{parser_trait::Parser, IResult, Input};

macro_rules! impl_parsable_for_tuple {
    ($($name:ident $name2:ident),*) => {
        #[allow(non_snake_case)]
        impl<$($name: Parser<Output = $name2>, $name2),*> Parser for &mut ($($name,)*) {
            type Output = ($($name::Output,)*);

            fn process<'a>(&mut self, tokens: Input<'a>) -> IResult<'a, Self::Output> {
                let ($(
                    $name,
                )*) = self;
                $(
                    let (tokens, $name) = match $name.process(tokens) {
                        Ok(result) => result,
                        Err(e) => {
                            super::track_error(&e);
                            return Err(e);
                        }
                    };
                )*
                Ok((tokens, ($($name,)*)))
            }
        }

        #[allow(non_snake_case)]
        impl<$($name: Parser<Output = $name2>, $name2),*> Parser for ($($name,)*) {
            type Output = ($($name::Output,)*);

            fn process<'a>(&mut self, tokens: Input<'a>) -> IResult<'a, Self::Output> {
                let ($(
                    $name,
                )*) = self;
                $(
                    let (tokens, $name) = match $name.process(tokens) {
                        Ok(result) => result,
                        Err(e) => {
                            super::track_error(&e);
                            return Err(e);
                        }
                    };
                )*
                Ok((tokens, ($($name,)*)))
            }
        }

        #[allow(non_snake_case)]
        impl<$($name: Parser<Output = $name2> + Clone, $name2),*> Parser for &($($name,)*) where Self: Clone {
            type Output = ($($name::Output,)*);

            fn process<'a>(&mut self, tokens: Input<'a>) -> IResult<'a, Self::Output> {
                let ($(
                    mut $name,
                )*) = self.clone();
                $(
                    let (tokens, $name) = match $name.process(tokens) {
                        Ok(result) => result,
                        Err(e) => {
                            super::track_error(&e);
                            return Err(e);
                        }
                    };
                )*
                Ok((tokens, ($($name,)*)))
            }
        }
    };
}

macro_rules! impl_parsable_for_tuples_up_to_32 {
    ($name:ident $name2:ident, $($names:ident $names2:ident),*) => {
        impl_parsable_for_tuple!($name $name2, $($names $names2),*);
        impl_parsable_for_tuples_up_to_32!($($names $names2),*);
    };
    ($name:ident $name2:ident) => {
        impl_parsable_for_tuple!($name $name2);
    };
}

impl_parsable_for_tuples_up_to_32!(
    A1 A2, B1 B2, C1 C2, D1 D2, E1 E2, F1 F2, G1 G2, H1 H2, I1 I2, J1 J2, K1 K2, L1 L2, M1 M2, N1 N2, O1 O2, P1 P2, Q1 Q2, R1 R2, S1 S2, T1 T2, U1 U2, V1 V2, W1 W2, X1 X2, Y1 Y2, Z1 Z2,
    AA1 AA2, AB1 AB2, AC1 AC2, AD1 AD2, AE1 AE2, AF1 AF2, AG1 AG2, AH1 AH2
);
