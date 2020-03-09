extern crate serde;
#[allow(unused_imports)]
#[macro_use]
extern crate serde_derive;
extern crate rmp_serde;
extern crate serde_json;

use std::convert::TryFrom;
// use std::convert::TryInto;

#[derive(Serialize, Deserialize)]
/// @TODO this is hacky
/// i filed an upstream issue
/// https://github.com/3Hren/msgpack-rust/issues/244
pub enum Result {
    Ok(SerializedBytes),
    Err(SerializedBytes),
}

#[derive(Debug)]
pub enum SerializedBytesError {
    /// somehow failed to move to bytes
    /// most likely hit a messagepack limit https://github.com/msgpack/msgpack/blob/master/spec.md#limitation
    ToBytes(String),
    /// somehow failed to restore bytes
    /// i mean, this could be anything, how do i know what's wrong with your bytes?
    FromBytes(String),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct SerializedBytes(Vec<u8>);

impl std::fmt::Debug for SerializedBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut deserializer = rmp_serde::Deserializer::from_read_ref(&self.0);
        let writer = Vec::new();
        let mut serializer = serde_json::ser::Serializer::new(writer);
        serde_transcode::transcode(&mut deserializer, &mut serializer).unwrap();
        let s = unsafe { String::from_utf8_unchecked(serializer.into_inner()) };
        write!(f, "{}", s)
    }
}

#[macro_export]
macro_rules! holochain_serial {
    ( $( $t:ty ),* ) => {

        $(
            impl std::convert::TryFrom<$t> for $crate::SerializedBytes {
                type Error = $crate::SerializedBytesError;
                fn try_from(t: $t) -> std::result::Result<$crate::SerializedBytes, $crate::SerializedBytesError> {
                    match $crate::rmp_serde::to_vec_named(&t) {
                        Ok(v) => Ok($crate::SerializedBytes(v)),
                        Err(e) => Err($crate::SerializedBytesError::ToBytes(e.to_string())),
                    }
                }
            }

            impl std::convert::TryFrom<Option<$t>> for $crate::SerializedBytes {
                type Error = $crate::SerializedBytesError;
                fn try_from(r: Option<$t>) -> std::result::Result<$crate::SerializedBytes, $crate::SerializedBytesError> {
                    match $crate::rmp_serde::to_vec_named(&r) {
                        Ok(v) => Ok($crate::SerializedBytes(v)),
                        Err(e) => Err($crate::SerializedBytesError::ToBytes(e.to_string())),
                    }
                }
            }

            impl std::convert::TryFrom<$crate::SerializedBytes> for $t {
                type Error = $crate::SerializedBytesError;
                fn try_from(sb: $crate::SerializedBytes) -> std::result::Result<$t, $crate::SerializedBytesError> {
                    match $crate::rmp_serde::from_read_ref(&sb.0) {
                        Ok(v) => Ok(v),
                        Err(e) => Err($crate::SerializedBytesError::FromBytes(e.to_string())),
                    }
                }
            }

        )*

    };
}

holochain_serial!(crate::Result);

// impl<S: $crate::serde::Serialize> std::convert::TryFrom<Result<$t, S>> for $crate::SerializedBytes {
//     type Error = $crate::SerializedBytesError;
//     fn try_from(r: std::result::Result<$t, S>) -> std::result::Result<$crate::SerializedBytes, $crate::SerializedBytesError> {
//         match $crate::rmp_serde::to_vec_named(&r) {
//             Ok(v) => Ok($crate::SerializedBytes(v)),
//             Err(e) => Err($crate::SerializedBytesError::ToBytes(e.to_string())),
//         }
//     }
// }

impl<T: serde::ser::Serialize> std::convert::TryFrom<SerializedBytes> for Option<T> where SerializedBytes: TryFrom<T> {
    type Error = SerializedBytesError;
    fn try_from(sb: SerializedBytes) -> std::result::Result<Self, Self::Error> {
        match crate::rmp_serde::from_read_ref(&sb.0) {
            Ok(v) => Ok(v),
            Err(e) => Err(SerializedBytesError::FromBytes(e.to_string())),
        }
    }
}

impl<T: serde::ser::Serialize, E: serde::ser::Serialize> TryFrom<std::result::Result<T, E>>
    for SerializedBytes
where
    SerializedBytes: TryFrom<T>,
    SerializedBytes: TryFrom<E>,
{
    type Error = SerializedBytesError;
    fn try_from(
        r: std::result::Result<T, E>,
    ) -> std::result::Result<SerializedBytes, SerializedBytesError> {
        let enum_result = match r {
            Ok(v) => Result::Ok(
                SerializedBytes::try_from(v)
                    .map_err(|_| SerializedBytesError::ToBytes("z".into()))?,
            ),
            Err(e) => Result::Err(
                SerializedBytes::try_from(e)
                    .map_err(|_| SerializedBytesError::ToBytes("y".into()))?,
            ),
        };
        // match crate::rmp_serde::to_vec_named(&enum_result) {
        //     Ok(v) => Ok(SerializedBytes(v)),
        //     Err(e) => Err(SerializedBytesError::ToBytes(e.to_string())),
        // }
    }
}

impl<T: TryFrom<SerializedBytes>, E: TryFrom<SerializedBytes>> TryFrom<SerializedBytes>
    for std::result::Result<T, E>
{
    type Error = SerializedBytesError;
    fn try_from(
        sb: SerializedBytes,
    ) -> std::result::Result<std::result::Result<T, E>, SerializedBytesError> {
        println!("zz {:?}", &sb);
        match crate::rmp_serde::from_read_ref(&sb.0) {
            Ok(crate::Result::Ok(v)) => {
                Ok(Ok(T::try_from(v).map_err(|_e| {
                    SerializedBytesError::FromBytes("foo".into())
                })?))
            }
            Ok(crate::Result::Err(e)) => {
                Ok(Err(E::try_from(e).map_err(|_e| {
                    SerializedBytesError::FromBytes("bar".into())
                })?))
            }
            Err(e) => Err(SerializedBytesError::FromBytes(e.to_string())),
        }
    }
}

#[cfg(test)]
pub mod tests {

    use super::SerializedBytes;
    use std::convert::TryInto;

    /// struct with a utf8 string in it
    #[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
    struct Foo {
        inner: String,
    }

    /// struct with raw bytes in it
    #[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
    struct Bar {
        whatever: Vec<u8>,
    }

    holochain_serial!(Foo, Bar);

    fn fixture_foo() -> Foo {
        Foo {
            inner: "foo".into(),
        }
    }

    fn fixture_bar() -> Bar {
        Bar {
            whatever: vec![1_u8, 2_u8, 3_u8],
        }
    }

    #[test]
    fn round_trip() {
        macro_rules! do_test {
            ( $t:ty, $i:expr, $o:expr ) => {{
                let i = $i;
                let sb: SerializedBytes = i.clone().try_into().unwrap();
                // this isn't for testing it just shows how the debug output looks
                println!("{:?}", &sb);

                assert_eq!(&$o, &sb.0,);

                let returned: $t = sb.try_into().unwrap();

                assert_eq!(returned, i);
            }};
        }

        do_test!(
            Foo,
            fixture_foo(),
            vec![
                129_u8, 165_u8, 105_u8, 110_u8, 110_u8, 101_u8, 114_u8, 163_u8, 102_u8, 111_u8,
                111_u8,
            ]
        );

        do_test!(
            Bar,
            fixture_bar(),
            vec![
                129_u8, 168_u8, 119_u8, 104_u8, 97_u8, 116_u8, 101_u8, 118_u8, 101_u8, 114_u8,
                147_u8, 1_u8, 2_u8, 3_u8,
            ]
        );

        do_test!(
            Result<Foo, Bar>,
            Ok(fixture_foo()),
            vec![129, 0, 129, 165, 105, 110, 110, 101, 114, 163, 102, 111, 111]
        );

        do_test!(
            Result<Foo, Bar>,
            Err(fixture_bar()),
            vec![129, 1, 129, 168, 119, 104, 97, 116, 101, 118, 101, 114, 147, 1, 2, 3]
        );

        do_test!(
            Result<Bar, Foo>,
            Ok(fixture_bar()),
            vec![129, 0, 129, 168, 119, 104, 97, 116, 101, 118, 101, 114, 147, 1, 2, 3]
        );

        do_test!(
            Result<Bar, Foo>,
            Err(fixture_foo()),
            vec![129, 1, 129, 165, 105, 110, 110, 101, 114, 163, 102, 111, 111]
        );

        // do_test!(
        //     Result<Bar, Result<Foo, Bar>>,
        //     Ok(Ok(fixture_foo())),
        //     vec![129, 1, 129, 165, 105, 110, 110, 101, 114, 163, 102, 111, 111]
        // );

        do_test!(
            Option<Foo>,
            Some(fixture_foo()),
            vec![129, 165, 105, 110, 110, 101, 114, 163, 102, 111, 111]
        );

        do_test!(Option<Foo>, None, vec![192]);
    }
}
