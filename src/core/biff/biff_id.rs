#[cfg(feature = "test")]
use strum_macros::Display as EnumDisplay;

use super::BiffRecord;

/// BiffId ID from u16, based on section 2.1.4
const fn as_biff_id(id: u16) -> u16 {
    if id > 0x00_7f {
        (id & 0x7f | 0x80) | ((id & 0x3f80) << 1)
    } else {
        id & 0x7f
    }
}

#[repr(u16)]
#[allow(non_camel_case_types, dead_code)]
#[cfg_attr(feature = "test", derive(Debug, EnumDisplay))]
#[derive(Copy, Clone, PartialEq)]
/// Enumeration with all variants of XLSB BIFF IDs from section 2.3.2
pub(crate) enum BiffId {
    BrtRowHdr = as_biff_id(0),
    BrtCellBlank = as_biff_id(1),
    BrtCellRk = as_biff_id(2),
    BrtCellError = as_biff_id(3),
    BrtCellBool = as_biff_id(4),
    BrtCellReal = as_biff_id(5),
    BrtCellSt = as_biff_id(6),
    BrtCellIsst = as_biff_id(7),
    BrtFmlaString = as_biff_id(8),
    BrtFmlaNum = as_biff_id(9),
    BrtFmlaBool = as_biff_id(10),
    BrtFmlaError = as_biff_id(11),
    BrtSSTItem = as_biff_id(19),
    BrtPCDIMissing = as_biff_id(20),
    BrtPCDINumber = as_biff_id(21),
    BrtPCDIBoolean = as_biff_id(22),
    BrtPCDIError = as_biff_id(23),
    BrtPCDIString = as_biff_id(24),
    BrtPCDIDatetime = as_biff_id(25),
    BrtPCDIIndex = as_biff_id(26),
    BrtPCDIAMissing = as_biff_id(27),
    BrtPCDIANumber = as_biff_id(28),
    BrtPCDIABoolean = as_biff_id(29),
    BrtPCDIAError = as_biff_id(30),
    BrtPCDIAString = as_biff_id(31),
    BrtPCDIADatetime = as_biff_id(32),
    BrtPCRRecord = as_biff_id(33),
    BrtPCRRecordDt = as_biff_id(34),
    BrtFRTBegin = as_biff_id(35),
    BrtFRTEnd = as_biff_id(36),
    BrtACBegin = as_biff_id(37),
    BrtACEnd = as_biff_id(38),
    BrtName = as_biff_id(39),
    BrtIndexRowBlock = as_biff_id(40),
    BrtIndexBlock = as_biff_id(42),
    BrtFont = as_biff_id(43),
    BrtFmt = as_biff_id(44),
    BrtFill = as_biff_id(45),
    BrtBorder = as_biff_id(46),
    BrtXF = as_biff_id(47),
    BrtStyle = as_biff_id(48),
    BrtCellMeta = as_biff_id(49),
    BrtValueMeta = as_biff_id(50),
    BrtMdb = as_biff_id(51),
    BrtBeginFmd = as_biff_id(52),
    BrtEndFmd = as_biff_id(53),
    BrtBeginMdx = as_biff_id(54),
    BrtEndMdx = as_biff_id(55),
    BrtBeginMdxTuple = as_biff_id(56),
    BrtEndMdxTuple = as_biff_id(57),
    BrtMdxMbrIstr = as_biff_id(58),
    BrtStr = as_biff_id(59),
    BrtColInfo = as_biff_id(60),
    BrtCellRString = as_biff_id(62),
    BrtDVal = as_biff_id(64),
    BrtSxvcellNum = as_biff_id(65),
    BrtSxvcellStr = as_biff_id(66),
    BrtSxvcellBool = as_biff_id(67),
    BrtSxvcellErr = as_biff_id(68),
    BrtSxvcellDate = as_biff_id(69),
    BrtSxvcellNil = as_biff_id(70),
    BrtFileVersion = as_biff_id(128),
    BrtBeginSheet = as_biff_id(129),
    BrtEndSheet = as_biff_id(130),
    BrtBeginBook = as_biff_id(131),
    BrtEndBook = as_biff_id(132),
    BrtBeginWsViews = as_biff_id(133),
    BrtEndWsViews = as_biff_id(134),
    BrtBeginBookViews = as_biff_id(135),
    BrtEndBookViews = as_biff_id(136),
    BrtBeginWsView = as_biff_id(137),
    BrtEndWsView = as_biff_id(138),
    BrtBeginCsViews = as_biff_id(139),
    BrtEndCsViews = as_biff_id(140),
    BrtBeginCsView = as_biff_id(141),
    BrtEndCsView = as_biff_id(142),
    BrtBeginBundleShs = as_biff_id(143),
    BrtEndBundleShs = as_biff_id(144),
    BrtBeginSheetData = as_biff_id(145),
    BrtEndSheetData = as_biff_id(146),
    BrtWsProp = as_biff_id(147),
    BrtWsDim = as_biff_id(148),
    BrtPane = as_biff_id(151),
    BrtSel = as_biff_id(152),
    BrtWbProp = as_biff_id(153),
    BrtWbFactoid = as_biff_id(154),
    BrtFileRecover = as_biff_id(155),
    BrtBundleSh = as_biff_id(156),
    BrtCalcProp = as_biff_id(157),
    BrtBookView = as_biff_id(158),
    BrtBeginSst = as_biff_id(159),
    BrtEndSst = as_biff_id(160),
    BrtBeginAFilter = as_biff_id(161),
    BrtEndAFilter = as_biff_id(162),
    BrtBeginFilterColumn = as_biff_id(163),
    BrtEndFilterColumn = as_biff_id(164),
    BrtBeginFilters = as_biff_id(165),
    BrtEndFilters = as_biff_id(166),
    BrtFilter = as_biff_id(167),
    BrtColorFilter = as_biff_id(168),
    BrtIconFilter = as_biff_id(169),
    BrtTop10Filter = as_biff_id(170),
    BrtDynamicFilter = as_biff_id(171),
    BrtBeginCustomFilters = as_biff_id(172),
    BrtEndCustomFilters = as_biff_id(173),
    BrtCustomFilter = as_biff_id(174),
    BrtAFilterDateGroupItem = as_biff_id(175),
    BrtMergeCell = as_biff_id(176),
    BrtBeginMergeCells = as_biff_id(177),
    BrtEndMergeCells = as_biff_id(178),
    BrtBeginPivotCacheDef = as_biff_id(179),
    BrtEndPivotCacheDef = as_biff_id(180),
    BrtBeginPCDFields = as_biff_id(181),
    BrtEndPCDFields = as_biff_id(182),
    BrtBeginPCDField = as_biff_id(183),
    BrtEndPCDField = as_biff_id(184),
    BrtBeginPCDSource = as_biff_id(185),
    BrtEndPCDSource = as_biff_id(186),
    BrtBeginPCDSRange = as_biff_id(187),
    BrtEndPCDSRange = as_biff_id(188),
    BrtBeginPCDFAtbl = as_biff_id(189),
    BrtEndPCDFAtbl = as_biff_id(190),
    BrtBeginPCDIRun = as_biff_id(191),
    BrtEndPCDIRun = as_biff_id(192),
    BrtBeginPivotCacheRecords = as_biff_id(193),
    BrtEndPivotCacheRecords = as_biff_id(194),
    BrtBeginPCDHierarchies = as_biff_id(195),
    BrtEndPCDHierarchies = as_biff_id(196),
    BrtBeginPCDHierarchy = as_biff_id(197),
    BrtEndPCDHierarchy = as_biff_id(198),
    BrtBeginPCDHFieldsUsage = as_biff_id(199),
    BrtEndPCDHFieldsUsage = as_biff_id(200),
    BrtBeginExtConnection = as_biff_id(201),
    BrtEndExtConnection = as_biff_id(202),
    BrtBeginECDbProps = as_biff_id(203),
    BrtEndECDbProps = as_biff_id(204),
    BrtBeginECOlapProps = as_biff_id(205),
    BrtEndECOlapProps = as_biff_id(206),
    BrtBeginPCDSConsol = as_biff_id(207),
    BrtEndPCDSConsol = as_biff_id(208),
    BrtBeginPCDSCPages = as_biff_id(209),
    BrtEndPCDSCPages = as_biff_id(210),
    BrtBeginPCDSCPage = as_biff_id(211),
    BrtEndPCDSCPage = as_biff_id(212),
    BrtBeginPCDSCPItem = as_biff_id(213),
    BrtEndPCDSCPItem = as_biff_id(214),
    BrtBeginPCDSCSets = as_biff_id(215),
    BrtEndPCDSCSets = as_biff_id(216),
    BrtBeginPCDSCSet = as_biff_id(217),
    BrtEndPCDSCSet = as_biff_id(218),
    BrtBeginPCDFGroup = as_biff_id(219),
    BrtEndPCDFGroup = as_biff_id(220),
    BrtBeginPCDFGItems = as_biff_id(221),
    BrtEndPCDFGItems = as_biff_id(222),
    BrtBeginPCDFGRange = as_biff_id(223),
    BrtEndPCDFGRange = as_biff_id(224),
    BrtBeginPCDFGDiscrete = as_biff_id(225),
    BrtEndPCDFGDiscrete = as_biff_id(226),
    BrtBeginPCDSDTupleCache = as_biff_id(227),
    BrtEndPCDSDTupleCache = as_biff_id(228),
    BrtBeginPCDSDTCEntries = as_biff_id(229),
    BrtEndPCDSDTCEntries = as_biff_id(230),
    BrtBeginPCDSDTCEMembers = as_biff_id(231),
    BrtEndPCDSDTCEMembers = as_biff_id(232),
    BrtBeginPCDSDTCEMember = as_biff_id(233),
    BrtEndPCDSDTCEMember = as_biff_id(234),
    BrtBeginPCDSDTCQueries = as_biff_id(235),
    BrtEndPCDSDTCQueries = as_biff_id(236),
    BrtBeginPCDSDTCQuery = as_biff_id(237),
    BrtEndPCDSDTCQuery = as_biff_id(238),
    BrtBeginPCDSDTCSets = as_biff_id(239),
    BrtEndPCDSDTCSets = as_biff_id(240),
    BrtBeginPCDSDTCSet = as_biff_id(241),
    BrtEndPCDSDTCSet = as_biff_id(242),
    BrtBeginPCDCalcItems = as_biff_id(243),
    BrtEndPCDCalcItems = as_biff_id(244),
    BrtBeginPCDCalcItem = as_biff_id(245),
    BrtEndPCDCalcItem = as_biff_id(246),
    BrtBeginPRule = as_biff_id(247),
    BrtEndPRule = as_biff_id(248),
    BrtBeginPRFilters = as_biff_id(249),
    BrtEndPRFilters = as_biff_id(250),
    BrtBeginPRFilter = as_biff_id(251),
    BrtEndPRFilter = as_biff_id(252),
    BrtBeginPNames = as_biff_id(253),
    BrtEndPNames = as_biff_id(254),
    BrtBeginPName = as_biff_id(255),
    BrtEndPName = as_biff_id(256),
    BrtBeginPNPairs = as_biff_id(257),
    BrtEndPNPairs = as_biff_id(258),
    BrtBeginPNPair = as_biff_id(259),
    BrtEndPNPair = as_biff_id(260),
    BrtBeginECWebProps = as_biff_id(261),
    BrtEndECWebProps = as_biff_id(262),
    BrtBeginEcWpTables = as_biff_id(263),
    BrtEndECWPTables = as_biff_id(264),
    BrtBeginECParams = as_biff_id(265),
    BrtEndECParams = as_biff_id(266),
    BrtBeginECParam = as_biff_id(267),
    BrtEndECParam = as_biff_id(268),
    BrtBeginPCDKPIs = as_biff_id(269),
    BrtEndPCDKPIs = as_biff_id(270),
    BrtBeginPCDKPI = as_biff_id(271),
    BrtEndPCDKPI = as_biff_id(272),
    BrtBeginDims = as_biff_id(273),
    BrtEndDims = as_biff_id(274),
    BrtBeginDim = as_biff_id(275),
    BrtEndDim = as_biff_id(276),
    BrtIndexPartEnd = as_biff_id(277),
    BrtBeginStyleSheet = as_biff_id(278),
    BrtEndStyleSheet = as_biff_id(279),
    BrtBeginSXView = as_biff_id(280),
    BrtEndSXVI = as_biff_id(281),
    BrtBeginSXVI = as_biff_id(282),
    BrtBeginSXVIs = as_biff_id(283),
    BrtEndSXVIs = as_biff_id(284),
    BrtBeginSXVD = as_biff_id(285),
    BrtEndSXVD = as_biff_id(286),
    BrtBeginSXVDs = as_biff_id(287),
    BrtEndSXVDs = as_biff_id(288),
    BrtBeginSXPI = as_biff_id(289),
    BrtEndSXPI = as_biff_id(290),
    BrtBeginSXPIs = as_biff_id(291),
    BrtEndSXPIs = as_biff_id(292),
    BrtBeginSXDI = as_biff_id(293),
    BrtEndSXDI = as_biff_id(294),
    BrtBeginSXDIs = as_biff_id(295),
    BrtEndSXDIs = as_biff_id(296),
    BrtBeginSXLI = as_biff_id(297),
    BrtEndSXLI = as_biff_id(298),
    BrtBeginSXLIRws = as_biff_id(299),
    BrtEndSXLIRws = as_biff_id(300),
    BrtBeginSXLICols = as_biff_id(301),
    BrtEndSXLICols = as_biff_id(302),
    BrtBeginSXFormat = as_biff_id(303),
    BrtEndSXFormat = as_biff_id(304),
    BrtBeginSXFormats = as_biff_id(305),
    BrtEndSxFormats = as_biff_id(306),
    BrtBeginSxSelect = as_biff_id(307),
    BrtEndSxSelect = as_biff_id(308),
    BrtBeginISXVDRws = as_biff_id(309),
    BrtEndISXVDRws = as_biff_id(310),
    BrtBeginISXVDCols = as_biff_id(311),
    BrtEndISXVDCols = as_biff_id(312),
    BrtEndSXLocation = as_biff_id(313),
    BrtBeginSXLocation = as_biff_id(314),
    BrtEndSXView = as_biff_id(315),
    BrtBeginSXTHs = as_biff_id(316),
    BrtEndSXTHs = as_biff_id(317),
    BrtBeginSXTH = as_biff_id(318),
    BrtEndSXTH = as_biff_id(319),
    BrtBeginISXTHRws = as_biff_id(320),
    BrtEndISXTHRws = as_biff_id(321),
    BrtBeginISXTHCols = as_biff_id(322),
    BrtEndISXTHCols = as_biff_id(323),
    BrtBeginSXTDMPS = as_biff_id(324),
    BrtEndSXTDMPs = as_biff_id(325),
    BrtBeginSXTDMP = as_biff_id(326),
    BrtEndSXTDMP = as_biff_id(327),
    BrtBeginSXTHItems = as_biff_id(328),
    BrtEndSXTHItems = as_biff_id(329),
    BrtBeginSXTHItem = as_biff_id(330),
    BrtEndSXTHItem = as_biff_id(331),
    BrtBeginMetadata = as_biff_id(332),
    BrtEndMetadata = as_biff_id(333),
    BrtBeginEsmdtinfo = as_biff_id(334),
    BrtMdtinfo = as_biff_id(335),
    BrtEndEsmdtinfo = as_biff_id(336),
    BrtBeginEsmdb = as_biff_id(337),
    BrtEndEsmdb = as_biff_id(338),
    BrtBeginEsfmd = as_biff_id(339),
    BrtEndEsfmd = as_biff_id(340),
    BrtBeginSingleCells = as_biff_id(341),
    BrtEndSingleCells = as_biff_id(342),
    BrtBeginList = as_biff_id(343),
    BrtEndList = as_biff_id(344),
    BrtBeginListCols = as_biff_id(345),
    BrtEndListCols = as_biff_id(346),
    BrtBeginListCol = as_biff_id(347),
    BrtEndListCol = as_biff_id(348),
    BrtBeginListXmlCPr = as_biff_id(349),
    BrtEndListXmlCPr = as_biff_id(350),
    BrtListCCFmla = as_biff_id(351),
    BrtListTrFmla = as_biff_id(352),
    BrtBeginExternals = as_biff_id(353),
    BrtEndExternals = as_biff_id(354),
    BrtSupBookSrc = as_biff_id(355),
    BrtSupSelf = as_biff_id(357),
    BrtSupSame = as_biff_id(358),
    BrtSupTabs = as_biff_id(359),
    BrtBeginSupBook = as_biff_id(360),
    BrtPlaceholderName = as_biff_id(361),
    BrtExternSheet = as_biff_id(362),
    BrtExternTableStart = as_biff_id(363),
    BrtExternTableEnd = as_biff_id(364),
    BrtExternRowHdr = as_biff_id(366),
    BrtExternCellBlank = as_biff_id(367),
    BrtExternCellReal = as_biff_id(368),
    BrtExternCellBool = as_biff_id(369),
    BrtExternCellError = as_biff_id(370),
    BrtExternCellString = as_biff_id(371),
    BrtBeginEsmdx = as_biff_id(372),
    BrtEndEsmdx = as_biff_id(373),
    BrtBeginMdxSet = as_biff_id(374),
    BrtEndMdxSet = as_biff_id(375),
    BrtBeginMdxMbrProp = as_biff_id(376),
    BrtEndMdxMbrProp = as_biff_id(377),
    BrtBeginMdxKPI = as_biff_id(378),
    BrtEndMdxKPI = as_biff_id(379),
    BrtBeginEsstr = as_biff_id(380),
    BrtEndEsstr = as_biff_id(381),
    BrtBeginPRFItem = as_biff_id(382),
    BrtEndPRFItem = as_biff_id(383),
    BrtBeginPivotCacheIDs = as_biff_id(384),
    BrtEndPivotCacheIDs = as_biff_id(385),
    BrtBeginPivotCacheID = as_biff_id(386),
    BrtEndPivotCacheID = as_biff_id(387),
    BrtBeginISXVIs = as_biff_id(388),
    BrtEndISXVIs = as_biff_id(389),
    BrtBeginColInfos = as_biff_id(390),
    BrtEndColInfos = as_biff_id(391),
    BrtBeginRwBrk = as_biff_id(392),
    BrtEndRwBrk = as_biff_id(393),
    BrtBeginColBrk = as_biff_id(394),
    BrtEndColBrk = as_biff_id(395),
    BrtBrk = as_biff_id(396),
    BrtUserBookView = as_biff_id(397),
    BrtInfo = as_biff_id(398),
    BrtCUsr = as_biff_id(399),
    BrtUsr = as_biff_id(400),
    BrtBeginUsers = as_biff_id(401),
    BrtEOF = as_biff_id(403),
    BrtUCR = as_biff_id(404),
    BrtRRInsDel = as_biff_id(405),
    BrtRREndInsDel = as_biff_id(406),
    BrtRRMove = as_biff_id(407),
    BrtRREndMove = as_biff_id(408),
    BrtRRChgCell = as_biff_id(409),
    BrtRREndChgCell = as_biff_id(410),
    BrtRRHeader = as_biff_id(411),
    BrtRRUserView = as_biff_id(412),
    BrtRRRenSheet = as_biff_id(413),
    BrtRRInsertSh = as_biff_id(414),
    BrtRRDefName = as_biff_id(415),
    BrtRRNote = as_biff_id(416),
    BrtRRConflict = as_biff_id(417),
    BrtRRTQSIF = as_biff_id(418),
    BrtRRFormat = as_biff_id(419),
    BrtRREndFormat = as_biff_id(420),
    BrtRRAutoFmt = as_biff_id(421),
    BrtBeginUserShViews = as_biff_id(422),
    BrtBeginUserShView = as_biff_id(423),
    BrtEndUserShView = as_biff_id(424),
    BrtEndUserShViews = as_biff_id(425),
    BrtArrFmla = as_biff_id(426),
    BrtShrFmla = as_biff_id(427),
    BrtTable = as_biff_id(428),
    BrtBeginExtConnections = as_biff_id(429),
    BrtEndExtConnections = as_biff_id(430),
    BrtBeginPCDCalcMems = as_biff_id(431),
    BrtEndPCDCalcMems = as_biff_id(432),
    BrtBeginPCDCalcMem = as_biff_id(433),
    BrtEndPCDCalcMem = as_biff_id(434),
    BrtBeginPCDHGLevels = as_biff_id(435),
    BrtEndPCDHGLevels = as_biff_id(436),
    BrtBeginPCDHGLevel = as_biff_id(437),
    BrtEndPCDHGLevel = as_biff_id(438),
    BrtBeginPCDHGLGroups = as_biff_id(439),
    BrtEndPCDHGLGroups = as_biff_id(440),
    BrtBeginPCDHGLGroup = as_biff_id(441),
    BrtEndPCDHGLGroup = as_biff_id(442),
    BrtBeginPCDHGLGMembers = as_biff_id(443),
    BrtEndPCDHGLGMembers = as_biff_id(444),
    BrtBeginPCDHGLGMember = as_biff_id(445),
    BrtEndPCDHGLGMember = as_biff_id(446),
    BrtBeginQSI = as_biff_id(447),
    BrtEndQSI = as_biff_id(448),
    BrtBeginQSIR = as_biff_id(449),
    BrtEndQSIR = as_biff_id(450),
    BrtBeginDeletedNames = as_biff_id(451),
    BrtEndDeletedNames = as_biff_id(452),
    BrtBeginDeletedName = as_biff_id(453),
    BrtEndDeletedName = as_biff_id(454),
    BrtBeginQSIFs = as_biff_id(455),
    BrtEndQSIFs = as_biff_id(456),
    BrtBeginQSIF = as_biff_id(457),
    BrtEndQSIF = as_biff_id(458),
    BrtBeginAutoSortScope = as_biff_id(459),
    BrtEndAutoSortScope = as_biff_id(460),
    BrtBeginConditionalFormatting = as_biff_id(461),
    BrtEndConditionalFormatting = as_biff_id(462),
    BrtBeginCFRule = as_biff_id(463),
    BrtEndCFRule = as_biff_id(464),
    BrtBeginIconSet = as_biff_id(465),
    BrtEndIconSet = as_biff_id(466),
    BrtBeginDatabar = as_biff_id(467),
    BrtEndDatabar = as_biff_id(468),
    BrtBeginColorScale = as_biff_id(469),
    BrtEndColorScale = as_biff_id(470),
    BrtCFVO = as_biff_id(471),
    BrtExternValueMeta = as_biff_id(472),
    BrtBeginColorPalette = as_biff_id(473),
    BrtEndColorPalette = as_biff_id(474),
    BrtIndexedColor = as_biff_id(475),
    BrtMargins = as_biff_id(476),
    BrtPrintOptions = as_biff_id(477),
    BrtPageSetup = as_biff_id(478),
    BrtBeginHeaderFooter = as_biff_id(479),
    BrtEndHeaderFooter = as_biff_id(480),
    BrtBeginSXCrtFormat = as_biff_id(481),
    BrtEndSXCrtFormat = as_biff_id(482),
    BrtBeginSXCrtFormats = as_biff_id(483),
    BrtEndSXCrtFormats = as_biff_id(484),
    BrtWsFmtInfo = as_biff_id(485),
    BrtBeginMgs = as_biff_id(486),
    BrtEndMGs = as_biff_id(487),
    BrtBeginMGMaps = as_biff_id(488),
    BrtEndMGMaps = as_biff_id(489),
    BrtBeginMG = as_biff_id(490),
    BrtEndMG = as_biff_id(491),
    BrtBeginMap = as_biff_id(492),
    BrtEndMap = as_biff_id(493),
    BrtHLink = as_biff_id(494),
    BrtBeginDCon = as_biff_id(495),
    BrtEndDCon = as_biff_id(496),
    BrtBeginDRefs = as_biff_id(497),
    BrtEndDRefs = as_biff_id(498),
    BrtDRef = as_biff_id(499),
    BrtBeginScenMan = as_biff_id(500),
    BrtEndScenMan = as_biff_id(501),
    BrtBeginSct = as_biff_id(502),
    BrtEndSct = as_biff_id(503),
    BrtSlc = as_biff_id(504),
    BrtBeginDXFs = as_biff_id(505),
    BrtEndDXFs = as_biff_id(506),
    BrtDXF = as_biff_id(507),
    BrtBeginTableStyles = as_biff_id(508),
    BrtEndTableStyles = as_biff_id(509),
    BrtBeginTableStyle = as_biff_id(510),
    BrtEndTableStyle = as_biff_id(511),
    BrtTableStyleElement = as_biff_id(512),
    BrtTableStyleClient = as_biff_id(513),
    BrtBeginVolDeps = as_biff_id(514),
    BrtEndVolDeps = as_biff_id(515),
    BrtBeginVolType = as_biff_id(516),
    BrtEndVolType = as_biff_id(517),
    BrtBeginVolMain = as_biff_id(518),
    BrtEndVolMain = as_biff_id(519),
    BrtBeginVolTopic = as_biff_id(520),
    BrtEndVolTopic = as_biff_id(521),
    BrtVolSubtopic = as_biff_id(522),
    BrtVolRef = as_biff_id(523),
    BrtVolNum = as_biff_id(524),
    BrtVolErr = as_biff_id(525),
    BrtVolStr = as_biff_id(526),
    BrtVolBool = as_biff_id(527),
    BrtBeginSortState = as_biff_id(530),
    BrtEndSortState = as_biff_id(531),
    BrtBeginSortCond = as_biff_id(532),
    BrtEndSortCond = as_biff_id(533),
    BrtBookProtection = as_biff_id(534),
    BrtSheetProtection = as_biff_id(535),
    BrtRangeProtection = as_biff_id(536),
    BrtPhoneticInfo = as_biff_id(537),
    BrtBeginECTxtWiz = as_biff_id(538),
    BrtEndECTxtWiz = as_biff_id(539),
    BrtBeginECTWFldInfoLst = as_biff_id(540),
    BrtEndECTWFldInfoLst = as_biff_id(541),
    BrtBeginECTwFldInfo = as_biff_id(542),
    BrtFileSharing = as_biff_id(548),
    BrtOleSize = as_biff_id(549),
    BrtDrawing = as_biff_id(550),
    BrtLegacyDrawing = as_biff_id(551),
    BrtLegacyDrawingHF = as_biff_id(552),
    BrtWebOpt = as_biff_id(553),
    BrtBeginWebPubItems = as_biff_id(554),
    BrtEndWebPubItems = as_biff_id(555),
    BrtBeginWebPubItem = as_biff_id(556),
    BrtEndWebPubItem = as_biff_id(557),
    BrtBeginSXCondFmt = as_biff_id(558),
    BrtEndSXCondFmt = as_biff_id(559),
    BrtBeginSXCondFmts = as_biff_id(560),
    BrtEndSXCondFmts = as_biff_id(561),
    BrtBkHim = as_biff_id(562),
    BrtColor = as_biff_id(564),
    BrtBeginIndexedColors = as_biff_id(565),
    BrtEndIndexedColors = as_biff_id(566),
    BrtBeginMRUColors = as_biff_id(569),
    BrtEndMRUColors = as_biff_id(570),
    BrtMRUColor = as_biff_id(572),
    BrtBeginDVals = as_biff_id(573),
    BrtEndDVals = as_biff_id(574),
    BrtSupNameStart = as_biff_id(577),
    BrtSupNameValueStart = as_biff_id(578),
    BrtSupNameValueEnd = as_biff_id(579),
    BrtSupNameNum = as_biff_id(580),
    BrtSupNameErr = as_biff_id(581),
    BrtSupNameSt = as_biff_id(582),
    BrtSupNameNil = as_biff_id(583),
    BrtSupNameBool = as_biff_id(584),
    BrtSupNameFmla = as_biff_id(585),
    BrtSupNameBits = as_biff_id(586),
    BrtSupNameEnd = as_biff_id(587),
    BrtEndSupBook = as_biff_id(588),
    BrtCellSmartTagProperty = as_biff_id(589),
    BrtBeginCellSmartTag = as_biff_id(590),
    BrtEndCellSmartTag = as_biff_id(591),
    BrtBeginCellSmartTags = as_biff_id(592),
    BrtEndCellSmartTags = as_biff_id(593),
    BrtBeginSmartTags = as_biff_id(594),
    BrtEndSmartTags = as_biff_id(595),
    BrtSmartTagType = as_biff_id(596),
    BrtBeginSmartTagTypes = as_biff_id(597),
    BrtEndSmartTagTypes = as_biff_id(598),
    BrtBeginSXFilters = as_biff_id(599),
    BrtEndSXFilters = as_biff_id(600),
    BrtBeginSXFILTER = as_biff_id(601),
    BrtEndSXFilter = as_biff_id(602),
    BrtBeginFills = as_biff_id(603),
    BrtEndFills = as_biff_id(604),
    BrtBeginCellWatches = as_biff_id(605),
    BrtEndCellWatches = as_biff_id(606),
    BrtCellWatch = as_biff_id(607),
    BrtBeginCRErrs = as_biff_id(608),
    BrtEndCRErrs = as_biff_id(609),
    BrtCrashRecErr = as_biff_id(610),
    BrtBeginFonts = as_biff_id(611),
    BrtEndFonts = as_biff_id(612),
    BrtBeginBorders = as_biff_id(613),
    BrtEndBorders = as_biff_id(614),
    BrtBeginFmts = as_biff_id(615),
    BrtEndFmts = as_biff_id(616),
    BrtBeginCellXFs = as_biff_id(617),
    BrtEndCellXFs = as_biff_id(618),
    BrtBeginStyles = as_biff_id(619),
    BrtEndStyles = as_biff_id(620),
    BrtBigName = as_biff_id(625),
    BrtBeginCellStyleXFs = as_biff_id(626),
    BrtEndCellStyleXFs = as_biff_id(627),
    BrtBeginComments = as_biff_id(628),
    BrtEndComments = as_biff_id(629),
    BrtBeginCommentAuthors = as_biff_id(630),
    BrtEndCommentAuthors = as_biff_id(631),
    BrtCommentAuthor = as_biff_id(632),
    BrtBeginCommentList = as_biff_id(633),
    BrtEndCommentList = as_biff_id(634),
    BrtBeginComment = as_biff_id(635),
    BrtEndComment = as_biff_id(636),
    BrtCommentText = as_biff_id(637),
    BrtBeginOleObjects = as_biff_id(638),
    BrtOleObject = as_biff_id(639),
    BrtEndOleObjects = as_biff_id(640),
    BrtBeginSxrules = as_biff_id(641),
    BrtEndSxRules = as_biff_id(642),
    BrtBeginActiveXControls = as_biff_id(643),
    BrtActiveX = as_biff_id(644),
    BrtEndActiveXControls = as_biff_id(645),
    BrtBeginPCDSDTCEMembersSortBy = as_biff_id(646),
    BrtBeginCellIgnoreECs = as_biff_id(648),
    BrtCellIgnoreEC = as_biff_id(649),
    BrtEndCellIgnoreECs = as_biff_id(650),
    BrtCsProp = as_biff_id(651),
    BrtCsPageSetup = as_biff_id(652),
    BrtBeginUserCsViews = as_biff_id(653),
    BrtEndUserCsViews = as_biff_id(654),
    BrtBeginUserCsView = as_biff_id(655),
    BrtEndUserCsView = as_biff_id(656),
    BrtBeginPcdSFCIEntries = as_biff_id(657),
    BrtEndPCDSFCIEntries = as_biff_id(658),
    BrtPCDSFCIEntry = as_biff_id(659),
    BrtBeginListParts = as_biff_id(660),
    BrtListPart = as_biff_id(661),
    BrtEndListParts = as_biff_id(662),
    BrtSheetCalcProp = as_biff_id(663),
    BrtBeginFnGroup = as_biff_id(664),
    BrtFnGroup = as_biff_id(665),
    BrtEndFnGroup = as_biff_id(666),
    BrtSupAddin = as_biff_id(667),
    BrtSXTDMPOrder = as_biff_id(668),
    BrtCsProtection = as_biff_id(669),
    BrtBeginWsSortMap = as_biff_id(671),
    BrtEndWsSortMap = as_biff_id(672),
    BrtBeginRRSort = as_biff_id(673),
    BrtEndRRSort = as_biff_id(674),
    BrtRRSortItem = as_biff_id(675),
    BrtFileSharingIso = as_biff_id(676),
    BrtBookProtectionIso = as_biff_id(677),
    BrtSheetProtectionIso = as_biff_id(678),
    BrtCsProtectionIso = as_biff_id(679),
    BrtRangeProtectionIso = as_biff_id(680),
    BrtDValList = as_biff_id(681),
    BrtRwDescent = as_biff_id(1024),
    BrtKnownFonts = as_biff_id(1025),
    BrtBeginSXTupleSet = as_biff_id(1026),
    BrtEndSXTupleSet = as_biff_id(1027),
    BrtBeginSXTupleSetHeader = as_biff_id(1028),
    BrtEndSXTupleSetHeader = as_biff_id(1029),
    BrtSXTupleSetHeaderItem = as_biff_id(1030),
    BrtBeginSXTupleSetData = as_biff_id(1031),
    BrtEndSXTupleSetData = as_biff_id(1032),
    BrtBeginSXTupleSetRow = as_biff_id(1033),
    BrtEndSXTupleSetRow = as_biff_id(1034),
    BrtSXTupleSetRowItem = as_biff_id(1035),
    BrtNameExt = as_biff_id(1036),
    BrtPCDH14 = as_biff_id(1037),
    BrtBeginPCDCalcMem14 = as_biff_id(1038),
    BrtEndPCDCalcMem14 = as_biff_id(1039),
    BrtSXTH14 = as_biff_id(1040),
    BrtBeginSparklineGroup = as_biff_id(1041),
    BrtEndSparklineGroup = as_biff_id(1042),
    BrtSparkline = as_biff_id(1043),
    BrtSXDI14 = as_biff_id(1044),
    BrtWsFmtInfoEx14 = as_biff_id(1045),
    BrtBeginConditionalFormatting14 = as_biff_id(1046),
    BrtEndConditionalFormatting14 = as_biff_id(1047),
    BrtBeginCFRule14 = as_biff_id(1048),
    BrtEndCFRule14 = as_biff_id(1049),
    BrtCFVO14 = as_biff_id(1050),
    BrtBeginDatabar14 = as_biff_id(1051),
    BrtBeginIconSet14 = as_biff_id(1052),
    BrtDVal14 = as_biff_id(1053),
    BrtBeginDVals14 = as_biff_id(1054),
    BrtColor14 = as_biff_id(1055),
    BrtBeginSparklines = as_biff_id(1056),
    BrtEndSparklines = as_biff_id(1057),
    BrtBeginSparklineGroups = as_biff_id(1058),
    BrtEndSparklineGroups = as_biff_id(1059),
    BrtSXVD14 = as_biff_id(1061),
    BrtBeginSxView14 = as_biff_id(1062),
    BrtEndSxView14 = as_biff_id(1063),
    BrtBeginSXView16 = as_biff_id(1064),
    BrtEndSXView16 = as_biff_id(1065),
    BrtBeginPCD14 = as_biff_id(1066),
    BrtEndPCD14 = as_biff_id(1067),
    BrtBeginExtConn14 = as_biff_id(1068),
    BrtEndExtConn14 = as_biff_id(1069),
    BrtBeginSlicerCacheIDs = as_biff_id(1070),
    BrtEndSlicerCacheIDs = as_biff_id(1071),
    BrtBeginSlicerCacheID = as_biff_id(1072),
    BrtEndSlicerCacheID = as_biff_id(1073),
    BrtBeginSlicerCache = as_biff_id(1075),
    BrtEndSlicerCache = as_biff_id(1076),
    BrtBeginSlicerCacheDef = as_biff_id(1077),
    BrtEndSlicerCacheDef = as_biff_id(1078),
    BrtBeginSlicersEx = as_biff_id(1079),
    BrtEndSlicersEx = as_biff_id(1080),
    BrtBeginSlicerEx = as_biff_id(1081),
    BrtEndSlicerEx = as_biff_id(1082),
    BrtBeginSlicer = as_biff_id(1083),
    BrtEndSlicer = as_biff_id(1084),
    BrtSlicerCachePivotTables = as_biff_id(1085),
    BrtBeginSlicerCacheOlapImpl = as_biff_id(1086),
    BrtEndSlicerCacheOlapImpl = as_biff_id(1087),
    BrtBeginSlicerCacheLevelsData = as_biff_id(1088),
    BrtEndSlicerCacheLevelsData = as_biff_id(1089),
    BrtBeginSlicerCacheLevelData = as_biff_id(1090),
    BrtEndSlicerCacheLevelData = as_biff_id(1091),
    BrtBeginSlicerCacheSiRanges = as_biff_id(1092),
    BrtEndSlicerCacheSiRanges = as_biff_id(1093),
    BrtBeginSlicerCacheSiRange = as_biff_id(1094),
    BrtEndSlicerCacheSiRange = as_biff_id(1095),
    BrtSlicerCacheOlapItem = as_biff_id(1096),
    BrtBeginSlicerCacheSelections = as_biff_id(1097),
    BrtSlicerCacheSelection = as_biff_id(1098),
    BrtEndSlicerCacheSelections = as_biff_id(1099),
    BrtBeginSlicerCacheNative = as_biff_id(1100),
    BrtEndSlicerCacheNative = as_biff_id(1101),
    BrtSlicerCacheNativeItem = as_biff_id(1102),
    BrtRangeProtection14 = as_biff_id(1103),
    BrtRangeProtectionIso14 = as_biff_id(1104),
    BrtCellIgnoreEC14 = as_biff_id(1105),
    BrtList14 = as_biff_id(1111),
    BrtCFIcon = as_biff_id(1112),
    BrtBeginSlicerCachesPivotCacheIDs = as_biff_id(1113),
    BrtEndSlicerCachesPivotCacheIDs = as_biff_id(1114),
    BrtBeginSlicers = as_biff_id(1115),
    BrtEndSlicers = as_biff_id(1116),
    BrtWbProp14 = as_biff_id(1117),
    BrtBeginSXEdit = as_biff_id(1118),
    BrtEndSXEdit = as_biff_id(1119),
    BrtBeginSXEdits = as_biff_id(1120),
    BrtEndSXEdits = as_biff_id(1121),
    BrtBeginSXChange = as_biff_id(1122),
    BrtEndSXChange = as_biff_id(1123),
    BrtBeginSXChanges = as_biff_id(1124),
    BrtEndSXChanges = as_biff_id(1125),
    BrtSXTupleItems = as_biff_id(1126),
    BrtBeginSlicerStyle = as_biff_id(1128),
    BrtEndSlicerStyle = as_biff_id(1129),
    BrtSlicerStyleElement = as_biff_id(1130),
    BrtBeginStyleSheetExt14 = as_biff_id(1131),
    BrtEndStyleSheetExt14 = as_biff_id(1132),
    BrtBeginSlicerCachesPivotCacheID = as_biff_id(1133),
    BrtEndSlicerCachesPivotCacheID = as_biff_id(1134),
    BrtBeginConditionalFormattings = as_biff_id(1135),
    BrtEndConditionalFormattings = as_biff_id(1136),
    BrtBeginPCDCalcMemExt = as_biff_id(1137),
    BrtEndPCDCalcMemExt = as_biff_id(1138),
    BrtBeginPCDCalcMemsExt = as_biff_id(1139),
    BrtEndPCDCalcMemsExt = as_biff_id(1140),
    BrtPCDField14 = as_biff_id(1141),
    BrtBeginSlicerStyles = as_biff_id(1142),
    BrtEndSlicerStyles = as_biff_id(1143),
    BrtBeginSlicerStyleElements = as_biff_id(1144),
    BrtEndSlicerStyleElements = as_biff_id(1145),
    BrtCFRuleExt = as_biff_id(1146),
    BrtBeginSXCondFmt14 = as_biff_id(1147),
    BrtEndSXCondFmt14 = as_biff_id(1148),
    BrtBeginSXCondFmts14 = as_biff_id(1149),
    BrtEndSXCondFmts14 = as_biff_id(1150),
    BrtBeginSortCond14 = as_biff_id(1152),
    BrtEndSortCond14 = as_biff_id(1153),
    BrtEndDVals14 = as_biff_id(1154),
    BrtEndIconSet14 = as_biff_id(1155),
    BrtEndDatabar14 = as_biff_id(1156),
    BrtBeginColorScale14 = as_biff_id(1157),
    BrtEndColorScale14 = as_biff_id(1158),
    BrtBeginSxrules14 = as_biff_id(1159),
    BrtEndSxrules14 = as_biff_id(1160),
    BrtBeginPRule14 = as_biff_id(1161),
    BrtEndPRule14 = as_biff_id(1162),
    BrtBeginPRFilters14 = as_biff_id(1163),
    BrtEndPRFilters14 = as_biff_id(1164),
    BrtBeginPRFilter14 = as_biff_id(1165),
    BrtEndPRFilter14 = as_biff_id(1166),
    BrtBeginPRFItem14 = as_biff_id(1167),
    BrtEndPRFItem14 = as_biff_id(1168),
    BrtBeginCellIgnoreECs14 = as_biff_id(1169),
    BrtEndCellIgnoreECs14 = as_biff_id(1170),
    BrtDxf14 = as_biff_id(1171),
    BrtBeginDxF14s = as_biff_id(1172),
    BrtEndDxf14s = as_biff_id(1173),
    BrtFilter14 = as_biff_id(1177),
    BrtBeginCustomFilters14 = as_biff_id(1178),
    BrtCustomFilter14 = as_biff_id(1180),
    BrtIconFilter14 = as_biff_id(1181),
    BrtPivotCacheConnectionName = as_biff_id(1182),
    BrtBeginDecoupledPivotCacheIDs = as_biff_id(2048),
    BrtEndDecoupledPivotCacheIDs = as_biff_id(2049),
    BrtDecoupledPivotCacheID = as_biff_id(2050),
    BrtBeginPivotTableRefs = as_biff_id(2051),
    BrtEndPivotTableRefs = as_biff_id(2052),
    BrtPivotTableRef = as_biff_id(2053),
    BrtSlicerCacheBookPivotTables = as_biff_id(2054),
    BrtBeginSxvcells = as_biff_id(2055),
    BrtEndSxvcells = as_biff_id(2056),
    BrtBeginSxRow = as_biff_id(2057),
    BrtEndSxRow = as_biff_id(2058),
    BrtPcdCalcMem15 = as_biff_id(2060),
    BrtQsi15 = as_biff_id(2067),
    BrtBeginWebExtensions = as_biff_id(2068),
    BrtEndWebExtensions = as_biff_id(2069),
    BrtWebExtension = as_biff_id(2070),
    BrtAbsPath15 = as_biff_id(2071),
    BrtBeginPivotTableUISettings = as_biff_id(2072),
    BrtEndPivotTableUISettings = as_biff_id(2073),
    BrtTableSlicerCacheIDs = as_biff_id(2075),
    BrtTableSlicerCacheID = as_biff_id(2076),
    BrtBeginTableSlicerCache = as_biff_id(2077),
    BrtEndTableSlicerCache = as_biff_id(2078),
    BrtSxFilter15 = as_biff_id(2079),
    BrtBeginTimelineCachePivotCacheIDs = as_biff_id(2080),
    BrtEndTimelineCachePivotCacheIDs = as_biff_id(2081),
    BrtTimelineCachePivotCacheID = as_biff_id(2082),
    BrtBeginTimelineCacheIDs = as_biff_id(2083),
    BrtEndTimelineCacheIDs = as_biff_id(2084),
    BrtBeginTimelineCacheID = as_biff_id(2085),
    BrtEndTimelineCacheID = as_biff_id(2086),
    BrtBeginTimelinesEx = as_biff_id(2087),
    BrtEndTimelinesEx = as_biff_id(2088),
    BrtBeginTimelineEx = as_biff_id(2089),
    BrtEndTimelineEx = as_biff_id(2090),
    BrtWorkBookPr15 = as_biff_id(2091),
    BrtPCDH15 = as_biff_id(2092),
    BrtBeginTimelineStyle = as_biff_id(2093),
    BrtEndTimelineStyle = as_biff_id(2094),
    BrtTimelineStyleElement = as_biff_id(2095),
    BrtBeginTimelineStylesheetExt15 = as_biff_id(2096),
    BrtEndTimelineStylesheetExt15 = as_biff_id(2097),
    BrtBeginTimelineStyles = as_biff_id(2098),
    BrtEndTimelineStyles = as_biff_id(2099),
    BrtBeginTimelineStyleElements = as_biff_id(2100),
    BrtEndTimelineStyleElements = as_biff_id(2101),
    BrtDxf15 = as_biff_id(2102),
    BrtBeginDxfs15 = as_biff_id(2103),
    BrtEndDXFs15 = as_biff_id(2104),
    BrtSlicerCacheHideItemsWithNoData = as_biff_id(2105),
    BrtBeginItemUniqueNames = as_biff_id(2106),
    BrtEndItemUniqueNames = as_biff_id(2107),
    BrtItemUniqueName = as_biff_id(2108),
    BrtBeginExtConn15 = as_biff_id(2109),
    BrtEndExtConn15 = as_biff_id(2110),
    BrtBeginOledbPr15 = as_biff_id(2111),
    BrtEndOledbPr15 = as_biff_id(2112),
    BrtBeginDataFeedPr15 = as_biff_id(2113),
    BrtEndDataFeedPr15 = as_biff_id(2114),
    BrtTextPr15 = as_biff_id(2115),
    BrtRangePr15 = as_biff_id(2116),
    BrtDbCommand15 = as_biff_id(2117),
    BrtBeginDbTables15 = as_biff_id(2118),
    BrtEndDbTables15 = as_biff_id(2119),
    BrtDbTable15 = as_biff_id(2120),
    BrtBeginDataModel = as_biff_id(2121),
    BrtEndDataModel = as_biff_id(2122),
    BrtBeginModelTables = as_biff_id(2123),
    BrtEndModelTables = as_biff_id(2124),
    BrtModelTable = as_biff_id(2125),
    BrtBeginModelRelationships = as_biff_id(2126),
    BrtEndModelRelationships = as_biff_id(2127),
    BrtModelRelationship = as_biff_id(2128),
    BrtBeginECTxtWiz15 = as_biff_id(2129),
    BrtEndECTxtWiz15 = as_biff_id(2130),
    BrtBeginECTWFldInfoLst15 = as_biff_id(2131),
    BrtEndECTWFldInfoLst15 = as_biff_id(2132),
    BrtBeginECTWFldInfo15 = as_biff_id(2133),
    BrtFieldListActiveItem = as_biff_id(2134),
    BrtPivotCacheIdVersion = as_biff_id(2135),
    BrtSXDI15 = as_biff_id(2136),
    brtBeginModelTimeGroupings = as_biff_id(2137),
    brtEndModelTimeGroupings = as_biff_id(2138),
    brtBeginModelTimeGrouping = as_biff_id(2139),
    brtEndModelTimeGrouping = as_biff_id(2140),
    brtModelTimeGroupingCalcCol = as_biff_id(2141),
    brtRevisionPtr = as_biff_id(3073),
    BrtBeginDynamicArrayPr = as_biff_id(4096),
    BrtEndDynamicArrayPr = as_biff_id(4097),
    BrtBeginRichValueBlock = as_biff_id(5002),
    BrtEndRichValueBlock = as_biff_id(5003),
    BrtBeginRichFilters = as_biff_id(5081),
    BrtEndRichFilters = as_biff_id(5082),
    BrtRichFilter = as_biff_id(5083),
    BrtBeginRichFilterColumn = as_biff_id(5084),
    BrtEndRichFilterColumn = as_biff_id(5085),
    BrtBeginCustomRichFilters = as_biff_id(5086),
    BrtEndCustomRichFilters = as_biff_id(5087),
    BRTCustomRichFilter = as_biff_id(5088),
    BrtTop10RichFilter = as_biff_id(5089),
    BrtDynamicRichFilter = as_biff_id(5090),
    BrtBeginRichSortCondition = as_biff_id(5092),
    BrtEndRichSortCondition = as_biff_id(5093),
    BrtRichFilterDateGroupItem = as_biff_id(5094),
    BrtBeginCalcFeatures = as_biff_id(5095),
    BrtEndCalcFeatures = as_biff_id(5096),
    BrtCalcFeature = as_biff_id(5097),
    BrtExternalLinksPr = as_biff_id(5099),
    BrtPivotCacheImplicitMeasureSupport = as_biff_id(5100),
    BrtPivotFieldIgnorableAfter = as_biff_id(5101),
    BrtPivotHierarchyIgnorableAfter = as_biff_id(5102),
    BrtPivotDataFieldFutureData = as_biff_id(5103),
    BrtPivotCacheRichData = as_biff_id(5105),
    BrtExternalLinksAlternateUrls = as_biff_id(5108),
    BrtBeginPivotVersionInfo = as_biff_id(5109),
    BrtEndPivotVersionInfo = as_biff_id(5110),
    BrtBeginCacheVersionInfo = as_biff_id(5111),
    // BrtBeginCacheVersionInfo = as_biff_id(5112),
    BrtPivotRequiredFeature = as_biff_id(5113),
    BrtPivotLastUsedFeature = as_biff_id(5114),
    BrtExternalCodeService = as_biff_id(5117),
    BrtSXDIAggregation = as_biff_id(5130),
    BrtPivotFieldFeatureSupportInfo = as_biff_id(5131),
    BrtPivotCacheAutoRefresh = as_biff_id(5132),
}

impl From<u16> for BiffId {
    #[inline]
    fn from(id: u16) -> Self {
        // unsafe, because the enumeration purposed for internal usage only
        unsafe { std::mem::transmute(id) }
    }
}

impl Into<u16> for BiffId {
    #[inline]
    fn into(self) -> u16 {
        self as u16
    }
}

impl Into<Box<[u8]>> for BiffId {
    fn into(self) -> Box<[u8]> {
        let id = self as u16;

        if id & 0x80 != 0 {
            Box::from(id.to_le_bytes())
        } else {
            Box::from([id as u8])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_id() {
        assert_eq!(BiffId::from(as_biff_id(0)), BiffId::BrtRowHdr);
        // assert_eq!(as_biff_id(33), BiffId::BrtPCRRecord.into());
    }
}
