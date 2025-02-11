#![allow(non_snake_case)]

use deku::{DekuRead, DekuWrite};

#[derive(DekuRead, DekuWrite, Debug, PartialEq, PartialOrd, Eq, Ord)]
#[repr(u16)]
#[deku(id_type = "u16")]
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
pub(crate) enum Cetab {
    BEEP = 0x0000,
    OPEN = 0x0001,
    OPEN_LINKS = 0x0002,
    CLOSE_ALL = 0x0003,
    SAVE = 0x0004,
    SAVE_AS = 0x0005,
    FILE_DELETE = 0x0006,
    PAGE_SETUP = 0x0007,
    PRINT = 0x0008,
    PRINTER_SETUP = 0x0009,
    QUIT = 0x000A,
    NEW_WINDOW = 0x000B,
    ARRANGE_ALL = 0x000C,
    WINDOW_SIZE = 0x000D,
    WINDOW_MOVE = 0x000E,
    FULL = 0x000F,
    CLOSE = 0x0010,
    RUN = 0x0011,
    SET_PRINT_AREA = 0x0016,
    SET_PRINT_TITLES = 0x0017,
    SET_PAGE_BREAK = 0x0018,
    REMOVE_PAGE_BREAK = 0x0019,
    FONT = 0x001A,
    DISPLAY = 0x001B,
    PROTECT_DOCUMENT = 0x001C,
    PRECISION = 0x001D,
    A1_R1C1 = 0x001E,
    CALCULATE_NOW = 0x001F,
    CALCULATION = 0x0020,
    DATA_FIND = 0x0022,
    EXTRACT = 0x0023,
    DATA_DELETE = 0x0024,
    SET_DATABASE = 0x0025,
    SET_CRITERIA = 0x0026,
    SORT = 0x0027,
    DATA_SERIES = 0x0028,
    TABLE = 0x0029,
    FORMAT_NUMBER = 0x002A,
    ALIGNMENT = 0x002B,
    STYLE = 0x002C,
    BORDER = 0x002D,
    CELL_PROTECTION = 0x002E,
    COLUMN_WIDTH = 0x002F,
    UNDO = 0x0030,
    CUT = 0x0031,
    COPY = 0x0032,
    PASTE = 0x0033,
    CLEAR = 0x0034,
    PASTE_SPECIAL = 0x0035,
    EDIT_DELETE = 0x0036,
    INSERT = 0x0037,
    FILL_RIGHT = 0x0038,
    FILL_DOWN = 0x0039,
    DEFINE_NAME = 0x003D,
    CREATE_NAMES = 0x003E,
    FORMULA_GOTO = 0x003F,
    FORMULA_FIND = 0x0040,
    SELECT_LAST_CELL = 0x0041,
    SHOW_ACTIVE_CELL = 0x0042,
    GALLERY_AREA = 0x0043,
    GALLERY_BAR = 0x0044,
    GALLERY_COLUMN = 0x0045,
    GALLERY_LINE = 0x0046,
    GALLERY_PIE = 0x0047,
    GALLERY_SCATTER = 0x0048,
    COMBINATION = 0x0049,
    PREFERRED = 0x004A,
    ADD_OVERLAY = 0x004B,
    GRIDLINES = 0x004C,
    SET_PREFERRED = 0x004D,
    AXES = 0x004E,
    LEGEND = 0x004F,
    ATTACH_TEXT = 0x0050,
    ADD_ARROW = 0x0051,
    SELECT_CHART = 0x0052,
    SELECT_PLOT_AREA = 0x0053,
    PATTERNS = 0x0054,
    MAIN_CHART = 0x0055,
    OVERLAY = 0x0056,
    SCALE = 0x0057,
    FORMAT_LEGEND = 0x0058,
    FORMAT_TEXT = 0x0059,
    EDIT_REPEAT = 0x005A,
    PARSE = 0x005B,
    JUSTIFY = 0x005C,
    HIDE = 0x005D,
    UNHIDE = 0x005E,
    WORKSPACE = 0x005F,
    FORMULA = 0x0060,
    FORMULA_FILL = 0x0061,
    FORMULA_ARRAY = 0x0062,
    DATA_FIND_NEXT = 0x0063,
    DATA_FIND_PREV = 0x0064,
    FORMULA_FIND_NEXT = 0x0065,
    FORMULA_FIND_PREV = 0x0066,
    ACTIVATE = 0x0067,
    ACTIVATE_NEXT = 0x0068,
    ACTIVATE_PREV = 0x0069,
    UNLOCKED_NEXT = 0x006A,
    UNLOCKED_PREV = 0x006B,
    COPY_PICTURE = 0x006C,
    SELECT = 0x006D,
    DELETE_NAME = 0x006E,
    DELETE_FORMAT = 0x006F,
    VLINE = 0x0070,
    HLINE = 0x0071,
    VPAGE = 0x0072,
    HPAGE = 0x0073,
    VSCROLL = 0x0074,
    HSCROLL = 0x0075,
    ALERT = 0x0076,
    NEW = 0x0077,
    CANCEL_COPY = 0x0078,
    SHOW_CLIPBOARD = 0x0079,
    MESSAGE = 0x007A,
    PASTE_LINK = 0x007C,
    APP_ACTIVATE = 0x007D,
    DELETE_ARROW = 0x007E,
    ROW_HEIGHT = 0x007F,
    FORMAT_MOVE = 0x0080,
    FORMAT_SIZE = 0x0081,
    FORMULA_REPLACE = 0x0082,
    SEND_KEYS = 0x0083,
    SELECT_SPECIAL = 0x0084,
    APPLY_NAMES = 0x0085,
    REPLACE_FONT = 0x0086,
    FREEZE_PANES = 0x0087,
    SHOW_INFO = 0x0088,
    SPLIT = 0x0089,
    ON_WINDOW = 0x008A,
    ON_DATA = 0x008B,
    DISABLE_INPUT = 0x008C,
    OUTLINE = 0x008E,
    LIST_NAMES = 0x008F,
    FILE_CLOSE = 0x0090,
    SAVE_WORKBOOK = 0x0091,
    DATA_FORM = 0x0092,
    COPY_CHART = 0x0093,
    ON_TIME = 0x0094,
    WAIT = 0x0095,
    FORMAT_FONT = 0x0096,
    FILL_UP = 0x0097,
    FILL_LEFT = 0x0098,
    DELETE_OVERLAY = 0x0099,
    SHORT_MENUS = 0x009B,
    SET_UPDATE_STATUS = 0x009F,
    COLOR_PALETTE = 0x00A1,
    DELETE_STYLE = 0x00A2,
    WINDOW_RESTORE = 0x00A3,
    WINDOW_MAXIMIZE = 0x00A4,
    CHANGE_LINK = 0x00A6,
    CALCULATE_DOCUMENT = 0x00A7,
    ON_KEY = 0x00A8,
    APP_RESTORE = 0x00A9,
    APP_MOVE = 0x00AA,
    APP_SIZE = 0x00AB,
    APP_MINIMIZE = 0x00AC,
    APP_MAXIMIZE = 0x00AD,
    BRING_TO_FRONT = 0x00AE,
    SEND_TO_BACK = 0x00AF,
    MAIN_CHART_TYPE = 0x00B9,
    OVERLAY_CHART_TYPE = 0x00BA,
    SELECT_END = 0x00BB,
    OPEN_MAIL = 0x00BC,
    SEND_MAIL = 0x00BD,
    STANDARD_FONT = 0x00BE,
    CONSOLIDATE = 0x00BF,
    SORT_SPECIAL = 0x00C0,
    GALLERY_3D_AREA = 0x00C1,
    GALLERY_3D_COLUMN = 0x00C2,
    GALLERY_3D_LINE = 0x00C3,
    GALLERY_3D_PIE = 0x00C4,
    VIEW_3D = 0x00C5,
    GOAL_SEEK = 0x00C6,
    WORKGROUP = 0x00C7,
    FILL_GROUP = 0x00C8,
    UPDATE_LINK = 0x00C9,
    PROMOTE = 0x00CA,
    DEMOTE = 0x00CB,
    SHOW_DETAIL = 0x00CC,
    UNGROUP = 0x00CE,
    OBJECT_PROPERTIES = 0x00CF,
    SAVE_NEW_OBJECT = 0x00D0,
    SHARE = 0x00D1,
    SHARE_NAME = 0x00D2,
    DUPLICATE = 0x00D3,
    APPLY_STYLE = 0x00D4,
    ASSIGN_TO_OBJECT = 0x00D5,
    OBJECT_PROTECTION = 0x00D6,
    HIDE_OBJECT = 0x00D7,
    SET_EXTRACT = 0x00D8,
    CREATE_PUBLISHER = 0x00D9,
    SUBSCRIBE_TO = 0x00DA,
    ATTRIBUTES = 0x00DB,
    SHOW_TOOLBAR = 0x00DC,
    PRINT_PREVIEW = 0x00DE,
    EDIT_COLOR = 0x00DF,
    SHOW_LEVELS = 0x00E0,
    FORMAT_MAIN = 0x00E1,
    FORMAT_OVERLAY = 0x00E2,
    ON_RECALC = 0x00E3,
    EDIT_SERIES = 0x00E4,
    DEFINE_STYLE = 0x00E5,
    LINE_PRINT = 0x00F0,
    ENTER_DATA = 0x00F3,
    GALLERY_RADAR = 0x00F9,
    MERGE_STYLES = 0x00FA,
    EDITION_OPTIONS = 0x00FB,
    PASTE_PICTURE = 0x00FC,
    PASTE_PICTURE_LINK = 0x00FD,
    SPELLING = 0x00FE,
    ZOOM = 0x0100,
    INSERT_OBJECT = 0x0103,
    WINDOW_MINIMIZE = 0x0104,
    SOUND_NOTE = 0x0109,
    SOUND_PLAY = 0x010A,
    FORMAT_SHAPE = 0x010B,
    EXTEND_POLYGON = 0x010C,
    FORMAT_AUTO = 0x010D,
    GALLERY_3D_BAR = 0x0110,
    GALLERY_3D_SURFACE = 0x0111,
    FILL_AUTO = 0x0112,
    CUSTOMIZE_TOOLBAR = 0x0114,
    ADD_TOOL = 0x0115,
    EDIT_OBJECT = 0x0116,
    ON_DOUBLECLICK = 0x0117,
    ON_ENTRY = 0x0118,
    WORKBOOK_ADD = 0x0119,
    WORKBOOK_MOVE = 0x011A,
    WORKBOOK_COPY = 0x011B,
    WORKBOOK_OPTIONS = 0x011C,
    SAVE_WORKSPACE = 0x011D,
    CHART_WIZARD = 0x0120,
    DELETE_TOOL = 0x0121,
    MOVE_TOOL = 0x0122,
    WORKBOOK_SELECT = 0x0123,
    WORKBOOK_ACTIVATE = 0x0124,
    ASSIGN_TO_TOOL = 0x0125,
    COPY_TOOL = 0x0127,
    RESET_TOOL = 0x0128,
    CONSTRAIN_NUMERIC = 0x0129,
    PASTE_TOOL = 0x012A,
    WORKBOOK_NEW = 0x012E,
    SCENARIO_CELLS = 0x0131,
    SCENARIO_DELETE = 0x0132,
    SCENARIO_ADD = 0x0133,
    SCENARIO_EDIT = 0x0134,
    SCENARIO_SHOW = 0x0135,
    SCENARIO_SHOW_NEXT = 0x0136,
    SCENARIO_SUMMARY = 0x0137,
    PIVOT_TABLE_WIZARD = 0x0138,
    PIVOT_FIELD_PROPERTIES = 0x0139,
    PIVOT_FIELD = 0x013A,
    PIVOT_ITEM = 0x013B,
    PIVOT_ADD_FIELDS = 0x013C,
    OPTIONS_CALCULATION = 0x013E,
    OPTIONS_EDIT = 0x013F,
    OPTIONS_VIEW = 0x0140,
    ADDIN_MANAGER = 0x0141,
    MENU_EDITOR = 0x0142,
    ATTACH_TOOLBARS = 0x0143,
    VBAActivate = 0x0144,
    OPTIONS_CHART = 0x0145,
    VBA_INSERT_FILE = 0x0148,
    VBA_PROCEDURE_DEFINITION = 0x014A,
    ROUTING_SLIP = 0x0150,
    ROUTE_DOCUMENT = 0x0152,
    MAIL_LOGON = 0x0153,
    INSERT_PICTURE = 0x0156,
    EDIT_TOOL = 0x0157,
    GALLERY_DOUGHNUT = 0x0158,
    CHART_TREND = 0x015E,
    PIVOT_ITEM_PROPERTIES = 0x0160,
    WORKBOOK_INSERT = 0x0162,
    OPTIONS_TRANSITION = 0x0163,
    OPTIONS_GENERAL = 0x0164,
    FILTER_ADVANCED = 0x0172,
    MAIL_ADD_MAILER = 0x0175,
    MAIL_DELETE_MAILER = 0x0176,
    MAIL_REPLY = 0x0177,
    MAIL_REPLY_ALL = 0x0178,
    MAIL_FORWARD = 0x0179,
    MAIL_NEXT_LETTER = 0x017A,
    DATA_LABEL = 0x017B,
    INSERT_TITLE = 0x017C,
    FONT_PROPERTIES = 0x017D,
    MACRO_OPTIONS = 0x017E,
    WORKBOOK_HIDE = 0x017F,
    WORKBOOK_UNHIDE = 0x0180,
    WORKBOOK_DELETE = 0x0181,
    WORKBOOK_NAME = 0x0182,
    GALLERY_CUSTOM = 0x0184,
    ADD_CHART_AUTOFORMAT = 0x0186,
    DELETE_CHART_AUTOFORMAT = 0x0187,
    CHART_ADD_DATA = 0x0188,
    AUTO_OUTLINE = 0x0189,
    TAB_ORDER = 0x018A,
    SHOW_DIALOG = 0x018B,
    SELECT_ALL = 0x018C,
    UNGROUP_SHEETS = 0x018D,
    SUBTOTAL_CREATE = 0x018E,
    SUBTOTAL_REMOVE = 0x018F,
    RENAME_OBJECT = 0x0190,
    WORKBOOK_SCROLL = 0x019C,
    WORKBOOK_NEXT = 0x019D,
    WORKBOOK_PREV = 0x019E,
    WORKBOOK_TAB_SPLIT = 0x019F,
    FULL_SCREEN = 0x01A0,
    WORKBOOK_PROTECT = 0x01A1,
    SCROLLBAR_PROPERTIES = 0x01A4,
    PIVOT_SHOW_PAGES = 0x01A5,
    TEXT_TO_COLUMNS = 0x01A6,
    FORMAT_CHARTTYPE = 0x01A7,
    LINK_FORMAT = 0x01A8,
    TRACER_DISPLAY = 0x01A9,
    TRACER_NAVIGATE = 0x01AE,
    TRACER_CLEAR = 0x01AF,
    TRACER_ERROR = 0x01B0,
    PIVOT_FIELD_GROUP = 0x01B1,
    PIVOT_FIELD_UNGROUP = 0x01B2,
    CHECKBOX_PROPERTIES = 0x01B3,
    LABEL_PROPERTIES = 0x01B4,
    LISTBOX_PROPERTIES = 0x01B5,
    EDITBOX_PROPERTIES = 0x01B6,
    PIVOT_REFRESH = 0x01B7,
    LINK_COMBO = 0x01B8,
    OPEN_TEXT = 0x01B9,
    HIDE_DIALOG = 0x01BA,
    SET_DIALOG_FOCUS = 0x01BB,
    ENABLE_OBJECT = 0x01BC,
    PUSHBUTTON_PROPERTIES = 0x01BD,
    SET_DIALOG_DEFAULT = 0x01BE,
    FILTER = 0x01BF,
    FILTER_SHOW_ALL = 0x01C0,
    CLEAR_OUTLINE = 0x01C1,
    FUNCTION_WIZARD = 0x01C2,
    ADD_LIST_ITEM = 0x01C3,
    SET_LIST_ITEM = 0x01C4,
    REMOVE_LIST_ITEM = 0x01C5,
    SELECT_LIST_ITEM = 0x01C6,
    SET_CONTROL_VALUE = 0x01C7,
    SAVE_COPY_AS = 0x01C8,
    OPTIONS_LISTS_ADD = 0x01CA,
    OPTIONS_LISTS_DELETE = 0x01CB,
    SERIES_AXES = 0x01CC,
    SERIES_X = 0x01CD,
    SERIES_Y = 0x01CE,
    ERRORBAR_X = 0x01CF,
    ERRORBAR_Y = 0x01D0,
    FORMAT_CHART = 0x01D1,
    SERIES_ORDER = 0x01D2,
    MAIL_LOGOFF = 0x01D3,
    CLEAR_ROUTING_SLIP = 0x01D4,
    APP_ACTIVATE_MICROSOFT = 0x01D5,
    MAIL_EDIT_MAILER = 0x01D6,
    ON_SHEET = 0x01D7,
    STANDARD_WIDTH = 0x01D8,
    SCENARIO_MERGE = 0x01D9,
    SUMMARY_INFO = 0x01DA,
    FIND_FILE = 0x01DB,
    ACTIVE_CELL_FONT = 0x01DC,
    ENABLE_TIPWIZARD = 0x01DD,
    VBA_MAKE_ADDIN = 0x01DE,
    INSERTDATATABLE = 0x01E0,
    WORKGROUP_OPTIONS = 0x01E1,
    MAIL_SEND_MAILER = 0x01E2,
    AUTOCORRECT = 0x01E5,
    POST_DOCUMENT = 0x01E9,
    PICKLIST = 0x01EB,
    VIEW_SHOW = 0x01ED,
    VIEW_DEFINE = 0x01EE,
    VIEW_DELETE = 0x01EF,
    SHEET_BACKGROUND = 0x01FD,
    INSERT_MAP_OBJECT = 0x01FE,
    OPTIONS_MENONO = 0x01FF,
    MSOCHECKS = 0x0205,
    NORMAL = 0x0206,
    LAYOUT = 0x0207,
    RM_PRINT_AREA = 0x0208,
    CLEAR_PRINT_AREA = 0x0209,
    ADD_PRINT_AREA = 0x020A,
    MOVE_BRK = 0x020B,
    HIDECURR_NOTE = 0x0221,
    HIDEALL_NOTES = 0x0222,
    DELETE_NOTE = 0x0223,
    TRAVERSE_NOTES = 0x0224,
    ACTIVATE_NOTES = 0x0225,
    PROTECT_REVISIONS = 0x026C,
    UNPROTECT_REVISIONS = 0x026D,
    OPTIONS_ME = 0x0287,
    WEB_PUBLISH = 0x028D,
    NEWWEBQUERY = 0x029B,
    PIVOT_TABLE_CHART = 0x02A1,
    OPTIONS_SAVE = 0x02F1,
    OPTIONS_SPELL = 0x02F3,
    HIDEALL_INKANNOTS = 0x0328,
}

#[derive(DekuRead, DekuWrite, Debug, PartialEq, PartialOrd, Eq, Ord)]
#[repr(u16)]
#[deku(id_type = "u16")]
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
pub(crate) enum Ftab {
    COUNT = 0x0000,
    IF = 0x0001,
    ISNA = 0x0002,
    ISERROR = 0x0003,
    SUM = 0x0004,
    AVERAGE = 0x0005,
    MIN = 0x0006,
    MAX = 0x0007,
    ROW = 0x0008,
    COLUMN = 0x0009,
    NA = 0x000a,
    NPV = 0x000b,
    STDEV = 0x000c,
    DOLLAR = 0x000D,
    FIXED = 0x000E,
    SIN = 0x000F,
    COS = 0x0010,
    TAN = 0x0011,
    ATAN = 0x0012,
    PI = 0x0013,
    SQRT = 0x0014,
    EXP = 0x0015,
    LN = 0x0016,
    LOG10 = 0x0017,
    ABS = 0x0018,
    INT = 0x0019,
    SIGN = 0x001A,
    ROUND = 0x001B,
    LOOKUP = 0x001C,
    INDEX = 0x001D,
    REPT = 0x001E,
    MID = 0x001F,
    LEN = 0x0020,
    VALUE = 0x0021,
    TRUE = 0x0022,
    FALSE = 0x0023,
    AND = 0x0024,
    OR = 0x0025,
    NOT = 0x0026,
    MOD = 0x0027,
    DCOUNT = 0x0028,
    DSUM = 0x0029,
    DAVERAGE = 0x002A,
    DMIN = 0x002B,
    DMAX = 0x002C,
    DSTDEV = 0x002D,
    VAR = 0x002E,
    DVAR = 0x002F,
    TEXT = 0x0030,
    LINEST = 0x0031,
    TREND = 0x0032,
    LOGEST = 0x0033,
    GROWTH = 0x0034,
    GOTO = 0x0035,
    HALT = 0x0036,
    RETURN = 0x0037,
    PV = 0x0038,
    FV = 0x0039,
    NPER = 0x003A,
    PMT = 0x003B,
    RATE = 0x003C,
    MIRR = 0x003D,
    IRR = 0x003E,
    RAND = 0x003F,
    MATCH = 0x0040,
    DATE = 0x0041,
    TIME = 0x0042,
    DAY = 0x0043,
    MONTH = 0x0044,
    YEAR = 0x0045,
    WEEKDAY = 0x0046,
    HOUR = 0x0047,
    MINUTE = 0x0048,
    SECOND = 0x0049,
    NOW = 0x004A,
    AREAS = 0x004B,
    ROWS = 0x004C,
    COLUMNS = 0x004D,
    OFFSET = 0x004E,
    ABSREF = 0x004F,
    RELREF = 0x0050,
    ARGUMENT = 0x0051,
    SEARCH = 0x0052,
    TRANSPOSE = 0x0053,
    ERROR = 0x0054,
    STEP = 0x0055,
    TYPE = 0x0056,
    ECHO = 0x0057,
    SET_NAME = 0x0058,
    CALLER = 0x0059,
    DEREF = 0x005A,
    WINDOWS = 0x005B,
    DOCUMENTS = 0x005D,
    ACTIVE_CELL = 0x005E,
    SELECTION = 0x005F,
    RESULT = 0x0060,
    ATAN2 = 0x0061,
    ASIN = 0x0062,
    ACOS = 0x0063,
    CHOOSE = 0x0064,
    HLOOKUP = 0x0065,
    VLOOKUP = 0x0066,
    LINKS = 0x0067,
    INPUT = 0x0068,
    ISREF = 0x0069,
    GET_FORMULA = 0x006A,
    GET_NAME = 0x006B,
    SET_VALUE = 0x006C,
    LOG = 0x006D,
    EXEC = 0x006E,
    CHAR = 0x006F,
    LOWER = 0x0070,
    UPPER = 0x0071,
    PROPER = 0x0072,
    LEFT = 0x0073,
    RIGHT = 0x0074,
    EXACT = 0x0075,
    TRIM = 0x0076,
    REPLACE = 0x0077,
    SUBSTITUTE = 0x0078,
    CODE = 0x0079,
    NAMES = 0x007A,
    DIRECTORY = 0x007B,
    FIND = 0x007C,
    CELL = 0x007D,
    ISERR = 0x007E,
    ISTEXT = 0x007F,
    ISNUMBER = 0x0080,
    ISBLANK = 0x0081,
    T = 0x0082,
    N = 0x0083,
    FOPEN = 0x0084,
    FCLOSE = 0x0085,
    FSIZE = 0x0086,
    FREADLN = 0x0087,
    FREAD = 0x0088,
    FWRITELN = 0x0089,
    FWRITE = 0x008A,
    FPOS = 0x008B,
    DATEVALUE = 0x008C,
    TIMEVALUE = 0x008D,
    SLN = 0x008E,
    SYD = 0x008F,
    DDB = 0x0090,
    GET_DEF = 0x0091,
    REFTEXT = 0x0092,
    TEXTREF = 0x0093,
    INDIRECT = 0x0094,
    REGISTER = 0x0095,
    CALL = 0x0096,
    ADD_BAR = 0x0097,
    ADD_MENU = 0x0098,
    ADD_COMMAND = 0x0099,
    ENABLE_COMMAND = 0x009A,
    CHECK_COMMAND = 0x009B,
    RENAME_COMMAND = 0x009C,
    SHOW_BAR = 0x009D,
    DELETE_MENU = 0x009E,
    DELETE_COMMAND = 0x009F,
    GET_CHART_ITEM = 0x00A0,
    DIALOG_BOX = 0x00A1,
    CLEAN = 0x00A2,
    MDETERM = 0x00A3,
    MINVERSE = 0x00A4,
    MMULT = 0x00A5,
    FILES = 0x00A6,
    IPMT = 0x00A7,
    PPMT = 0x00A8,
    COUNTA = 0x00A9,
    CANCEL_KEY = 0x00AA,
    FOR = 0x00AB,
    WHILE = 0x00AC,
    BREAK = 0x00AD,
    NEXT = 0x00AE,
    INITIATE = 0x00AF,
    REQUEST = 0x00B0,
    POKE = 0x00B1,
    EXECUTE = 0x00B2,
    TERMINATE = 0x00B3,
    RESTART = 0x00B4,
    HELP = 0x00B5,
    GET_BAR = 0x00B6,
    PRODUCT = 0x00B7,
    FACT = 0x00B8,
    GET_CELL = 0x00B9,
    GET_WORKSPACE = 0x00BA,
    GET_WINDOW = 0x00BB,
    GET_DOCUMENT = 0x00BC,
    DPRODUCT = 0x00BD,
    ISNONTEXT = 0x00BE,
    GET_NOTE = 0x00BF,
    NOTE = 0x00C0,
    STDEVP = 0x00C1,
    VARP = 0x00C2,
    DSTDEVP = 0x00C3,
    DVARP = 0x00C4,
    TRUNC = 0x00C5,
    ISLOGICAL = 0x00C6,
    DCOUNTA = 0x00C7,
    DELETE_BAR = 0x00C8,
    UNREGISTER = 0x00C9,
    USDOLLAR = 0x00CC,
    FINDB = 0x00CD,
    SEARCHB = 0x00CE,
    REPLACEB = 0x00CF,
    LEFTB = 0x00D0,
    RIGHTB = 0x00D1,
    MIDB = 0x00D2,
    LENB = 0x00D3,
    ROUNDUP = 0x00D4,
    ROUNDDOWN = 0x00D5,
    ASC = 0x00D6,
    DBCS = 0x00D7,
    RANK = 0x00D8,
    ADDRESS = 0x00DB,
    DAYS360 = 0x00DC,
    TODAY = 0x00DD,
    VDB = 0x00DE,
    ELSE = 0x00DF,
    ELSE_IF = 0x00E0,
    END_IF = 0x00E1,
    FOR_CELL = 0x00E2,
    MEDIAN = 0x00E3,
    SUMPRODUCT = 0x00E4,
    SINH = 0x00E5,
    COSH = 0x00E6,
    TANH = 0x00E7,
    ASINH = 0x00E8,
    ACOSH = 0x00E9,
    ATANH = 0x00EA,
    DGET = 0x00EB,
    CREATE_OBJECT = 0x00EC,
    VOLATILE = 0x00ED,
    LAST_ERROR = 0x00EE,
    CUSTOM_UNDO = 0x00EF,
    CUSTOM_REPEAT = 0x00F0,
    FORMULA_CONVERT = 0x00F1,
    GET_LINK_INFO = 0x00F2,
    TEXT_BOX = 0x00F3,
    INFO = 0x00F4,
    GROUP = 0x00F5,
    GET_OBJECT = 0x00F6,
    DB = 0x00F7,
    PAUSE = 0x00F8,
    RESUME = 0x00FB,
    FREQUENCY = 0x00FC,
    ADD_TOOLBAR = 0x00FD,
    DELETE_TOOLBAR = 0x00FE,
    UDFOrFutureFunc = 0x00FF,
    RESET_TOOLBAR = 0x0100,
    EVALUATE = 0x0101,
    GET_TOOLBAR = 0x0102,
    GET_TOOL = 0x0103,
    SPELLING_CHECK = 0x0104,
    ERROR_TYPE = 0x0105,
    APP_TITLE = 0x0106,
    WINDOW_TITLE = 0x0107,
    SAVE_TOOLBAR = 0x0108,
    ENABLE_TOOL = 0x0109,
    PRESS_TOOL = 0x010A,
    REGISTER_ID = 0x010B,
    GET_WORKBOOK = 0x010C,
    AVEDEV = 0x010D,
    BETADIST = 0x010E,
    GAMMALN = 0x010F,
    BETAINV = 0x0110,
    BINOMDIST = 0x0111,
    CHIDIST = 0x0112,
    CHIINV = 0x0113,
    COMBIN = 0x0114,
    CONFIDENCE = 0x0115,
    CRITBINOM = 0x0116,
    EVEN = 0x0117,
    EXPONDIST = 0x0118,
    FDIST = 0x0119,
    FINV = 0x011A,
    FISHER = 0x011B,
    FISHERINV = 0x011C,
    FLOOR = 0x011D,
    GAMMADIST = 0x011E,
    GAMMAINV = 0x011F,
    CEILING = 0x0120,
    HYPGEOMDIST = 0x0121,
    LOGNORMDIST = 0x0122,
    LOGINV = 0x0123,
    NEGBINOMDIST = 0x0124,
    NORMDIST = 0x0125,
    NORMSDIST = 0x0126,
    NORMINV = 0x0127,
    NORMSINV = 0x0128,
    STANDARDIZE = 0x0129,
    ODD = 0x012A,
    PERMUT = 0x012B,
    POISSON = 0x012C,
    TDIST = 0x012D,
    WEIBULL = 0x012E,
    SUMXMY2 = 0x012F,
    SUMX2MY2 = 0x0130,
    SUMX2PY2 = 0x0131,
    CHITEST = 0x0132,
    CORREL = 0x0133,
    COVAR = 0x0134,
    FORECAST = 0x0135,
    FTEST = 0x0136,
    INTERCEPT = 0x0137,
    PEARSON = 0x0138,
    RSQ = 0x0139,
    STEYX = 0x013A,
    SLOPE = 0x013B,
    TTEST = 0x013C,
    PROB = 0x013D,
    DEVSQ = 0x013E,
    GEOMEAN = 0x013F,
    HARMEAN = 0x0140,
    SUMSQ = 0x0141,
    KURT = 0x0142,
    SKEW = 0x0143,
    ZTEST = 0x0144,
    LARGE = 0x0145,
    SMALL = 0x0146,
    QUARTILE = 0x0147,
    PERCENTILE = 0x0148,
    PERCENTRANK = 0x0149,
    MODE = 0x014A,
    TRIMMEAN = 0x014B,
    TINV = 0x014C,
    MOVIE_COMMAND = 0x014E,
    GET_MOVIE = 0x014F,
    CONCATENATE = 0x0150,
    POWER = 0x0151,
    PIVOT_ADD_DATA = 0x0152,
    GET_PIVOT_TABLE = 0x0153,
    GET_PIVOT_FIELD = 0x0154,
    GET_PIVOT_ITEM = 0x0155,
    RADIANS = 0x0156,
    DEGREES = 0x0157,
    SUBTOTAL = 0x0158,
    SUMIF = 0x0159,
    COUNTIF = 0x015A,
    COUNTBLANK = 0x015B,
    SCENARIO_GET = 0x015C,
    OPTIONS_LISTS_GET = 0x015D,
    ISPMT = 0x015E,
    DATEDIF = 0x015F,
    DATESTRING = 0x0160,
    NUMBERSTRING = 0x0161,
    ROMAN = 0x0162,
    OPEN_DIALOG = 0x0163,
    SAVE_DIALOG = 0x0164,
    VIEW_GET = 0x0165,
    GETPIVOTDATA = 0x0166,
    HYPERLINK = 0x0167,
    PHONETIC = 0x0168,
    AVERAGEA = 0x0169,
    MAXA = 0x016A,
    MINA = 0x016B,
    STDEVPA = 0x016C,
    VARPA = 0x016D,
    STDEVA = 0x016E,
    VARA = 0x016F,
    BAHTTEXT = 0x0170,
    THAIDAYOFWEEK = 0x0171,
    THAIDIGIT = 0x0172,
    THAIMONTHOFYEAR = 0x0173,
    THAINUMSOUND = 0x0174,
    THAINUMSTRING = 0x0175,
    THAISTRINGLENGTH = 0x0176,
    ISTHAIDIGIT = 0x0177,
    ROUNDBAHTDOWN = 0x0178,
    ROUNDBAHTUP = 0x0179,
    THAIYEAR = 0x017A,
    RTD = 0x017B,
    CUBEVALUE = 0x017C,
    CUBEMEMBER = 0x017D,
    CUBEMEMBERPROPERTY = 0x017E,
    CUBERANKEDMEMBER = 0x017F,
    HEX2BIN = 0x0180,
    HEX2DEC = 0x0181,
    HEX2OCT = 0x0182,
    DEC2BIN = 0x0183,
    DEC2HEX = 0x0184,
    DEC2OCT = 0x0185,
    OCT2BIN = 0x0186,
    OCT2HEX = 0x0187,
    OCT2DEC = 0x0188,
    BIN2DEC = 0x0189,
    BIN2OCT = 0x018A,
    BIN2HEX = 0x018B,
    IMSUB = 0x018C,
    IMDIV = 0x018D,
    IMPOWER = 0x018E,
    IMABS = 0x018F,
    IMSQRT = 0x0190,
    IMLN = 0x0191,
    IMLOG2 = 0x0192,
    IMLOG10 = 0x0193,
    IMSIN = 0x0194,
    IMCOS = 0x0195,
    IMEXP = 0x0196,
    IMARGUMENT = 0x0197,
    IMCONJUGATE = 0x0198,
    IMAGINARY = 0x0199,
    IMREAL = 0x019A,
    COMPLEX = 0x019B,
    IMSUM = 0x019C,
    IMPRODUCT = 0x019D,
    SERIESSUM = 0x019E,
    FACTDOUBLE = 0x019F,
    SQRTPI = 0x01A0,
    QUOTIENT = 0x01A1,
    DELTA = 0x01A2,
    GESTEP = 0x01A3,
    ISEVEN = 0x01A4,
    ISODD = 0x01A5,
    MROUND = 0x01A6,
    ERF = 0x01A7,
    ERFC = 0x01A8,
    BESSELJ = 0x01A9,
    BESSELK = 0x01AA,
    BESSELY = 0x01AB,
    BESSELI = 0x01AC,
    XIRR = 0x01AD,
    XNPV = 0x01AE,
    PRICEMAT = 0x01AF,
    YIELDMAT = 0x01B0,
    INTRATE = 0x01B1,
    RECEIVED = 0x01B2,
    DISC = 0x01B3,
    PRICEDISC = 0x01B4,
    YIELDDISC = 0x01B5,
    TBILLEQ = 0x01B6,
    TBILLPRICE = 0x01B7,
    TBILLYIELD = 0x01B8,
    PRICE = 0x01B9,
    YIELD = 0x01BA,
    DOLLARDE = 0x01BB,
    DOLLARFR = 0x01BC,
    NOMINAL = 0x01BD,
    EFFECT = 0x01BE,
    CUMPRINC = 0x01BF,
    CUMIPMT = 0x01C0,
    EDATE = 0x01C1,
    EOMONTH = 0x01C2,
    YEARFRAC = 0x01C3,
    COUPDAYBS = 0x01C4,
    COUPDAYS = 0x01C5,
    COUPDAYSNC = 0x01C6,
    COUPNCD = 0x01C7,
    COUPNUM = 0x01C8,
    COUPPCD = 0x01C9,
    DURATION = 0x01CA,
    MDURATION = 0x01CB,
    ODDLPRICE = 0x01CC,
    ODDLYIELD = 0x01CD,
    ODDFPRICE = 0x01CE,
    ODDFYIELD = 0x01CF,
    RANDBETWEEN = 0x01D0,
    WEEKNUM = 0x01D1,
    AMORDEGRC = 0x01D2,
    AMORLINC = 0x01D3,
    ACCRINT = 0x01D5,
    ACCRINTM = 0x01D6,
    WORKDAY = 0x01D7,
    NETWORKDAYS = 0x01D8,
    GCD = 0x01D9,
    MULTINOMIAL = 0x01DA,
    LCM = 0x01DB,
    FVSCHEDULE = 0x01DC,
    CUBEKPIMEMBER = 0x01DD,
    CUBESET = 0x01DE,
    CUBESETCOUNT = 0x01DF,
    IFERROR = 0x01E0,
    COUNTIFS = 0x01E1,
    SUMIFS = 0x01E2,
    AVERAGEIF = 0x01E3,
    AVERAGEIFS = 0x01E4,
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
pub(crate) enum Tab {
    F(Ftab),
    Ce(Cetab),
}

pub(crate) mod ptg {
    use std::borrow::Cow;
    use std::io::{Read, Seek, Write};

    use deku::DekuContainerWrite;
    use deku::writer::Writer;
    use deku::{DekuError, DekuRead, DekuReader, DekuWrite, reader::Reader};
    use zerocopy::IntoBytes;

    use crate::util::Xnum;
    use crate::{
        biff::{BErr, RgceArea, RgceAreaRel, RgceLoc, RgceLocRel},
        fmla::Cetab,
    };

    use super::{Ftab, Tab};

    #[repr(u8)]
    #[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Eq, Ord)]
    pub enum DataType {
        Ref = 0x01,
        Val = 0x02,
        Arr = 0x03,
    }

    const fn as_ptg_datatype(b: u8, o: usize) -> DataType {
        match (b >> o) & 0b11 {
            0b01 => DataType::Ref,
            0b10 => DataType::Val,
            _ => DataType::Arr,
        }
    }

    #[derive(DekuRead, DekuWrite, Debug, PartialEq, PartialOrd, Eq, Ord)]
    #[deku(id_type = "u8")]
    /// `2.5.98.32`
    pub(crate) enum AttrSpaceType {
        #[deku(id = 0x00)]
        SpaceBeforeBaseExpr(u8),
        #[deku(id = 0x01)]
        CarriageReturnBeforeBaseExpr(u8),
        #[deku(id = 0x02)]
        SpaceBeforeOpenParen(u8),
        #[deku(id = 0x03)]
        CarriageReturnBeforeOpenParen(u8),
        #[deku(id = 0x04)]
        SpaceBeforeCloseParen(u8),
        #[deku(id = 0x05)]
        CarriageReturnBeforeCloseParen(u8),
        #[deku(id = 0x06)]
        SpaceBeforeExpr,
    }

    #[derive(DekuRead, DekuWrite, Debug, PartialEq, PartialOrd, Eq, Ord)]
    #[deku(id_type = "u8", bits = 2)]
    pub(crate) enum ListColumns {
        AllColumns = 0x00,
        FirstColumn = 0x01,
        ColumnsBetween = 0x02,
    }

    #[derive(DekuRead, DekuWrite, Debug, PartialEq, PartialOrd, Eq, Ord)]
    #[deku(id_type = "u8", bits = 5)]
    pub(crate) enum RowType {
        Data = 0x00,
        All = 0x01,
        Headers = 0x02,
        Data2 = 0x03,
        DataHeaders = 0x06,
        Totals = 0x08,
        DataTotals = 0x0c,
        Current = 0x10,
    }

    #[derive(DekuRead, DekuWrite, Debug, PartialEq, PartialOrd, Eq, Ord)]
    #[deku(id_type = "u8", bits = 2)]
    pub(crate) enum ListDataType {
        Ref = 0x00,
        Val = 0x01,
        Arr = 0x02,
    }

    #[derive(DekuRead, DekuWrite, Debug, PartialEq, PartialOrd, Eq, Ord)]
    pub(crate) struct List {
        ixti: u16,
        columns: ListColumns,
        rowType: RowType,
        #[deku(bits = 1)]
        squareBracketSpace: bool,
        #[deku(bits = 1, pad_bits_after = "1")]
        commaSpace: bool,

        r#type: ListDataType,
        #[deku(bits = 1)]
        invalid: bool,

        listIndex: u32,
        colFirst: u16,
        colLast: u16,
    }

    #[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
    pub enum Ptg {
        /// `2.5.98.17`
        Add,
        /// `2.5.98.18`
        Area { r#type: DataType, area: RgceArea },
        /// `2.5.98.19`
        Area3d {
            r#type: DataType,
            ixti: u16,
            area: RgceArea,
        },

        /// `2.5.98.20`
        AreaErr { r#type: DataType },
        /// `2.5.98.21`
        AreaErr3d { r#type: DataType, ixti: u16 },
        /// `2.5.98.22`
        AreaN { r#type: DataType, area: RgceAreaRel },
        /// `2.5.98.23`
        Array { r#type: DataType },
        /// `2.5.98.24`
        AttrBaxcel { bitSemi: bool },
        /// `2.5.98.25`
        AttrChoose { cOffset: u16, rgOffset: Vec<u16> },
        /// `2.5.98.26`
        AttrGoTo { offset: u16 },
        /// `2.5.98.27`
        AttrIf { offset: u16 },
        /// `2.5.98.28`
        AttrIfError { offset: u16 },
        /// `2.5.98.29`
        AttrSemi,
        /// `2.5.98.30`
        AttrSpace { r#type: AttrSpaceType },
        /// `2.5.98.31`
        AttrSpaceSemi { r#type: AttrSpaceType },
        /// `2.5.98.33`
        AttrSum,
        /// `2.5.98.34`
        Bool(bool),
        /// `2.5.98.35`
        Concat,
        /// `2.5.98.37`
        Div,
        /// `2.5.98.38`
        Eq,
        /// `2.5.98.39`
        Err(BErr),
        /// `2.5.98.40`
        Exp { row: u32 },
        /// `2.5.98.45`
        Func { iftab: Ftab },
        /// `2.5.98.46`
        FuncVar {
            r#type: DataType,
            cparams: u8,
            tab: Tab,
        },
        /// `2.5.98.47`
        Ge,
        /// `2.5.98.48`
        Gt,
        /// `2.5.98.49`
        Int(u16),
        /// `2.5.98.50`
        Isect,
        /// `2.5.98.51`
        Le,
        /// `2.5.98.52`
        List(List),
        /// `2.5.98.53`
        Lt,
        /// `2.5.98.54`
        MemArea { r#type: DataType, cce: u16 },
        /// `2.5.98.55`
        MemErr {
            r#type: DataType,
            err: BErr,
            cce: u16,
        },
        /// `2.5.98.56`
        MemFunc { r#type: DataType, cce: u16 },
        /// `2.5.98.57`
        MemNoMem { r#type: DataType, cce: u16 },
        /// `2.5.98.58`
        MissArg,
        /// `2.5.98.59`
        Mul,
        /// `2.5.98.60`
        Name { r#type: DataType, nameindex: u32 },
        /// `2.5.98.61`
        NameX {
            r#type: DataType,
            ixti: u16,
            nameindex: u32,
        },
        /// `2.5.98.62`
        Ne,
        /// `2.5.98.63`
        Num(Xnum),
        /// `2.5.98.64`
        Paren,
        /// `2.5.98.65`
        Percent,
        /// `2.5.98.66`
        Power,
        /// `2.5.98.67`
        Range,
        /// `2.5.98.68`
        Ref { r#type: DataType, loc: RgceLoc },
        /// `2.5.98.69`
        Ref3d {
            r#type: DataType,
            ixti: u16,
            loc: RgceLoc,
        },
        /// `2.5.98.70`
        RefErr { r#type: DataType },
        /// `2.5.98.71`
        RefErr3d { r#type: DataType, ixti: u16 },
        /// `2.5.98.72`
        RefN { r#type: DataType, loc: RgceLocRel },
        /// `2.5.98.74`
        Str(String),
        /// `2.5.98.75`
        Sub,
        /// `2.5.98.76`
        SxName { sxIndex: u32 },
        /// `2.5.98.77`
        UMinus,
        /// `2.5.98.78`
        Union,
        /// `2.5.98.79`
        UPlus,
    }

    pub(crate) fn write_ptg<W: Write + Seek>(
        ptg: &Ptg,
        w: &mut Writer<W>,
    ) -> Result<(), DekuError> {
        let buf: Box<[u8]> = match ptg {
            // plain types
            Ptg::Add => Box::new([0x03]),
            Ptg::Sub => Box::new([0x04]),
            Ptg::Mul => Box::new([0x05]),
            Ptg::Div => Box::new([0x06]),
            Ptg::Power => Box::new([0x07]),
            Ptg::Concat => Box::new([0x08]),
            Ptg::Lt => Box::new([0x09]),
            Ptg::Le => Box::new([0x0A]),
            Ptg::Eq => Box::new([0x0B]),
            Ptg::Ge => Box::new([0x0C]),
            Ptg::Gt => Box::new([0x0D]),
            Ptg::Ne => Box::new([0x0E]),
            Ptg::Isect => Box::new([0x0F]),
            Ptg::Union => Box::new([0x10]),
            Ptg::Range => Box::new([0x11]),
            Ptg::UPlus => Box::new([0x12]),
            Ptg::UMinus => Box::new([0x13]),
            Ptg::Percent => Box::new([0x14]),
            Ptg::Paren => Box::new([0x15]),
            Ptg::MissArg => Box::new([0x16]),
            // simple types
            Ptg::Str(s) => {
                let mut buf = Box::new_uninit_slice(s.len() * 2 + 2 + 1);
                buf[0].write(0x17);
                buf[1..3].write_copy_of_slice(&(s.len() as u16).to_le_bytes());
                let utf_16_repr = str::encode_utf16(s.as_str()).collect::<Vec<_>>();

                buf[3..].write_copy_of_slice(utf_16_repr.as_bytes());
                unsafe { buf.assume_init() }
            }
            Ptg::Int(i) => Box::new(i.to_le_bytes()),
            Ptg::Num(n) => Box::new(n.to_le_bytes()),
            Ptg::Bool(b) => Box::new(if *b { [0x01] } else { [0x00] }),
            Ptg::List(l) => {
                let mut repr = l.to_bytes()?;
                repr.insert(0, 0x19);
                repr.insert(0, 0x18);
                repr.into_boxed_slice()
            }
            // simple field types
            Ptg::Area { r#type, area } => {
                let mut buf = Box::new_zeroed_slice(13);
                buf[0].write(0x05_u8 | ((*r#type as u8) << 5));
                area.to_slice(unsafe { buf[1..].assume_init_mut() })?;
                unsafe { buf.assume_init() }
            }
            Ptg::Area3d { r#type, ixti, area } => {
                let mut buf = Box::new_zeroed_slice(15);
                buf[0].write(0x1B_u8 | ((*r#type as u8) << 5));
                buf[1..3].write_copy_of_slice(&ixti.to_le_bytes());
                area.to_slice(unsafe { buf[3..].assume_init_mut() })?;
                unsafe { buf.assume_init() }
            }
            Ptg::AreaErr { r#type } => {
                let mut buf = Box::new_zeroed_slice(4 + 4 + 4 + 1);
                buf[0].write(0x0B | ((*r#type as u8) << 5));
                unsafe { buf.assume_init() }
            }
            Ptg::AreaErr3d { r#type, ixti } => {
                let mut buf = Box::new_zeroed_slice(4 + 4 + 4 + 1 + 2);
                buf[0].write(0x1D | ((*r#type as u8) << 5));
                buf[1..3].write_copy_of_slice(&ixti.to_le_bytes());
                unsafe { buf.assume_init() }
            }
            Ptg::AreaN { r#type, area } => {
                let mut buf = Box::new_zeroed_slice(12 + 1);
                buf[0].write(0x0D | ((*r#type as u8) << 5));
                area.to_slice(unsafe { buf[1..].assume_init_mut() })?;
                unsafe { buf.assume_init() }
            }
            Ptg::Array { r#type } => {
                let mut buf = Box::new_zeroed_slice(4 + 2 + 4 + 4 + 1);
                buf[0].write((*r#type as u8) << 5);
                unsafe { buf.assume_init() }
            }
            Ptg::AttrBaxcel { bitSemi } => {
                let mut buf = Box::new_zeroed_slice(4);
                buf[0].write(0x19);
                buf[1].write(if *bitSemi { 0b0000_0110 } else { 0b0000_0100 });
                unsafe { buf.assume_init() }
            }

            _ => unimplemented!(),
        };
        w.write_bytes(buf.as_ref())
    }

    #[allow(clippy::uninit_vec)]
    pub(crate) fn read_ptg<R: Read + Seek>(r: &mut Reader<R>) -> Result<Ptg, deku::DekuError> {
        let mut buf = [0u8; 1];
        r.read_bytes(1, &mut buf)?;

        let ptg = match buf[0] {
            0x01 => Ptg::Exp {
                row: u32::from_reader_with_ctx(r, ())?,
            },
            0x03 => Ptg::Add,
            0x04 => Ptg::Sub,
            0x05 => Ptg::Mul,
            0x06 => Ptg::Div,
            0x07 => Ptg::Power,
            0x08 => Ptg::Concat,
            0x09 => Ptg::Lt,
            0x0A => Ptg::Le,
            0x0B => Ptg::Eq,
            0x0C => Ptg::Ge,
            0x0D => Ptg::Gt,
            0x0E => Ptg::Ne,
            0x0F => Ptg::Isect,
            0x10 => Ptg::Union,
            0x11 => Ptg::Range,
            0x12 => Ptg::UPlus,
            0x13 => Ptg::UMinus,
            0x14 => Ptg::Percent,
            0x15 => Ptg::Paren,
            0x16 => Ptg::MissArg,
            0x17 => Ptg::Str({
                let cch = u16::from_reader_with_ctx(r, ())? as usize;
                String::from_utf16le(
                    {
                        let mut buf = Vec::with_capacity(cch * 2);
                        unsafe { buf.set_len(cch * 2) };
                        r.read_bytes(cch * 2, &mut buf)?;
                        buf
                    }
                    .as_slice(),
                )
                .map_err(|_| DekuError::Parse(Cow::Borrowed("Invalid PtgStr")))?
            }),

            0x18 => {
                r.read_bytes(1, &mut buf)?;
                match buf[0] {
                    0x19 => Ptg::List(List::from_reader_with_ctx(r, ())?),
                    0x1d => Ptg::SxName {
                        sxIndex: u32::from_reader_with_ctx(r, ())?,
                    },
                    _ => unreachable!(),
                }
            }
            0x19 => {
                r.read_bytes(1, &mut buf)?;
                match buf[0] {
                    0x01 => Ptg::AttrSemi,
                    0x02 => Ptg::AttrIf {
                        offset: u16::from_reader_with_ctx(r, ())?,
                    },
                    0x04 => {
                        let cOffset = u16::from_reader_with_ctx(r, ())?;

                        let rgOffset = (0..(cOffset + 1))
                            .map(|_| u16::from_reader_with_ctx(r, ()))
                            .collect::<Result<Vec<_>, DekuError>>()?;

                        Ptg::AttrChoose { cOffset, rgOffset }
                    }
                    0x08 => Ptg::AttrGoTo {
                        offset: u16::from_reader_with_ctx(r, ())?,
                    },
                    0x10 => Ptg::AttrSum,
                    0x20 => Ptg::AttrBaxcel { bitSemi: false },
                    0x21 => Ptg::AttrBaxcel { bitSemi: true },
                    0x40 => Ptg::AttrSpace {
                        r#type: AttrSpaceType::from_reader_with_ctx(r, ())?,
                    },
                    0x41 => Ptg::AttrSpaceSemi {
                        r#type: AttrSpaceType::from_reader_with_ctx(r, ())?,
                    },
                    0x80 => Ptg::AttrIfError {
                        offset: u16::from_reader_with_ctx(r, ())?,
                    },
                    _ => unreachable!(),
                }
            }

            0x1C => Ptg::Err(BErr::from_reader_with_ctx(r, ())?),
            0x1D => Ptg::Bool(u8::from_reader_with_ctx(r, ())? == 0x01_u8),
            0x1E => Ptg::Int(u16::from_reader_with_ctx(r, ())?),
            0x1F => Ptg::Num(Xnum::from_reader_with_ctx(r, ())?),

            0x20 | 0x40 | 0x60 => {
                r.skip_bits(4 * 3 * 8 + 2 * 8)?;

                Ptg::Array {
                    r#type: as_ptg_datatype(buf[0], 5),
                }
            }
            0x21 | 0x41 | 0x61 => Ptg::Func {
                iftab: Ftab::from_reader_with_ctx(r, ())?,
            },
            0x22 | 0x42 | 0x62 => {
                let r#type = as_ptg_datatype(buf[0], 5);
                let cparams = u8::from_reader_with_ctx(r, ())?;

                #[derive(DekuRead, DekuWrite)]
                struct RawTab {
                    #[deku(bits = 15)]
                    tab: u16,
                    #[deku(bits = 1)]
                    fCeFunc: bool,
                }

                let tab_raw = RawTab::from_reader_with_ctx(r, ())?;
                let tab = if tab_raw.fCeFunc {
                    Tab::Ce(unsafe { std::mem::transmute::<u16, Cetab>(tab_raw.tab) })
                } else {
                    Tab::F(unsafe { std::mem::transmute::<u16, Ftab>(tab_raw.tab) })
                };

                Ptg::FuncVar {
                    r#type,
                    cparams,
                    tab,
                }
            }
            0x23 | 0x43 | 0x63 => Ptg::Name {
                r#type: as_ptg_datatype(buf[0], 5),
                nameindex: u32::from_reader_with_ctx(r, ())?,
            },
            0x24 | 0x44 | 0x64 => Ptg::Ref {
                r#type: as_ptg_datatype(buf[0], 5),
                loc: RgceLoc::from_reader_with_ctx(r, ())?,
            },
            0x25 | 0x45 | 0x65 => Ptg::Area {
                r#type: as_ptg_datatype(buf[0], 5),
                area: RgceArea::from_reader_with_ctx(r, ())?,
            },
            0x26 | 0x46 | 0x66 => Ptg::MemArea {
                r#type: as_ptg_datatype(buf[0], 5),
                cce: {
                    r.skip_bits(4 * 8)?;
                    u16::from_reader_with_ctx(r, ())?
                },
            },
            0x27 | 0x47 | 0x67 => Ptg::MemErr {
                r#type: as_ptg_datatype(buf[0], 5),
                err: BErr::from_reader_with_ctx(r, ())?,
                cce: {
                    r.skip_bits(3 * 8)?;
                    u16::from_reader_with_ctx(r, ())?
                },
            },
            0x28 | 0x48 | 0x68 => Ptg::MemNoMem {
                r#type: as_ptg_datatype(buf[0], 5),
                cce: {
                    r.skip_bits(4 * 8)?;
                    u16::from_reader_with_ctx(r, ())?
                },
            },
            0x29 | 0x49 | 0x69 => Ptg::MemFunc {
                r#type: as_ptg_datatype(buf[0], 5),
                cce: {
                    r.skip_bits(4 * 8)?;
                    u16::from_reader_with_ctx(r, ())?
                },
            },
            0x2A | 0x4A | 0x6A => {
                r.skip_bits((4 + 2) * 8)?;
                Ptg::RefErr {
                    r#type: as_ptg_datatype(buf[0], 5),
                }
            }
            0x2B | 0x4B | 0x6B => {
                r.skip_bits(4 * 3 * 8)?;

                Ptg::AreaErr {
                    r#type: as_ptg_datatype(buf[0], 5),
                }
            }
            0x2C | 0x4C | 0x6C => Ptg::RefN {
                r#type: as_ptg_datatype(buf[0], 5),
                loc: RgceLocRel::from_reader_with_ctx(r, ())?,
            },
            0x2D | 0x4D | 0x6D => {
                let area = RgceAreaRel::from_reader_with_ctx(r, ())?;

                Ptg::AreaN {
                    r#type: as_ptg_datatype(buf[0], 5),
                    area,
                }
            }

            0x39 | 0x59 | 0x79 => Ptg::NameX {
                r#type: as_ptg_datatype(buf[0], 5),
                ixti: u16::from_reader_with_ctx(r, ())?,
                nameindex: u32::from_reader_with_ctx(r, ())?,
            },
            0x3A | 0x5A | 0x7A => Ptg::Ref3d {
                r#type: as_ptg_datatype(buf[0], 5),
                ixti: u16::from_reader_with_ctx(r, ())?,
                loc: RgceLoc::from_reader_with_ctx(r, ())?,
            },
            0x3B | 0x5B | 0x7B => Ptg::Area3d {
                r#type: as_ptg_datatype(buf[0], 5),
                ixti: u16::from_reader_with_ctx(r, ())?,
                area: RgceArea::from_reader_with_ctx(r, ())?,
            },
            0x3C | 0x5C | 0x7C => {
                let ixti = u16::from_reader_with_ctx(r, ())?;
                r.skip_bits((4 + 2) * 8)?;

                Ptg::RefErr3d {
                    r#type: as_ptg_datatype(buf[0], 5),
                    ixti,
                }
            }
            0x3D | 0x5D | 0x7D => {
                let ixti = u16::from_reader_with_ctx(r, ())?;
                r.skip_bits((4 + 4 + 4) * 8)?;

                Ptg::AreaErr3d {
                    r#type: as_ptg_datatype(buf[0], 5),
                    ixti,
                }
            }

            _ => unreachable!(),
        };
        Ok(ptg)
    }
}

#[test]
fn test_ptg() {
    #[derive(deku::DekuRead, deku::DekuWrite, Debug, PartialEq, PartialOrd, Eq, Ord)]
    struct PtgHolder {
        #[deku(
            reader = "ptg::read_ptg(deku::reader)",
            writer = "ptg::write_ptg(ptg, deku::writer)"
        )]
        ptg: ptg::Ptg,
    }
    use crate::biff::RgceArea;

    macro_rules! test_ptg {
        ($e: expr$(, $o: expr)*) => {{
            let a_1 = PtgHolder { ptg: $e };
            let buf = deku::DekuContainerWrite::to_bytes(&a_1).expect("Cannot serialize");
            let ((_, _), a_2): ((_, _), PtgHolder) =deku::DekuContainerRead::from_bytes((buf.as_ref(), 0)).expect("Cannot deserialize");
            assert!(a_1 == a_2);
            println!("[x] {:?}", a_1);

            $(
                let a_1 = PtgHolder { ptg: $o };
                let buf = deku::DekuContainerWrite::to_bytes(&a_1).expect("Cannot serialize");
                let ((_, _), a_2): ((_, _), PtgHolder) =deku::DekuContainerRead::from_bytes((buf.as_ref(), 0)).expect("Cannot deserialize");
                assert!(a_1 == a_2);
                println!("[x] {:?}", a_1);
            )*
        }};
    }

    test_ptg!(
        ptg::Ptg::Add,
        ptg::Ptg::Area {
            r#type: ptg::DataType::Val,
            area: RgceArea {
                rowFirst: 0,
                rowLast: 1,
                columnFirst: 2,
                columnLast: 3
            }
        },
        ptg::Ptg::Area3d {
            r#type: ptg::DataType::Arr,
            ixti: 0x0045,
            area: RgceArea {
                rowFirst: 0,
                rowLast: 1,
                columnFirst: 2,
                columnLast: 3,
            },
        },
        ptg::Ptg::AreaErr {
            r#type: ptg::DataType::Arr
        }
    );
}
