macro_rules! define_tests_for {
    ($ext:ident) => {
        #[cfg(test)]
        mod $ext {
            use std::str::FromStr;

            use libmuso::format::ParsedFormat;
            use libmuso::metadata::Metadata;
            use libmuso::{Error, Result};

            #[test]
            fn complete_with_ok_format() -> AnyResult<()> {
                let ext = stringify!($ext);
                let metadata = Metadata::from_path(format!("test_files/complete.{}", ext))?;
                let format =
                    ParsedFormat::from_str("{artist}/{album}/{disc}.{track} - {title}.{ext}")?;

                assert_eq! {
                    Ok(format!("Album Artist/Album/1.1 - Title.{}", ext)),
                    format.build_path(&metadata, false)
                };

                Ok(())
            }

            #[test]
            fn partial_with_ok_format() -> AnyResult<()> {
                let ext = stringify!($ext);
                let metadata = Metadata::from_path(format!("test_files/partial.{}", ext))?;
                let format = ParsedFormat::from_str("{artist}/{disc}.{track} - {title}.{ext}")?;

                assert_eq! {
                    Ok(format!("Artist/1.1 - Title.{}", ext)),
                    format.build_path(&metadata, false)
                };

                Ok(())
            }

            #[test]
            fn both_with_ok_optional_format() -> AnyResult<()> {
                let ext = stringify!($ext);
                let metadata = Metadata::from_path(format!("test_files/partial.{}", ext))?;
                let format = ParsedFormat::from_str("{artist}/{album?} - {title}.{ext}")?;

                assert_eq! {
                    Ok(format!("Artist/ - Title.{}", ext)),
                    format.build_path(&metadata, false)
                };

                let metadata = Metadata::from_path(format!("test_files/complete.{}", ext))?;

                assert_eq!(
                    Ok(format!("Album Artist/Album - Title.{}", ext)),
                    format.build_path(&metadata, false)
                );

                Ok(())
            }

            #[test]
            fn bad_optional_formats() -> AnyResult<()> {
                let ext = stringify!($ext);
                let metadata = Metadata::from_path(format!("test_files/partial.{}", ext))?;
                let format = ParsedFormat::from_str("{artist}/{album?}/{title}.{ext}")?;

                assert_eq! {
                    Err(MusoError::OptionalInDir),
                    format.build_path(&metadata, false)
                };

                let format = ParsedFormat::from_str("{artist}/{title?}.{ext}")?;

                assert_eq! {
                    Err(MusoError::RequiredInFile),
                    format.build_path(&metadata, false)
                }

                Ok(())
            }

            #[test]
            fn bad_file_format() -> AnyResult<()> {
                let ext = stringify!($ext);
                let metadata = Metadata::from_path(format!("test_files/partial.{}", ext))?;
                let format = ParsedFormat::from_str("{artist}/foo.{ext}")?;

                assert_eq! {
                    Err(MusoError::RequiredInFile),
                    format.build_path(&metadata, false)
                };

                Ok(())
            }

            #[test]
            fn partial_with_bad_format() -> AnyResult<()> {
                let ext = stringify!($ext);
                let metadata = Metadata::from_path(format!("test_files/partial.{}", ext))?;
                let format = ParsedFormat::from_str("{artist}/{album}")?;

                assert_eq! {
                    Err(MusoError::MissingTag{ tag: "album".into() }),
                    format.build_path(&metadata, false)
                };

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
