#[macro_export]
#[expect(missing_docs)]
macro_rules! with_supported_type_sets {
    ($callback:ident) => {
        with_supported_type_sets! {
            @build $callback;
            prefix: [];
            each: [];
            enums: E1, E2, E3, E4, E5, E6, E7, E8, E9, E10, E11, E12, E13, E14, E15, E16;
            types: T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16;
        }
    };
    (@build $callback:ident;
        prefix: [$($prefix:ident,)*];
        each: [$($built:tt)*];
        enums: $enum:ident;
        types: $next:ident;
    ) => {
        $callback! {
            each:
                $($built)*
                $enum => $($prefix,)* $next;
        }
    };
    (@build $callback:ident;
        prefix: [$($prefix:ident,)*];
        each: [$($built:tt)*];
        enums: $enum:ident, $($rest_enums:ident),+;
        types: $next:ident, $($rest_types:ident),+;
    ) => {
        with_supported_type_sets! {
            @build $callback;
            prefix: [$($prefix,)* $next,];
            each: [
                $($built)*
                $enum => $($prefix,)* $next;
            ];
            enums: $($rest_enums),+;
            types: $($rest_types),+;
        }
    };
}
