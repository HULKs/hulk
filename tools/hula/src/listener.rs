use std::{
    fs::remove_file,
    io,
    ops::{Deref, DerefMut},
    os::unix::net,
};

pub struct HulaListener {
    listener: net::UnixListener,
    path: &'static str,
}

impl HulaListener {
    pub fn bind(path: &'static str) -> io::Result<HulaListener> {
        Ok(Self {
            listener: net::UnixListener::bind(path)?,
            path,
        })
    }
}

impl Deref for HulaListener {
    type Target = net::UnixListener;

    fn deref(&self) -> &Self::Target {
        &self.listener
    }
}

impl DerefMut for HulaListener {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.listener
    }
}

impl Drop for HulaListener {
    fn drop(&mut self) {
        remove_file(self.path).expect("failed to remove listening unix socket");
    }
}
