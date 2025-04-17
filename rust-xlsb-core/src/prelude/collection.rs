macro_rules! declare_simple_collection {
    ($name: ident) => {
        concat_idents::concat_idents!(name = BrtBegin, $name, {
            #[derive(deku::DekuRead, deku::DekuWrite, Debug, Hash, PartialEq, Default)]
            pub struct name {
                pub count: u32,
            }
            impl crate::prelude::Unchecked for name {}
        });

        concat_idents::concat_idents!(name = BrtEnd, $name, {
            #[derive(deku::DekuRead, deku::DekuWrite, Debug, Hash, PartialEq, Default)]
            pub struct name {}
            impl crate::prelude::Unchecked for name {}
        });
    };
}

macro_rules! declare_simple_collections {
    ($name: ident$(, $name_other: ident)*) => {
        declare_simple_collection!($name);
        $( declare_simple_collection!($name_other); )*
    };
}
declare_simple_collections!(ColInfos);
