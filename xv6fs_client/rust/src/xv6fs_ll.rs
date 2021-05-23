/*
* SPDX-License-Identifier: GPL-2.0 OR MIT
*
* Copyright (C) 2020 Samantha Miller, Kaiyuan Zhang, Danyang Zhuo, Tom
     Anderson, Ang Chen, University of Washington
* Copyright (C) 2006-2018 Frans Kaashoek, Robert Morris, Russ Cox,
*                      Massachusetts Institute of Technology
*/

#[cfg(not(feature = "user"))]
use crate::bento_utils;
#[cfg(not(feature = "user"))]
use crate::fuse;
#[cfg(not(feature = "user"))]
use crate::libc;
#[cfg(not(feature = "user"))]
use crate::std;
#[cfg(not(feature = "user"))]
use crate::time;

use alloc::vec::Vec;

use core::str;

use bento_utils::*;
use bento_utils::consts::*;

use fuse::*;

use std::ffi::OsStr;
use std::path::Path;

use time::*;

use std::net::*;
use std::io::{Read,Write};
use std::sync::Mutex;

use bento::println;

const PRIMARY_PORT: u16 = 1234;

static mut SOCKET: Option<Mutex<TcpStream>> = None;

pub struct Xv6FileSystem {
}

impl Xv6FileSystem {
    const NAME: &'static str = "xv6fs_client\0";
}

// messages are in the form of "operation_request local_node_address additional_args"
impl BentoFilesystem<'_> for Xv6FileSystem {

    // return name of file system.
    fn get_name(&self) -> &'static str {
        Self::NAME
    }

    fn bento_init(
        &mut self,
        _req: &Request,
        _devname: &OsStr,
        fc_info: &mut FuseConnInfo,
    ) -> Result<(), i32> {
        println!("bento_init");
        fc_info.proto_major = BENTO_KERNEL_VERSION;
        fc_info.proto_minor = BENTO_KERNEL_MINOR_VERSION;
        
        let mut max_readahead = u32::MAX;
        if fc_info.max_readahead < max_readahead {
            max_readahead = fc_info.max_readahead;
        }

        fc_info.max_readahead = max_readahead;
        fc_info.max_background = 0;
        fc_info.congestion_threshold = 0;
        fc_info.time_gran = 1;

        fc_info.want |= FUSE_BIG_WRITES;
        fc_info.want |= FUSE_ATOMIC_O_TRUNC;
        fc_info.want |= FUSE_WRITEBACK_CACHE;

        println!("creating addr");
        // set up socket
        let srv_addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, PRIMARY_PORT);
        println!("connecting ");
        let stream = match TcpStream::connect(SocketAddr::V4(srv_addr)) {
            Ok(x) => x,
            Err(_) => {

                println!("failed connection");
                return Err(-1);
            },
        };
        println!("writing to server");

        unsafe {
            SOCKET = Some(Mutex::new(stream));
        }



        println!("all successful");
        return Ok(());
    }




    fn bento_destroy(&mut self, _req: &Request) {
        unsafe {
            let msg = format!("exit");
            let msg_bytes = msg.as_bytes();
            let mut socket_guard = SOCKET.as_ref().unwrap().lock().unwrap();
            let _write_res = match socket_guard.write(msg_bytes) {
                Ok(x) => x,
                Err(_) => return,
            };
            let _ = socket_guard.shutdown(Shutdown::Both);
        }
    }

    fn bento_statfs(&self, _req: &Request, _ino: u64, reply: ReplyStatfs) {
        unsafe {

            let msg = format!("statfs");

            let mut socket_guard = SOCKET.as_ref().unwrap().lock().unwrap();
            let _ = socket_guard.write(msg.as_bytes());

            let mut msg_resp = [0 as u8; 4096];
            let size = match socket_guard.read(&mut msg_resp) {
                Ok(x) => x,
                Err(_) => {
                    reply.error(libc::EIO);
                    return;
                }
            };
            let statfs_msg = str::from_utf8(&msg_resp[0..size]).unwrap();
            let statfs_vec: Vec<&str> = statfs_msg.split(' ').collect();
            match *statfs_vec.get(0).unwrap() {
                "Ok" => {
                    if statfs_vec.len() < 9 {
                        reply.error(libc::EINVAL);
                    } else {
                        let blocks: u64 = statfs_vec.get(1).unwrap().parse().unwrap();
                        let bfree: u64 = statfs_vec.get(2).unwrap().parse().unwrap();
                        let bavail: u64 = statfs_vec.get(3).unwrap().parse().unwrap();
                        let files: u64 = statfs_vec.get(4).unwrap().parse().unwrap();
                        let ffree: u64 = statfs_vec.get(5).unwrap().parse().unwrap();
                        let bsize: u32 = statfs_vec.get(6).unwrap().parse().unwrap();
                        let namelen: u32 = statfs_vec.get(7).unwrap().parse().unwrap();
                        let frsize: u32 = statfs_vec.get(8).unwrap().parse().unwrap();
                        reply.statfs(
                            blocks,
                            bfree,
                            bavail,
                            files,
                            ffree,
                            bsize,
                            namelen,
                            frsize
                        );
                    }
                }
                "Err" => {
                    let err_val: i32 = statfs_vec.get(1).unwrap().parse().unwrap();
                    reply.error(err_val);
                },
                _ => reply.error(libc::EINVAL),
            }
        }
    }

    fn bento_open(
        &self,
        _req: &Request,
        nodeid: u64,
        flags: u32,
        reply: ReplyOpen,
    ) {
        unsafe {
            let msg = format!("open {} {}", nodeid, flags);


            let mut socket_guard = SOCKET.as_ref().unwrap().lock().unwrap();
            let _ = socket_guard.write(msg.as_bytes());

            let mut msg_resp = [0 as u8; 4096];

            let size = match socket_guard.read(&mut msg_resp) {
                Ok(x) => {
                    
                    x
                },
                Err(_) => {

                    reply.error(libc::EIO);
                    return;
                }
            };

            let open_msg = str::from_utf8(&msg_resp[0..size]).unwrap();
            let open_vec: Vec<&str> = open_msg.split(' ').collect();

            match *open_vec.get(0).unwrap() {
                "Ok" => {
                    if open_vec.len() < 3 {
                        reply.error(libc::EINVAL);
                    } else {

                        let fh: u64 = open_vec.get(1).unwrap().parse().unwrap();
                        let flags: u32 = open_vec.get(2).unwrap().parse().unwrap();
                        reply.opened(fh, flags);
                    }
                }
                "Err" => {
                    let err_val: i32 = open_vec.get(1).unwrap().parse().unwrap();
                    reply.error(err_val);
                },
                _ => reply.error(libc::EINVAL),
            }
        }
    }

    fn bento_opendir(
        &self,
        _req: &Request,
        nodeid: u64,
        _flags: u32,
        reply: ReplyOpen,
    ) {
        unsafe {
            let msg = format!("opendir {}", nodeid);
            let mut socket_guard = SOCKET.as_ref().unwrap().lock().unwrap();

            let _ = socket_guard.write(msg.as_bytes());

            let mut msg_resp = [0 as u8; 4096];
            let size = match socket_guard.read(&mut msg_resp) {
                Ok(x) => x,
                Err(_) => {
                    reply.error(libc::EIO);
                    return;
                }
            };
            let open_msg = str::from_utf8(&msg_resp[0..size]).unwrap();
            let open_vec: Vec<&str> = open_msg.split(' ').collect();
            match *open_vec.get(0).unwrap() {
                "Ok" => {
                    if open_vec.len() < 3 {
                        reply.error(libc::EINVAL);
                    } else {
                        let fh: u64 = open_vec.get(1).unwrap().parse().unwrap();
                        let flags: u32 = open_vec.get(2).unwrap().parse().unwrap();
                        reply.opened(fh, flags);
                    }
                }
                "Err" => {
                    let err_val: i32 = open_vec.get(1).unwrap().parse().unwrap();
                    reply.error(err_val);
                },
                _ => reply.error(libc::EINVAL),
            }
        }
    }

    fn bento_getattr(&self, _req: &Request, nodeid: u64, reply: ReplyAttr) {
        unsafe {
            let msg = format!("getattr {}", nodeid);
            let mut socket_guard = SOCKET.as_ref().unwrap().lock().unwrap();
            let _ = socket_guard.write(msg.as_bytes());

            let mut msg_resp = [0 as u8; 4096];
            let size = match socket_guard.read(&mut msg_resp) {
                Ok(x) => x,
                Err(_) => {
                    reply.error(libc::EIO);
                    return;
                }
            };
            let attr_msg = str::from_utf8(&msg_resp[0..size]).unwrap();
            let attr_vec: Vec<&str> = attr_msg.split(' ').collect();
            match *attr_vec.get(0).unwrap() {
                "Ok" => {
                    if attr_vec.len() < 21 {
                        reply.error(libc::EINVAL);
                    } else {
                        let ts_sec: i64 = attr_vec.get(1).unwrap().parse().unwrap();
                        let ts_nsec: i32 = attr_vec.get(2).unwrap().parse().unwrap();
                        let attr_valid = Timespec::new(ts_sec, ts_nsec);

                        let ino: u64 = attr_vec.get(3).unwrap().parse().unwrap();
                        let size: u64 = attr_vec.get(4).unwrap().parse().unwrap();
                        let blocks: u64 = attr_vec.get(5).unwrap().parse().unwrap();

                        let atime_sec: i64 = attr_vec.get(6).unwrap().parse().unwrap();
                        let atime_nsec: i32 = attr_vec.get(7).unwrap().parse().unwrap();

                        let mtime_sec: i64 = attr_vec.get(8).unwrap().parse().unwrap();
                        let mtime_nsec: i32 = attr_vec.get(9).unwrap().parse().unwrap();

                        let ctime_sec: i64 = attr_vec.get(10).unwrap().parse().unwrap();
                        let ctime_nsec: i32 = attr_vec.get(11).unwrap().parse().unwrap();

                        let crtime_sec: i64 = attr_vec.get(12).unwrap().parse().unwrap();
                        let crtime_nsec: i32 = attr_vec.get(13).unwrap().parse().unwrap();

                        let kind: FileType = match attr_vec.get(14).unwrap().parse().unwrap() {
                            1 => FileType::Directory,
                            _ => FileType::RegularFile,
                        };

                        let perm: u16 = attr_vec.get(15).unwrap().parse().unwrap();
                        let nlink: u32 = attr_vec.get(16).unwrap().parse().unwrap();
                        let uid: u32 = attr_vec.get(17).unwrap().parse().unwrap();
                        let gid: u32 = attr_vec.get(18).unwrap().parse().unwrap();
                        let rdev: u32 = attr_vec.get(19).unwrap().parse().unwrap();
                        let flags: u32 = attr_vec.get(20).unwrap().parse().unwrap();
                        let attr = FileAttr {
                            ino: ino,
                            size: size,
                            blocks: blocks,
                            atime: Timespec::new(atime_sec, atime_nsec),
                            mtime: Timespec::new(mtime_sec, mtime_nsec),
                            ctime: Timespec::new(ctime_sec, ctime_nsec),
                            crtime: Timespec::new(crtime_sec, crtime_nsec),
                            kind: kind,
                            perm: perm,
                            nlink: nlink,
                            uid: uid,
                            gid: gid,
                            rdev: rdev,
                            flags: flags,
                        };
                        reply.attr(&attr_valid, &attr);
                    }
                }
                "Err" => {
                    let err_val: i32 = attr_vec.get(1).unwrap().parse().unwrap();
                    reply.error(err_val);
                },
                _ => reply.error(libc::EINVAL),
            }
        }
    }

    fn bento_setattr(
        &self,
        _req: &Request,
        ino: u64,
        _mode: Option<u32>,
        _uid: Option<u32>,
        _gid: Option<u32>,
        size: Option<u64>,
        _atime: Option<Timespec>,
        _mtime: Option<Timespec>,
        _fh: Option<u64>,
        _crtime: Option<Timespec>,
        _chgtime: Option<Timespec>,
        _bkuptime: Option<Timespec>,
        _flags: Option<u32>,
        reply: ReplyAttr,
    ) {
        unsafe {
            let msg = match size {
                Some(fsize) => format!("setattr {} {}", ino, fsize),
                None => format!("setattr {} None", ino),
            };
            let mut socket_guard = SOCKET.as_ref().unwrap().lock().unwrap();
            let _ = socket_guard.write(msg.as_bytes());

            let mut msg_resp = [0 as u8; 4096];
            let size = match socket_guard.read(&mut msg_resp) {
                Ok(x) => x,
                Err(_) => {

                    reply.error(libc::EIO);
                    return;
                }
            };
            let attr_msg = str::from_utf8(&msg_resp[0..size]).unwrap();
            let attr_vec: Vec<&str> = attr_msg.split(' ').collect();

            match *attr_vec.get(0).unwrap() {
                "Ok" => {
                    if attr_vec.len() < 21 {

                        reply.error(libc::EINVAL);
                    } else {
                        let ts_sec: i64 = attr_vec.get(1).unwrap().parse().unwrap();
                        let ts_nsec: i32 = attr_vec.get(2).unwrap().parse().unwrap();
                        let attr_valid = Timespec::new(ts_sec, ts_nsec);

                        let ino: u64 = attr_vec.get(3).unwrap().parse().unwrap();
                        let size: u64 = attr_vec.get(4).unwrap().parse().unwrap();
                        let blocks: u64 = attr_vec.get(5).unwrap().parse().unwrap();

                        let atime_sec: i64 = attr_vec.get(6).unwrap().parse().unwrap();
                        let atime_nsec: i32 = attr_vec.get(7).unwrap().parse().unwrap();

                        let mtime_sec: i64 = attr_vec.get(8).unwrap().parse().unwrap();
                        let mtime_nsec: i32 = attr_vec.get(9).unwrap().parse().unwrap();

                        let ctime_sec: i64 = attr_vec.get(10).unwrap().parse().unwrap();
                        let ctime_nsec: i32 = attr_vec.get(11).unwrap().parse().unwrap();

                        let crtime_sec: i64 = attr_vec.get(12).unwrap().parse().unwrap();
                        let crtime_nsec: i32 = attr_vec.get(13).unwrap().parse().unwrap();

                        let kind: FileType = match attr_vec.get(14).unwrap().parse().unwrap() {
                            1 => FileType::Directory,
                            _ => FileType::RegularFile,
                        };

                        let perm: u16 = attr_vec.get(15).unwrap().parse().unwrap();
                        let nlink: u32 = attr_vec.get(16).unwrap().parse().unwrap();
                        let uid: u32 = attr_vec.get(17).unwrap().parse().unwrap();
                        let gid: u32 = attr_vec.get(18).unwrap().parse().unwrap();
                        let rdev: u32 = attr_vec.get(19).unwrap().parse().unwrap();
                        let flags: u32 = attr_vec.get(20).unwrap().parse().unwrap();
                        let attr = FileAttr {
                            ino: ino,
                            size: size,
                            blocks: blocks,
                            atime: Timespec::new(atime_sec, atime_nsec),
                            mtime: Timespec::new(mtime_sec, mtime_nsec),
                            ctime: Timespec::new(ctime_sec, ctime_nsec),
                            crtime: Timespec::new(crtime_sec, crtime_nsec),
                            kind: kind,
                            perm: perm,
                            nlink: nlink,
                            uid: uid,
                            gid: gid,
                            rdev: rdev,
                            flags: flags,
                        };

                        reply.attr(&attr_valid, &attr);
                    }
                }
                "Err" => {

                    let err_val: i32 = attr_vec.get(1).unwrap().parse().unwrap();
                    reply.error(err_val);
                },
                _ => reply.error(libc::EINVAL),
            }
        }
    }

    fn bento_create(
        &self,
        _req: &Request,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        _flags: u32,
        reply: ReplyCreate,
    ) {
        unsafe {
            let name_str = name.to_str().unwrap();
            let msg = format!("create {} {}", parent, name_str);
            let mut socket_guard = SOCKET.as_ref().unwrap().lock().unwrap();
            let _ = socket_guard.write(msg.as_bytes());

            let mut msg_resp = [0 as u8; 4096];
            let size = match socket_guard.read(&mut msg_resp) {
                Ok(x) => x,
                Err(_) => {

                    reply.error(libc::EIO);
                    return;
                }
            };
            let attr_msg = str::from_utf8(&msg_resp[0..size]).unwrap();
            let attr_vec: Vec<&str> = attr_msg.split(' ').collect();
            match *attr_vec.get(0).unwrap() {
                "Ok" => {
                    if attr_vec.len() < 24 {

                        reply.error(libc::EINVAL);
                    } else {
                        let ts_sec: i64 = attr_vec.get(1).unwrap().parse().unwrap();
                        let ts_nsec: i32 = attr_vec.get(2).unwrap().parse().unwrap();
                        let attr_valid = Timespec::new(ts_sec, ts_nsec);

                        let ino: u64 = attr_vec.get(3).unwrap().parse().unwrap();
                        let size: u64 = attr_vec.get(4).unwrap().parse().unwrap();
                        let blocks: u64 = attr_vec.get(5).unwrap().parse().unwrap();

                        let atime_sec: i64 = attr_vec.get(6).unwrap().parse().unwrap();
                        let atime_nsec: i32 = attr_vec.get(7).unwrap().parse().unwrap();

                        let mtime_sec: i64 = attr_vec.get(8).unwrap().parse().unwrap();
                        let mtime_nsec: i32 = attr_vec.get(9).unwrap().parse().unwrap();

                        let ctime_sec: i64 = attr_vec.get(10).unwrap().parse().unwrap();
                        let ctime_nsec: i32 = attr_vec.get(11).unwrap().parse().unwrap();

                        let crtime_sec: i64 = attr_vec.get(12).unwrap().parse().unwrap();
                        let crtime_nsec: i32 = attr_vec.get(13).unwrap().parse().unwrap();

                        let kind: FileType = match attr_vec.get(14).unwrap().parse().unwrap() {
                            1 => FileType::Directory,
                            _ => FileType::RegularFile,
                        };

                        let perm: u16 = attr_vec.get(15).unwrap().parse().unwrap();
                        let nlink: u32 = attr_vec.get(16).unwrap().parse().unwrap();
                        let uid: u32 = attr_vec.get(17).unwrap().parse().unwrap();
                        let gid: u32 = attr_vec.get(18).unwrap().parse().unwrap();
                        let rdev: u32 = attr_vec.get(19).unwrap().parse().unwrap();
                        let flags: u32 = attr_vec.get(20).unwrap().parse().unwrap();
                        let generation: u64 = attr_vec.get(21).unwrap().parse().unwrap();
                        let attr = FileAttr {
                            ino: ino,
                            size: size,
                            blocks: blocks,
                            atime: Timespec::new(atime_sec, atime_nsec),
                            mtime: Timespec::new(mtime_sec, mtime_nsec),
                            ctime: Timespec::new(ctime_sec, ctime_nsec),
                            crtime: Timespec::new(crtime_sec, crtime_nsec),
                            kind: kind,
                            perm: perm,
                            nlink: nlink,
                            uid: uid,
                            gid: gid,
                            rdev: rdev,
                            flags: flags,
                        };
                        let fh = attr_vec.get(22).unwrap().parse().unwrap();
                        let open_flags = attr_vec.get(23).unwrap().parse().unwrap();

                        reply.created(&attr_valid, &attr, generation, fh, open_flags);
                    }
                }
                "Err" => {

                    let err_val: i32 = attr_vec.get(1).unwrap().parse().unwrap();
                    reply.error(err_val);
                },
                _ => {

                    reply.error(libc::EINVAL);
                },
            }
        }
    }

    fn bento_mkdir(
        &self,
        _req: &Request,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        reply: ReplyEntry,
    ) {
        unsafe {
            let name_str = name.to_str().unwrap();
            let msg = format!("mkdir {} {}", parent, name_str);
            let mut socket_guard = SOCKET.as_ref().unwrap().lock().unwrap();
            let _ = socket_guard.write(msg.as_bytes());

            let mut msg_resp = [0 as u8; 4096];
            let size = match socket_guard.read(&mut msg_resp) {
                Ok(x) => x,
                Err(_) => {
                    reply.error(libc::EIO);
                    return;
                }
            };
            let attr_msg = str::from_utf8(&msg_resp[0..size]).unwrap();
            let attr_vec: Vec<&str> = attr_msg.split(' ').collect();
            match *attr_vec.get(0).unwrap() {
                "Ok" => {
                    if attr_vec.len() < 22 {
                        reply.error(libc::EINVAL);
                    } else {
                        let ts_sec: i64 = attr_vec.get(1).unwrap().parse().unwrap();
                        let ts_nsec: i32 = attr_vec.get(2).unwrap().parse().unwrap();
                        let attr_valid = Timespec::new(ts_sec, ts_nsec);

                        let ino: u64 = attr_vec.get(3).unwrap().parse().unwrap();
                        let size: u64 = attr_vec.get(4).unwrap().parse().unwrap();
                        let blocks: u64 = attr_vec.get(5).unwrap().parse().unwrap();

                        let atime_sec: i64 = attr_vec.get(6).unwrap().parse().unwrap();
                        let atime_nsec: i32 = attr_vec.get(7).unwrap().parse().unwrap();

                        let mtime_sec: i64 = attr_vec.get(8).unwrap().parse().unwrap();
                        let mtime_nsec: i32 = attr_vec.get(9).unwrap().parse().unwrap();

                        let ctime_sec: i64 = attr_vec.get(10).unwrap().parse().unwrap();
                        let ctime_nsec: i32 = attr_vec.get(11).unwrap().parse().unwrap();

                        let crtime_sec: i64 = attr_vec.get(12).unwrap().parse().unwrap();
                        let crtime_nsec: i32 = attr_vec.get(13).unwrap().parse().unwrap();

                        let kind: FileType = match attr_vec.get(14).unwrap().parse().unwrap() {
                            1 => FileType::Directory,
                            _ => FileType::RegularFile,
                        };

                        let perm: u16 = attr_vec.get(15).unwrap().parse().unwrap();
                        let nlink: u32 = attr_vec.get(16).unwrap().parse().unwrap();
                        let uid: u32 = attr_vec.get(17).unwrap().parse().unwrap();
                        let gid: u32 = attr_vec.get(18).unwrap().parse().unwrap();
                        let rdev: u32 = attr_vec.get(19).unwrap().parse().unwrap();
                        let flags: u32 = attr_vec.get(20).unwrap().parse().unwrap();
                        let generation: u64 = attr_vec.get(21).unwrap().parse().unwrap();
                        let attr = FileAttr {
                            ino: ino,
                            size: size,
                            blocks: blocks,
                            atime: Timespec::new(atime_sec, atime_nsec),
                            mtime: Timespec::new(mtime_sec, mtime_nsec),
                            ctime: Timespec::new(ctime_sec, ctime_nsec),
                            crtime: Timespec::new(crtime_sec, crtime_nsec),
                            kind: kind,
                            perm: perm,
                            nlink: nlink,
                            uid: uid,
                            gid: gid,
                            rdev: rdev,
                            flags: flags,
                        };
                        reply.entry(&attr_valid, &attr, generation);
                    }
                }
                "Err" => {
                    let err_val: i32 = attr_vec.get(1).unwrap().parse().unwrap();
                    reply.error(err_val);
                },
                _ => reply.error(libc::EINVAL),
            }
        }
    }

    fn bento_lookup(
        &self,
        _req: &Request,
        nodeid: u64,
        name: &OsStr,
        reply: ReplyEntry,
    ) {
        unsafe {
            let name_str = name.to_str().unwrap();
            let msg = format!("lookup {} {}", nodeid, name_str);
            let mut socket_guard = SOCKET.as_ref().unwrap().lock().unwrap();
            let _ = socket_guard.write(msg.as_bytes());

            let mut msg_resp = [0 as u8; 4096];
            let size = match socket_guard.read(&mut msg_resp) {
                Ok(x) => x,
                Err(_) => {
                    reply.error(libc::EIO);
                    return;
                }
            };
            let attr_msg = str::from_utf8(&msg_resp[0..size]).unwrap();
            let attr_vec: Vec<&str> = attr_msg.split(' ').collect();
            match *attr_vec.get(0).unwrap() {
                "Ok" => {
                    if attr_vec.len() < 22 {
                        reply.error(libc::EINVAL);
                    } else {
                        let ts_sec: i64 = attr_vec.get(1).unwrap().parse().unwrap();
                        let ts_nsec: i32 = attr_vec.get(2).unwrap().parse().unwrap();
                        let attr_valid = Timespec::new(ts_sec, ts_nsec);

                        let ino: u64 = attr_vec.get(3).unwrap().parse().unwrap();
                        let size: u64 = attr_vec.get(4).unwrap().parse().unwrap();
                        let blocks: u64 = attr_vec.get(5).unwrap().parse().unwrap();

                        let atime_sec: i64 = attr_vec.get(6).unwrap().parse().unwrap();
                        let atime_nsec: i32 = attr_vec.get(7).unwrap().parse().unwrap();

                        let mtime_sec: i64 = attr_vec.get(8).unwrap().parse().unwrap();
                        let mtime_nsec: i32 = attr_vec.get(9).unwrap().parse().unwrap();

                        let ctime_sec: i64 = attr_vec.get(10).unwrap().parse().unwrap();
                        let ctime_nsec: i32 = attr_vec.get(11).unwrap().parse().unwrap();

                        let crtime_sec: i64 = attr_vec.get(12).unwrap().parse().unwrap();
                        let crtime_nsec: i32 = attr_vec.get(13).unwrap().parse().unwrap();

                        let kind: FileType = match attr_vec.get(14).unwrap().parse().unwrap() {
                            1 => FileType::Directory,
                            _ => FileType::RegularFile,
                        };

                        let perm: u16 = attr_vec.get(15).unwrap().parse().unwrap();
                        let nlink: u32 = attr_vec.get(16).unwrap().parse().unwrap();
                        let uid: u32 = attr_vec.get(17).unwrap().parse().unwrap();
                        let gid: u32 = attr_vec.get(18).unwrap().parse().unwrap();
                        let rdev: u32 = attr_vec.get(19).unwrap().parse().unwrap();
                        let flags: u32 = attr_vec.get(20).unwrap().parse().unwrap();
                        let generation: u64 = attr_vec.get(21).unwrap().parse().unwrap();
                        let attr = FileAttr {
                            ino: ino,
                            size: size,
                            blocks: blocks,
                            atime: Timespec::new(atime_sec, atime_nsec),
                            mtime: Timespec::new(mtime_sec, mtime_nsec),
                            ctime: Timespec::new(ctime_sec, ctime_nsec),
                            crtime: Timespec::new(crtime_sec, crtime_nsec),
                            kind: kind,
                            perm: perm,
                            nlink: nlink,
                            uid: uid,
                            gid: gid,
                            rdev: rdev,
                            flags: flags,
                        };
                        reply.entry(&attr_valid, &attr, generation);
                    }
                }
                "Err" => {
                    let err_val: i32 = attr_vec.get(1).unwrap().parse().unwrap();
                    reply.error(err_val);
                },
                _ => {
                    reply.error(libc::EINVAL);
                },
            }
        }
    }

    fn bento_read(
        &self,
        _req: &Request,
        nodeid: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        reply: ReplyData,
    ) {
        unsafe {
            let mut bento_resp_vec = vec![0 as u8; size as usize + 3];
            let msg = format!("read {} {} {}", nodeid, offset, size);
            let mut socket_guard = SOCKET.as_ref().unwrap().lock().unwrap();
            let _ = socket_guard.write(msg.as_bytes());

            let mut msg_resp  = bento_resp_vec.as_mut_slice();
            let read_size = match socket_guard.read(&mut msg_resp) {
                Ok(x) => x,
                Err(_) => {
                    reply.error(libc::EIO);
                    return;
                }
            };
            let read_msg = str::from_utf8(&msg_resp[0..read_size]).unwrap();
            let read_vec: Vec<&str> = read_msg.split(' ').collect();
            match *read_vec.get(0).unwrap() {
                "Ok" => {

                    reply.data(&bento_resp_vec.as_slice()[3..]);
                }
                "Err" => {
                    let err_val: i32 = read_vec.get(1).unwrap().parse().unwrap();
                    reply.error(err_val);
                },
                _ => reply.error(libc::EINVAL),
            }
        }
        
    }

    fn bento_write(
        &self,
        _req: &Request,
        nodeid: u64,
        _fh: u64,
        offset: i64,
        data: &[u8],
        _flags: u32,
        reply: ReplyWrite,
    ) {
        unsafe {
            let data_size = data.len();
            let mut w_off = 0;
            let mut w_size = 4000;
            let mut socket_guard = SOCKET.as_ref().unwrap().lock().unwrap();
            while w_off < data_size {
                if data_size - w_off < 4000 { // last write
                    w_size = data_size - w_off;
                } else {
                    w_size = 4000;
                }
                let data_slice = &data[w_off..w_off + w_size];
                let msg = format!("write {} {} {}", nodeid, offset, str::from_utf8(&*data_slice).unwrap());
                let _ = socket_guard.write(msg.as_bytes());

                let mut msg_resp = [0 as u8; 4096];
                let size = match socket_guard.read(&mut msg_resp) {
                    Ok(x) => x,
                    Err(_) => {
                        reply.error(libc::EIO);
                        return;
                    }
                };
                let write_msg = str::from_utf8(&msg_resp[0..size]).unwrap();
                let write_vec: Vec<&str> = write_msg.split(' ').collect();
                match *write_vec.get(0).unwrap() {
                    "Ok" => {
                        if write_vec.len() < 2 {
                            reply.error(libc::EINVAL);
                        } else {
                            let size: u32 = write_vec.get(1).unwrap().parse().unwrap();
                            w_off += size as usize;
                        }
                    }
                    "Err" => {
                        let err_val: i32 = write_vec.get(1).unwrap().parse().unwrap();
                        reply.error(err_val);
                        return;
                    },
                    _ => {
                        reply.error(libc::EINVAL);
                        return;
                    },
                }
            }
            reply.written(data_size as u32);
        }
            
    }

    #[allow(unused_mut)]
    fn bento_readdir(
        &self,
        _req: &Request,
        nodeid: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        unsafe {
            let msg = format!("readdir {} {}", nodeid, offset);
            let mut socket_guard = SOCKET.as_ref().unwrap().lock().unwrap();
            let _ = socket_guard.write(msg.as_bytes());

            let mut msg_resp = [0 as u8; 4096];
            let size = match socket_guard.read(&mut msg_resp) {
                Ok(x) => x,
                Err(_) => {
                    reply.error(libc::EIO);
                    return;
                }
            };
            let readdir_msg = str::from_utf8(&msg_resp[0..size]).unwrap();
            let mut readdir_vec: Vec<&str> = readdir_msg.split(' ').collect();
            while readdir_vec.len() > 0 {
                match *readdir_vec.get(0).unwrap() {
                    "Add" => {
                        if readdir_vec.len() < 5 {
                            reply.error(libc::EINVAL);
                            return;
                        } else {
                            let ino: u64 = readdir_vec.get(1).unwrap().parse().unwrap();
                            let offset: i64 = readdir_vec.get(2).unwrap().parse().unwrap();
                            let kind: FileType = match readdir_vec.get(3).unwrap().parse().unwrap() {
                                1 => FileType::Directory,
                                _ => FileType::RegularFile,
                            };
                            let name: &str = readdir_vec.get(4).unwrap();
                            reply.add(ino, offset, kind, name);
                            readdir_vec = readdir_vec.drain(5..).collect();
                        }
                    }
                    "Ok" => {
                        reply.ok();
                        return;
                    },
                    "Err" => {
                        let err_val: i32 = readdir_vec.get(1).unwrap().parse().unwrap();
                        reply.error(err_val);
                        return;
                    },
                    _ => {
                        reply.error(libc::EINVAL);
                        return;
                    },
                }
            }
        }
    }

    fn bento_rmdir(&self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        unsafe {
            let name_str = name.to_str().unwrap();
            let msg = format!("rmdir {} {}", parent, name_str);
            let mut socket_guard = SOCKET.as_ref().unwrap().lock().unwrap();
            let _ = socket_guard.write(msg.as_bytes());

            let mut msg_resp = [0 as u8; 4096];
            let size = match socket_guard.read(&mut msg_resp) {
                Ok(x) => x,
                Err(_) => {
                    reply.error(libc::EIO);
                    return;
                }
            };
            let open_msg = str::from_utf8(&msg_resp[0..size]).unwrap();
            let open_vec: Vec<&str> = open_msg.split(' ').collect();
            match *open_vec.get(0).unwrap() {
                "Ok" => {
                    reply.ok();
                }
                "Err" => {
                    let err_val: i32 = open_vec.get(1).unwrap().parse().unwrap();
                    reply.error(err_val);
                },
                _ => reply.error(libc::EINVAL),
            }
        }
    }

    fn bento_unlink(&self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        unsafe {
            let name_str = name.to_str().unwrap();
            let msg = format!("unlink {} {}", parent, name_str);
            let mut socket_guard = SOCKET.as_ref().unwrap().lock().unwrap();
            let _ = socket_guard.write(msg.as_bytes());

            let mut msg_resp = [0 as u8; 4096];
            let size = match socket_guard.read(&mut msg_resp) {
                Ok(x) => x,
                Err(_) => {
                    reply.error(libc::EIO);
                    return;
                }
            };
            let open_msg = str::from_utf8(&msg_resp[0..size]).unwrap();
            let open_vec: Vec<&str> = open_msg.split(' ').collect();
            match *open_vec.get(0).unwrap() {
                "Ok" => {
                    reply.ok();
                }
                "Err" => {
                    let err_val: i32 = open_vec.get(1).unwrap().parse().unwrap();
                    reply.error(err_val);
                },
                _ => reply.error(libc::EINVAL),
            }
        }
    }

    fn bento_fsync(
        &self,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _datasync: bool,
        reply: ReplyEmpty,
    ) {
        unsafe {
            let msg = format!("fsync");
            let mut socket_guard = SOCKET.as_ref().unwrap().lock().unwrap();
            let _ = socket_guard.write(msg.as_bytes());

            let mut msg_resp = [0 as u8; 4096];
            let size = match socket_guard.read(&mut msg_resp) {
                Ok(x) => x,
                Err(_) => {
                    reply.error(libc::EIO);
                    return;
                }
            };
            let open_msg = str::from_utf8(&msg_resp[0..size]).unwrap();
            let open_vec: Vec<&str> = open_msg.split(' ').collect();
            match *open_vec.get(0).unwrap() {
                "Ok" => {
                    reply.ok();
                }
                "Err" => {
                    let err_val: i32 = open_vec.get(1).unwrap().parse().unwrap();
                    reply.error(err_val);
                },
                _ => reply.error(libc::EINVAL),
            }
        }
    }

    fn bento_fsyncdir(
        &self,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _datasync: bool,
        reply: ReplyEmpty
    ) {
        unsafe {
            let msg = format!("fsyncdir");
            let mut socket_guard = SOCKET.as_ref().unwrap().lock().unwrap();
            let _ = socket_guard.write(msg.as_bytes());

            let mut msg_resp = [0 as u8; 4096];
            let size = match socket_guard.read(&mut msg_resp) {
                Ok(x) => x,
                Err(_) => {
                    reply.error(libc::EIO);
                    return;
                }
            };
            let open_msg = str::from_utf8(&msg_resp[0..size]).unwrap();
            let open_vec: Vec<&str> = open_msg.split(' ').collect();
            match *open_vec.get(0).unwrap() {
                "Ok" => {
                    reply.ok();
                }
                "Err" => {
                    let err_val: i32 = open_vec.get(1).unwrap().parse().unwrap();
                    reply.error(err_val);
                },
                _ => reply.error(libc::EINVAL),
            }
        }
    }

    fn bento_symlink(
        &self,
        _req: &Request,
        nodeid: u64,
        name: &OsStr,
        linkname: &Path,
        reply: ReplyEntry,
    ) {
        unsafe {
            let name_str = name.to_str().unwrap();
            let linkname_str = linkname.to_str().unwrap();
            let msg = format!("symlink {} {} {}", nodeid, name_str, linkname_str);
            let mut socket_guard = SOCKET.as_ref().unwrap().lock().unwrap();
            let _ = socket_guard.write(msg.as_bytes());

            let mut msg_resp = [0 as u8; 4096];
            let size = match socket_guard.read(&mut msg_resp) {
                Ok(x) => x,
                Err(_) => {
                    reply.error(libc::EIO);
                    return;
                }
            };
            let attr_msg = str::from_utf8(&msg_resp[0..size]).unwrap();
            let attr_vec: Vec<&str> = attr_msg.split(' ').collect();
            match *attr_vec.get(0).unwrap() {
                "Ok" => {
                    if attr_vec.len() < 22 {
                        reply.error(libc::EINVAL);
                    } else {
                        let ts_sec: i64 = attr_vec.get(1).unwrap().parse().unwrap();
                        let ts_nsec: i32 = attr_vec.get(2).unwrap().parse().unwrap();
                        let attr_valid = Timespec::new(ts_sec, ts_nsec);

                        let ino: u64 = attr_vec.get(3).unwrap().parse().unwrap();
                        let size: u64 = attr_vec.get(4).unwrap().parse().unwrap();
                        let blocks: u64 = attr_vec.get(5).unwrap().parse().unwrap();

                        let atime_sec: i64 = attr_vec.get(6).unwrap().parse().unwrap();
                        let atime_nsec: i32 = attr_vec.get(7).unwrap().parse().unwrap();

                        let mtime_sec: i64 = attr_vec.get(8).unwrap().parse().unwrap();
                        let mtime_nsec: i32 = attr_vec.get(9).unwrap().parse().unwrap();

                        let ctime_sec: i64 = attr_vec.get(10).unwrap().parse().unwrap();
                        let ctime_nsec: i32 = attr_vec.get(11).unwrap().parse().unwrap();

                        let crtime_sec: i64 = attr_vec.get(12).unwrap().parse().unwrap();
                        let crtime_nsec: i32 = attr_vec.get(13).unwrap().parse().unwrap();

                        let kind: FileType = match attr_vec.get(14).unwrap().parse().unwrap() {
                            1 => FileType::Directory,
                            _ => FileType::RegularFile,
                        };

                        let perm: u16 = attr_vec.get(15).unwrap().parse().unwrap();
                        let nlink: u32 = attr_vec.get(16).unwrap().parse().unwrap();
                        let uid: u32 = attr_vec.get(17).unwrap().parse().unwrap();
                        let gid: u32 = attr_vec.get(18).unwrap().parse().unwrap();
                        let rdev: u32 = attr_vec.get(19).unwrap().parse().unwrap();
                        let flags: u32 = attr_vec.get(20).unwrap().parse().unwrap();
                        let generation: u64 = attr_vec.get(21).unwrap().parse().unwrap();
                        let attr = FileAttr {
                            ino: ino,
                            size: size,
                            blocks: blocks,
                            atime: Timespec::new(atime_sec, atime_nsec),
                            mtime: Timespec::new(mtime_sec, mtime_nsec),
                            ctime: Timespec::new(ctime_sec, ctime_nsec),
                            crtime: Timespec::new(crtime_sec, crtime_nsec),
                            kind: kind,
                            perm: perm,
                            nlink: nlink,
                            uid: uid,
                            gid: gid,
                            rdev: rdev,
                            flags: flags,
                        };
                        reply.entry(&attr_valid, &attr, generation);
                    }
                }
                "Err" => {
                    let err_val: i32 = attr_vec.get(1).unwrap().parse().unwrap();
                    reply.error(err_val);
                },
                _ => reply.error(libc::EINVAL),
            }
        }
    }


    fn bento_readlink(&self, _req: &Request, nodeid: u64, reply: ReplyData) {
        unsafe {
            let msg = format!("readlink {}", nodeid);
            let mut socket_guard = SOCKET.as_ref().unwrap().lock().unwrap();
            let _ = socket_guard.write(msg.as_bytes());

            let mut msg_resp = [0 as u8; 4096];
            let size = match socket_guard.read(&mut msg_resp) {
                Ok(x) => x,
                Err(_) => {
                    reply.error(libc::EIO);
                    return;
                }
            };
            let read_msg = str::from_utf8(&msg_resp[0..size]).unwrap();
            let read_vec: Vec<&str> = read_msg.split(' ').collect();
            match *read_vec.get(0).unwrap() {
                "Ok" => {
                    let data = &msg_resp[3..size];
                    reply.data(data);
                }
                "Err" => {
                    let err_val: i32 = read_vec.get(1).unwrap().parse().unwrap();
                    reply.error(err_val);
                },
                _ => reply.error(libc::EINVAL),
            }
        }
    }

    fn bento_rename(
        &self,
        _req: &Request,
        parent_ino: u64,
        name: &OsStr,
        newparent_ino: u64,
        newname: &OsStr,
        flags: u32,
        reply: ReplyEmpty,
    ) {
        unsafe {
            let name_str = name.to_str().unwrap();
            let newname_str = newname.to_str().unwrap();
            let msg = format!("rename {} {} {} {} {}", parent_ino, name_str, newparent_ino, newname_str, flags);
            let mut socket_guard = SOCKET.as_ref().unwrap().lock().unwrap();
            let _ = socket_guard.write(msg.as_bytes());

            let mut msg_resp = [0 as u8; 4096];
            let size = match socket_guard.read(&mut msg_resp) {
                Ok(x) => x,
                Err(_) => {
                    reply.error(libc::EIO);
                    return;
                }
            };
            let open_msg = str::from_utf8(&msg_resp[0..size]).unwrap();
            let open_vec: Vec<&str> = open_msg.split(' ').collect();
            match *open_vec.get(0).unwrap() {
                "Ok" => {
                    reply.ok();
                }
                "Err" => {
                    let err_val: i32 = open_vec.get(1).unwrap().parse().unwrap();
                    reply.error(err_val);
                },
                _ => reply.error(libc::EINVAL),
            }
        }
    }

}
