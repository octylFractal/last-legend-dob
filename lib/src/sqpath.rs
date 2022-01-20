//! Original from https://github.com/CerulanLumina/sqpack, under the Apache 2.0 License.
//! Slightly modified to use a CRC library rather than an in-crate implementation.
//! Also modified to produce a index2 hash instead.
use std::convert::Infallible;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use std::{
    borrow::Borrow,
    ops::Deref,
    path::{Path, PathBuf},
};

/// A representation of a location within the FFXIV data files. This is an
/// **unsized** type, so it must always be behind a reference such as & or Box.
/// Use SqPathBuf for the Owned/Sized/Allocated variant.
#[derive(Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct SqPath {
    inner: str,
}

impl SqPath {
    /// Creates a new borrowed SqPath from a str-like input reference
    ///
    /// # Examples
    /// ```
    /// use last_legend_dob::sqpath::{SqPath, SqPathBuf};
    ///
    /// // from an &str
    /// let a = SqPath::new("testing");
    ///
    /// // from a String
    /// let s = String::from("testing");
    /// let b = SqPath::new(&s);
    ///
    /// assert_eq!(a, b)
    /// ```
    pub fn new<S: AsRef<str> + ?Sized>(s: &S) -> &SqPath {
        // Use of unsafe follows same format as std::path::Path for unsized type
        unsafe { &*(s.as_ref() as *const str as *const SqPath) }
    }

    /// Gets the index hash (v2) of the file. This struct allows you to locate
    /// a specific file within the index, as the index files are all encoded
    /// based on a specific hash of the file path.
    pub fn sq_index_hash(&self) -> u32 {
        const CALCULATOR: crc::Crc<u32> = crc::Crc::<u32>::new(&crc::CRC_32_JAMCRC);
        CALCULATOR.checksum(self.inner.to_ascii_lowercase().as_bytes())
    }

    /// Gets the path to the index file (v2) that locates this SqPath within the .dat files. The
    /// location of the SqPack currently in use is specified by `sqpack`
    ///
    /// # Returns
    /// An Option of an OS `PathBuf` pointing to the index file (v2) if the proper index file could
    /// be parsed, None otherwise.
    pub fn sqpack_index_path<P: AsRef<Path>>(&self, sqpack: P) -> Option<PathBuf> {
        let sqpack = sqpack.as_ref();

        FileType::parse_from_sqpath(self)
            .and_then(|file_type| Expansion::parse_from_sqpath(self).map(|exp| (file_type, exp)))
            .and_then(|(file_type, expansion)| {
                SqPackNumber::parse_from_sqpath(self).map(|spn| (file_type, expansion, spn))
            })
            .map(|(file_type, expansion, sqpack_number)| {
                const SUFFIX: &[u8] = b".win32.index2";
                let ft_bytes = file_type.file_name_prefix_bytes();
                let exp_bytes = expansion.file_name_prefix_bytes();
                let spn_bytes = sqpack_number.file_name_prefix_bytes();
                let mut data = Vec::with_capacity(
                    ft_bytes.len() + exp_bytes.len() + spn_bytes.len() + SUFFIX.len(),
                );
                data.extend_from_slice(&ft_bytes);
                data.extend_from_slice(&exp_bytes);
                data.extend_from_slice(&spn_bytes);
                data.extend_from_slice(SUFFIX);
                sqpack
                    .join(expansion.as_str())
                    .join(String::from_utf8(data).expect("Always valid UTF-8"))
            })
    }

    /// Returns this path as a reference to a string
    pub fn as_str(&self) -> &str {
        &self.inner
    }
}

/// An owned, sized representation of a location within the FFXIV data files.
/// It implements `Deref<SqPath>` so you can call all the same functions as
/// `SqPath`.
#[derive(Ord, PartialOrd, PartialEq, Eq, Debug, Hash, Clone)]
pub struct SqPathBuf {
    inner: String,
}

impl SqPathBuf {
    /// Creates a new owned & allocated SqPathBuf from a str-like input reference
    ///
    /// # Examples
    /// ```
    /// use last_legend_dob::sqpath::{SqPath, SqPathBuf};
    ///
    /// // from an &str
    /// let a = SqPathBuf::new("testing");
    ///
    /// // from a String ref
    /// let s = String::from("testing");
    /// let b = SqPathBuf::new(&s);
    ///
    /// assert_eq!(a, b)
    /// ```
    pub fn new<S: AsRef<str> + ?Sized>(s: &S) -> SqPathBuf {
        SqPathBuf {
            inner: String::from(s.as_ref()),
        }
    }
}

impl Deref for SqPathBuf {
    type Target = SqPath;
    fn deref(&self) -> &SqPath {
        self.as_ref()
    }
}

/// The FileType of a SqPath. Specifically, not the actual file type, but rather
/// the index file it can be found in, which are grouped by broad categories of files.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
#[allow(clippy::upper_case_acronyms)]
pub enum FileType {
    Common,
    BGCommon,
    BG,
    Cut,
    Chara,
    Shader,
    UI,
    Sound,
    VFX,
    UIScript,
    EXD,
    GameScript,
    Music,
    SqpackTest,
    Debug,
}

impl FileType {
    /// Parses the filetype implied by the first segment of `sqpath`
    ///
    /// # Returns
    /// An option containing the variant corresponding to the file type, or `None`
    /// if the file type was unrecognized, or if the path was malformed.
    pub fn parse_from_sqpath<P: AsRef<SqPath>>(sqpath: P) -> Option<FileType> {
        let sqpath = sqpath.as_ref();
        let s = sqpath.as_str();

        let index_opt = s.find('/');
        let slice_opt = index_opt.map(|index| &s[..index]);

        slice_opt.and_then(|type_str| match type_str {
            "common" => Some(FileType::Common),
            "bgcommon" => Some(FileType::BGCommon),
            "bg" => Some(FileType::BG),
            "cut" => Some(FileType::Cut),
            "chara" => Some(FileType::Chara),
            "shader" => Some(FileType::Shader),
            "ui" => Some(FileType::UI),
            "sound" => Some(FileType::Sound),
            "vfx" => Some(FileType::VFX),
            "ui_script" => Some(FileType::UIScript),
            "exd" => Some(FileType::EXD),
            "game_script" => Some(FileType::GameScript),
            "music" => Some(FileType::Music),
            "_sqpack_test" => Some(FileType::SqpackTest),
            "_debug" => Some(FileType::Debug),
            _ => None,
        })
    }

    /// Gets a reference to a static string representing the hex code of the FileType variant.
    /// This hex code is part of what composes a file name in the sqpack, i.e. music .index and .dat
    /// files always begin with `0c`, such as `0c0000.win32.index`.
    pub fn file_name_prefix_bytes(&self) -> [u8; 2] {
        *match self {
            FileType::Common => b"00",
            FileType::BGCommon => b"01",
            FileType::BG => b"02",
            FileType::Cut => b"03",
            FileType::Chara => b"04",
            FileType::Shader => b"05",
            FileType::UI => b"06",
            FileType::Sound => b"07",
            FileType::VFX => b"08",
            FileType::UIScript => b"09",
            FileType::EXD => b"0a",
            FileType::GameScript => b"0b",
            FileType::Music => b"0c",
            FileType::SqpackTest => b"12",
            FileType::Debug => b"13",
        }
    }

    /// Gets a byte representing the hex code of the FileType variant. See `file_name_prefix_str`.
    pub fn file_name_prefix(&self) -> u8 {
        match self {
            FileType::Common => 0x00,
            FileType::BGCommon => 0x01,
            FileType::BG => 0x02,
            FileType::Cut => 0x03,
            FileType::Chara => 0x04,
            FileType::Shader => 0x05,
            FileType::UI => 0x06,
            FileType::Sound => 0x07,
            FileType::VFX => 0x08,
            FileType::UIScript => 0x09,
            FileType::EXD => 0x0a,
            FileType::GameScript => 0x0b,
            FileType::Music => 0x0c,
            FileType::SqpackTest => 0x12,
            FileType::Debug => 0x13,
        }
    }

    /// Returns a static str representation of this variant. Useful in composing SqPaths.
    pub fn as_str(&self) -> &'static str {
        match self {
            FileType::Common => "common",
            FileType::BGCommon => "bgcommon",
            FileType::BG => "bg",
            FileType::Cut => "cut",
            FileType::Chara => "chara",
            FileType::Shader => "shader",
            FileType::UI => "ui",
            FileType::Sound => "sound",
            FileType::VFX => "vfx",
            FileType::UIScript => "ui_script",
            FileType::EXD => "exd",
            FileType::GameScript => "game_script",
            FileType::Music => "music",
            FileType::SqpackTest => "_sqpack_test",
            FileType::Debug => "_debug",
        }
    }
}

/// The game expansion a file can be found in, such as FFXIV (base game), EX1 (Heavensward), etc...
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub enum Expansion {
    FFXIV,
    Heavensward,
    Stormblood,
    Shadowbringers,
    Endwalker,
}

impl Expansion {
    /// Parses the expansion implied by the second segment of `sqpath`
    ///
    /// # Returns
    /// An option containing the variant corresponding to the game expansion, or `None`
    /// if the expansion was unrecognized or the path was malformed.
    pub fn parse_from_sqpath<P: AsRef<SqPath>>(sqpath: P) -> Option<Expansion> {
        let sqpath = sqpath.as_ref();
        let s = sqpath.as_str();

        s.split('/').nth(1).and_then(|exp_str| match exp_str {
            "ffxiv" => Some(Expansion::FFXIV),
            "ex1" => Some(Expansion::Heavensward),
            "ex2" => Some(Expansion::Stormblood),
            "ex3" => Some(Expansion::Shadowbringers),
            "ex4" => Some(Expansion::Endwalker),
            _ => None,
        })
    }

    /// Gets a reference to a static string representing the hex code of the Expansion variant.
    /// This hex code is part of what composes a file name in the sqpack, i.e. music .index and .dat
    /// from Heavensward might be `0c0100.win32.index/dat`.
    pub fn file_name_prefix_bytes(&self) -> [u8; 2] {
        *match self {
            Expansion::FFXIV => b"00",
            Expansion::Heavensward => b"01",
            Expansion::Stormblood => b"02",
            Expansion::Shadowbringers => b"03",
            Expansion::Endwalker => b"04",
        }
    }

    /// Gets a byte representing the hex code of the Expansion variant. See `file_name_prefix_str`.
    pub fn file_name_prefix(&self) -> u8 {
        match self {
            Expansion::FFXIV => 0x00u8,
            Expansion::Heavensward => 0x01u8,
            Expansion::Stormblood => 0x02u8,
            Expansion::Shadowbringers => 0x03u8,
            Expansion::Endwalker => 0x04u8,
        }
    }

    /// Returns a static str representation of this variant. Useful in composing SqPaths.
    pub fn as_str(&self) -> &'static str {
        match self {
            Expansion::FFXIV => "ffxiv",
            Expansion::Heavensward => "ex1",
            Expansion::Stormblood => "ex2",
            Expansion::Shadowbringers => "ex3",
            Expansion::Endwalker => "ex4",
        }
    }
}

#[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Debug, Hash, Default)]
pub struct SqPackNumber(u8);

impl SqPackNumber {
    /// Parses the numerical index of the specific index/dat file implied by the filename of `sqpath`
    ///
    /// # Returns
    /// An option containing numerical index of the dat/index file, or `None`
    /// if the path was malformed.
    pub fn parse_from_sqpath<P: AsRef<SqPath>>(sqpath: P) -> Option<SqPackNumber> {
        let sqpath = sqpath.as_ref();
        let s = sqpath.as_str();

        s.split('/')
            .nth(2)
            .and_then(|filename_str: &str| filename_str.split('_').next())
            .map(|part: &str| {
                let val = u8::from_str_radix(part, 16).ok().unwrap_or(0);
                SqPackNumber(val)
            })
    }

    /// Returns the prefix for this numerical index as a byte array
    pub fn file_name_prefix_bytes(&self) -> [u8; 2] {
        // very simple byte to hex ascii chars implementation
        let mut data = [0; 2];
        let left = self.0 >> 4;
        let right = self.0 & 0xf;
        data[0] = if left < 10 {
            left + 0x30
        } else {
            left + 0x61 - 10
        };
        data[1] = if right < 10 {
            right + 0x30
        } else {
            right + 0x61 - 10
        };
        data
    }
}

impl AsRef<SqPath> for str {
    fn as_ref(&self) -> &SqPath {
        SqPath::new(self)
    }
}

impl AsRef<SqPath> for &SqPath {
    fn as_ref(&self) -> &SqPath {
        self
    }
}

impl AsRef<SqPath> for String {
    fn as_ref(&self) -> &SqPath {
        SqPath::new(self.as_str())
    }
}

impl AsRef<SqPath> for SqPathBuf {
    fn as_ref(&self) -> &SqPath {
        self.inner.as_ref()
    }
}

impl ToOwned for SqPath {
    type Owned = SqPathBuf;
    fn to_owned(&self) -> Self::Owned {
        SqPathBuf::new(&self.inner)
    }
}

impl Borrow<SqPath> for SqPathBuf {
    fn borrow(&self) -> &SqPath {
        SqPath::new(&self.inner)
    }
}

impl FromStr for SqPathBuf {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(SqPathBuf::new(s))
    }
}

impl Display for SqPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl Display for SqPathBuf {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

#[cfg(test)]
mod sqpath_tests {
    use std::borrow::Borrow;

    use crate::sqpath::{Expansion, FileType, SqPackNumber, SqPath, SqPathBuf};

    #[test]
    fn basic_sqpath() {
        let iref = &SqPath::new("asdasd").inner;
        assert_eq!(iref, "asdasd");
    }

    #[test]
    fn sqpath_as_refs() {
        let a: &SqPath = "uwu".as_ref();
        let s = String::from("uwu");
        let b: &SqPath = s.as_ref();
        let sqpb = SqPathBuf::new("uwu");
        let c: &SqPath = sqpb.as_ref();
        assert_eq!(&a.inner, "uwu");
        assert_eq!(&b.inner, "uwu");
        assert_eq!(&c.inner, "uwu");
    }

    #[test]
    fn basic_sqpathbuf() {
        let sqpb = SqPathBuf::new("uwu");
        assert_eq!(sqpb.inner, "uwu");
    }

    #[test]
    fn new_params_any() {
        SqPathBuf::new("uwu");
        let s = String::from("uwu");
        SqPathBuf::new(&s);
    }

    #[test]
    fn sq_index_path() {
        let sq_path = SqPath::new("music/ffxiv");
        let sq_index_path = sq_path.sq_index_hash();
        assert_eq!(sq_index_path, 0x0AF269D6);

        let sq_path = SqPath::new("BGM_System_Title.scd");
        let sq_index_path = sq_path.sq_index_hash();
        assert_eq!(sq_index_path, 0xE3B71579);

        let sq_pathbuf: SqPathBuf = sq_path.to_owned();
        let sq_index_path = sq_pathbuf.sq_index_hash();
        assert_eq!(sq_index_path, 0xE3B71579);
    }

    #[test]
    fn to_owned_and_borrow() {
        let sqpath = SqPath::new("uwu");
        let a: SqPathBuf = sqpath.to_owned();
        assert_eq!(a.inner, sqpath.inner);

        let x: &SqPath = a.borrow();
        assert_eq!(x.inner, a.inner);
    }

    #[test]
    fn file_type_parse() {
        let sqpath = SqPath::new("music/ffxiv/BGM_System_Title.scd");
        let ftype = FileType::parse_from_sqpath(sqpath);
        assert!(ftype.is_some());
        assert_eq!(ftype.unwrap(), FileType::Music);

        let sqpath = SqPath::new("exd/ffxiv/root.exl");
        let ftype = FileType::parse_from_sqpath(sqpath);
        assert!(ftype.is_some());
        assert_eq!(ftype.unwrap(), FileType::EXD);
    }

    #[test]
    fn file_type_index_fragment() {
        assert_eq!(FileType::Common.file_name_prefix_bytes(), *b"00");
        assert_eq!(FileType::BGCommon.file_name_prefix_bytes(), *b"01");
        assert_eq!(FileType::BG.file_name_prefix_bytes(), *b"02");
        assert_eq!(FileType::Cut.file_name_prefix_bytes(), *b"03");
        assert_eq!(FileType::Chara.file_name_prefix_bytes(), *b"04");
        assert_eq!(FileType::Shader.file_name_prefix_bytes(), *b"05");
        assert_eq!(FileType::UI.file_name_prefix_bytes(), *b"06");
        assert_eq!(FileType::Sound.file_name_prefix_bytes(), *b"07");
        assert_eq!(FileType::VFX.file_name_prefix_bytes(), *b"08");
        assert_eq!(FileType::UIScript.file_name_prefix_bytes(), *b"09");
        assert_eq!(FileType::EXD.file_name_prefix_bytes(), *b"0a");
        assert_eq!(FileType::GameScript.file_name_prefix_bytes(), *b"0b");
        assert_eq!(FileType::Music.file_name_prefix_bytes(), *b"0c");
        assert_eq!(FileType::SqpackTest.file_name_prefix_bytes(), *b"12");
        assert_eq!(FileType::Debug.file_name_prefix_bytes(), *b"13");
    }

    #[test]
    fn file_type_parse_and_as_str_eq() {
        assert_eq!(
            FileType::parse_from_sqpath("common/ffxiv/asdfdfh")
                .unwrap()
                .as_str(),
            "common"
        );
        assert_eq!(
            FileType::parse_from_sqpath("bgcommon/ex1/asdfdfh")
                .unwrap()
                .as_str(),
            "bgcommon"
        );
        assert_eq!(
            FileType::parse_from_sqpath("bg/ex2/asdfdfh")
                .unwrap()
                .as_str(),
            "bg"
        );
        assert_eq!(
            FileType::parse_from_sqpath("cut/ex3/dfsdfg")
                .unwrap()
                .as_str(),
            "cut"
        );
        assert_eq!(
            FileType::parse_from_sqpath("chara/ffxiv/sdfgdfs")
                .unwrap()
                .as_str(),
            "chara"
        );
        assert_eq!(
            FileType::parse_from_sqpath("shader/ffxiv/fdgsdgs")
                .unwrap()
                .as_str(),
            "shader"
        );
        assert_eq!(
            FileType::parse_from_sqpath("ui/ex3/srdsfvr")
                .unwrap()
                .as_str(),
            "ui"
        );
        assert_eq!(
            FileType::parse_from_sqpath("sound/ffxiv/sdfgdfg")
                .unwrap()
                .as_str(),
            "sound"
        );
        assert_eq!(
            FileType::parse_from_sqpath("vfx/ffxiv/sdfdfg")
                .unwrap()
                .as_str(),
            "vfx"
        );
        assert_eq!(
            FileType::parse_from_sqpath("ui_script/ffxiv/sdfsdf")
                .unwrap()
                .as_str(),
            "ui_script"
        );
        assert_eq!(
            FileType::parse_from_sqpath("exd/ffxiv/sdfdsfg")
                .unwrap()
                .as_str(),
            "exd"
        );
        assert_eq!(
            FileType::parse_from_sqpath("game_script/ffxiv/sdfdsfg")
                .unwrap()
                .as_str(),
            "game_script"
        );
        assert_eq!(
            FileType::parse_from_sqpath("music/ffxiv/sdfdsfg")
                .unwrap()
                .as_str(),
            "music"
        );
        assert_eq!(
            FileType::parse_from_sqpath("_sqpack_test/ffxiv/sdfdsfg")
                .unwrap()
                .as_str(),
            "_sqpack_test"
        );
        assert_eq!(
            FileType::parse_from_sqpath("_debug/ffxiv/sdfdsfg")
                .unwrap()
                .as_str(),
            "_debug"
        );
    }

    #[test]
    fn file_type_file_name_prefix() {
        assert_eq!(FileType::Common.file_name_prefix(), 0x00u8);
        assert_eq!(FileType::BGCommon.file_name_prefix(), 0x01u8);
        assert_eq!(FileType::BG.file_name_prefix(), 0x02u8);
        assert_eq!(FileType::Cut.file_name_prefix(), 0x03u8);
        assert_eq!(FileType::Chara.file_name_prefix(), 0x04u8);
        assert_eq!(FileType::Shader.file_name_prefix(), 0x05u8);
        assert_eq!(FileType::UI.file_name_prefix(), 0x06u8);
        assert_eq!(FileType::Sound.file_name_prefix(), 0x07u8);
        assert_eq!(FileType::VFX.file_name_prefix(), 0x08u8);
        assert_eq!(FileType::UIScript.file_name_prefix(), 0x09u8);
        assert_eq!(FileType::EXD.file_name_prefix(), 0x0au8);
        assert_eq!(FileType::GameScript.file_name_prefix(), 0x0bu8);
        assert_eq!(FileType::Music.file_name_prefix(), 0x0cu8);
        assert_eq!(FileType::SqpackTest.file_name_prefix(), 0x12u8);
        assert_eq!(FileType::Debug.file_name_prefix(), 0x13u8);
    }

    #[test]
    fn expansion_parse() {
        let sqpath = SqPath::new("music/ffxiv/BGM_System_Title.scd");
        let exp = Expansion::parse_from_sqpath(sqpath);
        assert!(exp.is_some());
        assert_eq!(exp.unwrap(), Expansion::FFXIV);

        let sqpath = SqPath::new("music/ex2/dfgdfgsdfg.scd");
        let exp = Expansion::parse_from_sqpath(sqpath);
        assert!(exp.is_some());
        assert_eq!(exp.unwrap(), Expansion::Stormblood);
    }

    #[test]
    fn expansion_index_fragment() {
        assert_eq!(Expansion::FFXIV.file_name_prefix_bytes(), *b"00");
        assert_eq!(Expansion::Heavensward.file_name_prefix_bytes(), *b"01");
        assert_eq!(Expansion::Stormblood.file_name_prefix_bytes(), *b"02");
        assert_eq!(Expansion::Shadowbringers.file_name_prefix_bytes(), *b"03");
    }

    #[test]
    fn expansion_parse_and_as_str_eq() {
        assert_eq!(
            Expansion::parse_from_sqpath("common/ffxiv/dfgsdfg.asd")
                .unwrap()
                .as_str(),
            "ffxiv"
        );
        assert_eq!(
            Expansion::parse_from_sqpath("bgcommon/ex1/asdasd.fgh")
                .unwrap()
                .as_str(),
            "ex1"
        );
        assert_eq!(
            Expansion::parse_from_sqpath("bg/ex2/dfhdfgh.hhjg")
                .unwrap()
                .as_str(),
            "ex2"
        );
        assert_eq!(
            Expansion::parse_from_sqpath("cut/ex3/dfghds.yss")
                .unwrap()
                .as_str(),
            "ex3"
        );
        assert_eq!(
            Expansion::parse_from_sqpath("cut/ex3/165_dfghds.yss")
                .unwrap()
                .as_str(),
            "ex3"
        );
    }

    #[test]
    fn expansion_file_name_prefix() {
        assert_eq!(Expansion::FFXIV.file_name_prefix(), 0x00u8);
        assert_eq!(Expansion::Heavensward.file_name_prefix(), 0x01u8);
        assert_eq!(Expansion::Stormblood.file_name_prefix(), 0x02u8);
        assert_eq!(Expansion::Shadowbringers.file_name_prefix(), 0x03u8);
    }

    #[test]
    fn parse_sqpack_number() {
        assert_eq!(
            SqPackNumber::parse_from_sqpath("common/ffxiv/sdfsfda.adasd")
                .unwrap()
                .0,
            0
        );
        assert_eq!(
            SqPackNumber::parse_from_sqpath("common/ex2/001_sdfsfda.adasd")
                .unwrap()
                .0,
            1
        );
        assert_eq!(
            SqPackNumber::parse_from_sqpath("common/ex2/00b_sdfsfda.adasd")
                .unwrap()
                .0,
            11
        );
    }

    #[test]
    fn sqpack_number_prefix() {
        assert_eq!(
            std::str::from_utf8(&SqPackNumber(0).file_name_prefix_bytes()).unwrap(),
            "00"
        );
        assert_eq!(
            std::str::from_utf8(&SqPackNumber(1).file_name_prefix_bytes()).unwrap(),
            "01"
        );
        assert_eq!(
            std::str::from_utf8(&SqPackNumber(10).file_name_prefix_bytes()).unwrap(),
            "0a"
        );
        assert_eq!(
            std::str::from_utf8(&SqPackNumber(16).file_name_prefix_bytes()).unwrap(),
            "10"
        );
        assert_eq!(
            std::str::from_utf8(&SqPackNumber(255).file_name_prefix_bytes()).unwrap(),
            "ff"
        );
    }

    #[test]
    fn sqpack_index_path() {
        let index = SqPath::new("music/ffxiv/BGM_System_Title.scd")
            .sqpack_index_path("/home/uwu/ffxiv/sqpack/");
        let pb = index.unwrap();
        assert_eq!(
            pb.as_os_str(),
            "/home/uwu/ffxiv/sqpack/ffxiv/0c0000.win32.index2"
        );

        let path = "/home/uwu/ffxiv/sqpack";
        assert_eq!(
            SqPath::new("music/ex3/BGM_EX3_Event_05.scd")
                .sqpack_index_path(path)
                .unwrap()
                .as_os_str(),
            "/home/uwu/ffxiv/sqpack/ex3/0c0300.win32.index2"
        );
        assert_eq!(
            SqPath::new("common/ex2/0fe_uwu.owo")
                .sqpack_index_path(path)
                .unwrap()
                .as_os_str(),
            "/home/uwu/ffxiv/sqpack/ex2/0002fe.win32.index2"
        );
    }
}
