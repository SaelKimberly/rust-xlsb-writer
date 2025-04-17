macro_rules! declare_scope {
    ($name: ident) => {
        concat_idents::concat_idents!(name = BrtBegin, $name, {
            #[derive(deku::DekuRead, deku::DekuWrite, Debug, Hash, PartialEq, Default)]
            pub struct name {}
            impl crate::prelude::Unchecked for name {}
        });

        concat_idents::concat_idents!(name = BrtEnd, $name, {
            #[derive(deku::DekuRead, deku::DekuWrite, Debug, Hash, PartialEq, Default)]
            pub struct name {}
            impl crate::prelude::Unchecked for name {}
        });
    };
}

macro_rules! declare_scopes {
    ($name: ident$(, $name_other: ident)*) => {
        declare_scope!($name);
        $( declare_scope!($name_other); )*
    };
}

declare_scopes!(
    Sheet,
    SheetData,
    SingleCells,
    SlicerCacheIDs,
    SlicerCacheSiRanges,
    SlicerCachesPivotCacheIDs,
    Slicers,
    SlicersEx,
    SlicerStyleElements
);
