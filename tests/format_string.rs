macro_rules! define_tests_for {
    ($ext:ident) => {
        #[cfg(test)]
        mod $ext {
            use std::path::PathBuf;
            use std::str::FromStr;

            use muso::format::ParsedFormat;
            use muso::metadata::Metadata;
            use muso::{Error, Result};

            #[test]
            fn complete_with_ok_format() -> Result<()> {
                let ext = stringify!($ext);
                let metadata = Metadata::from_path(format!("test_files/complete.{}", ext))?;

                let format = "{artist}/{album}/{disc}.{track} - {title}.{ext}";
                let format = ParsedFormat::from_str(format)?;

                let expected = format!("Album Artist/Album/1.1 - Title.{}", ext);
                let expected = PathBuf::from(expected);

                assert_eq!(expected, format.build_path(&metadata, false)?);

                Ok(())
            }

            #[test]
            fn partial_with_ok_format() -> Result<()> {
                let ext = stringify!($ext);
                let metadata = Metadata::from_path(format!("test_files/partial.{}", ext))?;

                let format = "{artist}/{disc}.{track} - {title}.{ext}";
                let format = ParsedFormat::from_str(format)?;

                let expected = format!("Artist/1.1 - Title.{}", ext);
                let expected = PathBuf::from(expected);

                assert_eq!(expected, format.build_path(&metadata, false)?);

                Ok(())
            }

            #[test]
            fn both_with_ok_optional_format() -> Result<()> {
                let ext = stringify!($ext);
                let metadata = Metadata::from_path(format!("test_files/partial.{}", ext))?;

                let format = "{artist}/{album?} - {title}.{ext}";
                let format = ParsedFormat::from_str(format)?;

                let expected = format!("Artist/ - Title.{}", ext);
                let expected = PathBuf::from(expected);

                assert_eq!(expected, format.build_path(&metadata, false)?);

                let metadata = Metadata::from_path(format!("test_files/complete.{}", ext))?;

                let expected = format!("Album Artist/Album - Title.{}", ext);
                let expected = PathBuf::from(expected);

                assert_eq!(expected, format.build_path(&metadata, false)?);

                Ok(())
            }

            #[test]
            fn bad_optional_formats() -> Result<()> {
                let ext = stringify!($ext);
                let metadata = Metadata::from_path(format!("test_files/partial.{}", ext))?;

                let format = "{artist}/{album?}/{title}.{ext}";
                let format = ParsedFormat::from_str(format)?;

                assert!(matches!(
                    format.build_path(&metadata, false),
                    Err(Error::OptionalInDir)
                ));

                let format = "{artist}/{title?}.{ext}";
                let format = ParsedFormat::from_str(format)?;

                assert!(matches!(
                    format.build_path(&metadata, false),
                    Err(Error::RequiredInFile)
                ));

                Ok(())
            }

            #[test]
            fn bad_file_format() -> Result<()> {
                let ext = stringify!($ext);
                let metadata = Metadata::from_path(format!("test_files/partial.{}", ext))?;

                let format = "{artist}/foo.{ext}";
                let format = ParsedFormat::from_str(format)?;

                assert!(matches!(
                    format.build_path(&metadata, false),
                    Err(Error::RequiredInFile)
                ));

                Ok(())
            }

            #[test]
            fn partial_with_bad_format() -> Result<()> {
                let ext = stringify!($ext);
                let metadata = Metadata::from_path(format!("test_files/partial.{}", ext))?;

                let format = "{artist}/{album}";
                let format = ParsedFormat::from_str(format)?;

                assert!(matches!(
                    format.build_path(&metadata, false),
                    Err(Error::MissingTag { .. })
                ));

                Ok(())
            }
        }
    };
}

define_tests_for!(flac);
define_tests_for!(mp3);
define_tests_for!(ogg);
define_tests_for!(m4a);
define_tests_for!(m4p);
