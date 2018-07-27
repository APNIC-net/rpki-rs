//! rsync URIs.

use std::{fmt, str};
use bytes::{BufMut, Bytes, BytesMut};


//------------ Uri -----------------------------------------------------------

/// An rsync URI.
///
/// This implements a simplified form of the the rsync URI defined in RFC 5781
/// which in turn references RFC 3986. Only absolute URIs including an
/// authority are allowed.
///
/// Parsing is simplified in that it only checks for the correct structure and
/// that no forbidden characters are present.
///
//  In particular, forbidden characters are
//
//     SPACE CONTROL " # < > ? [ \\ ] ^ ` { | }
//
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Uri {
    module: Module,
    path: Bytes
}

impl Uri {
    pub fn new(module: Module, path: Bytes) -> Self {
        Uri { module, path }
    }

    pub fn from_slice(slice: &[u8]) -> Result<Self, UriError> {
        Self::from_bytes(slice.into())
    }

    pub fn from_bytes(mut bytes: Bytes) -> Result<Self, UriError> {
        if !is_uri_ascii(&bytes) {
            return Err(UriError::NotAscii)
        }
        if !bytes.starts_with(b"rsync://") {
            return Err(UriError::BadScheme)
        }
        bytes.advance(8);
        let (authority, module) = {
            let mut parts = bytes.splitn(3, |ch| *ch == b'/');
            let authority = match parts.next() {
                Some(part) => part.len(),
                None => return Err(UriError::BadUri)
            };
            let module = match parts.next() {
                Some(part) => part.len(),
                None => return Err(UriError::BadUri)
            };
            (authority, module)
        };
        let authority = bytes.split_to(authority);
        bytes.advance(1);
        let module = bytes.split_to(module);
        bytes.advance(1);
        Ok(Uri {
            module: Module::new(authority, module),
            path: bytes
        })
    }

    pub fn module(&self) -> &Module {
        &self.module
    }

    pub fn to_module(&self) -> Module {
        self.module.clone()
    }

    pub fn path(&self) -> &str {
        unsafe { ::std::str::from_utf8_unchecked(self.path.as_ref()) }
    }

    pub fn to_string(&self) -> String {
        format!("{}", self)
    }

    pub fn parent(&self) -> Option<Self> {
        // rsplit always returns at least one element.
        let tail = self.path.rsplit(|ch| *ch == b'/').next().unwrap().len();
        if tail == 0 {
            None
        }
        else {
            let mut res = self.clone();
            if tail == self.path.len() {
                res.path = Bytes::from_static(b"")
            }
            else {
                res.path = self.path.slice(
                    0, self.path.len() - tail - 1
                );
            }
            Some(res)
        }
    }

    pub fn join(&self, path: &[u8]) -> Self {
        assert!(is_uri_ascii(path));
        let mut res = BytesMut::with_capacity(
            self.path.len() + path.len() + 1
        );
        if !self.path.is_empty() {
            res.put_slice(self.path.as_ref());
            if !self.path.ends_with(b"/") {
                res.put_slice(b"/");
            }
        }
        res.put_slice(path);
        Self::new(self.module.clone(), res.freeze())
    }

    pub fn ends_with(&self, extension: &str) -> bool {
        self.path.ends_with(extension.as_bytes())
    }
}

impl fmt::Display for Uri {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.module.fmt(f)?;
        if !self.path.is_empty() {
            write!(f, "{}", self.path())?;
        }
        Ok(())
    }
}


//------------ Module --------------------------------------------------------

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Module {
    authority: Bytes,
    module: Bytes,
}

impl Module {
    pub fn new<A, M>(authority: A, module: M) -> Self
    where A: Into<Bytes>, M: Into<Bytes> {
        let authority = authority.into();
        let module = module.into();
        assert!(is_uri_ascii(authority.as_ref()));
        assert!(is_uri_ascii(module.as_ref()));
        Module { authority, module }
    }

    pub fn to_uri(&self) -> Uri {
        Uri {
            module: self.clone(),
            path: Bytes::from_static(b""),
        }
    }

    pub fn to_string(&self) -> String {
        format!("{}", self)
    }

    pub fn authority(&self) -> &str {
        unsafe { ::std::str::from_utf8_unchecked(self.authority.as_ref()) }
    }

    pub fn module(&self) -> &str {
        unsafe { ::std::str::from_utf8_unchecked(self.module.as_ref()) }
    }
}

impl fmt::Display for Module {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "rsync://{}/{}/", self.authority(), self.module())
    }
}


//------------ Helper Functions ----------------------------------------------

pub fn is_uri_ascii<S: AsRef<[u8]>>(slice: S) -> bool {
    slice.as_ref().iter().all(|&ch| {
        ch > b' ' && ch != b'"' && ch != b'#' && ch != b'<' && ch != b'>'
            && ch != b'?' && ch != b'[' && ch != b'\\' && ch != b']'
            && ch != b'^' && ch != b'`' && ch != b'{' && ch != b'|'
            && ch != b'}' && ch < 0x7F
    })
}


//------------ UriError ------------------------------------------------------

#[derive(Clone, Debug, Fail)]
pub enum UriError {
    #[fail(display="invalid characters")]
    NotAscii,

    #[fail(display="bad URI")]
    BadUri,

    #[fail(display="bad URI scheme")]
    BadScheme,
}


