macro_rules! declare_scope {
    ($name: ident) => {
        concat_idents::concat_idents!(name = BrtBegin, $name, {
            #[derive(deku::DekuRead, deku::DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
            pub struct name {}
            impl crate::prelude::Unchecked for name {}
            impl crate::CheckedBiff for name {
                fn id(&self) -> crate::KnownID {
                    crate::KnownID::name
                }
            }
        });

        concat_idents::concat_idents!(name = BrtEnd, $name, {
            #[derive(deku::DekuRead, deku::DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
            pub struct name {}
            impl crate::prelude::Unchecked for name {}
            impl crate::CheckedBiff for name {
                fn id(&self) -> crate::KnownID {
                    crate::KnownID::name
                }
            }
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
    CellIgnoreECs,
    Book,
    BookViews,
    Externals,
    BundleShs,
    Sheet,
    SheetData,
    SingleCells,
    SlicerCacheIDs,
    SlicerCacheSiRanges,
    SlicerCachesPivotCacheIDs,
    Slicers,
    SlicersEx,
    SlicerStyleElements,
    StyleSheet,
    WsViews
);
