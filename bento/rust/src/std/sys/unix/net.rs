use alloc::boxed::Box;
use core::ops::{Deref, DerefMut};

use bindings as c;
use kernel::ffi;
use libc;

pub type Socket = c::sock;
pub type Net = c::net;
pub type SkBuff = c::sk_buff;

//pub struct SocketRef {
//    pub inner: *mut c::sock,
//}

//pub struct Net {
//    pub inner: *mut c::net,
//}

pub struct SocketLockGuard<'a> {
    sock: &'a mut Socket
}

impl Drop for SocketLockGuard<'_> {
    fn drop(&mut self) {
        unsafe {
            c::release_sock(self.sock)
        }
    }
}

impl Deref for SocketLockGuard<'_> {
    type Target = Socket;

    fn deref(&self) -> &Socket {
        unsafe { &*self.sock }
    }
}

impl DerefMut for SocketLockGuard<'_> {
    fn deref_mut(&mut self) -> &mut Socket {
        unsafe { &mut *self.sock }
    }
}

impl Socket {
    pub unsafe fn alloc(net: &c::net, family: i32, priority: c::gfp_t,
                        prot: *mut c::proto, kern: i32) -> Option<Box<Self>> {
        // Doesn't actually mutate net
        let sk = c::sk_alloc(net as *const c::net as *mut c::net, family, priority, prot, kern);
        if sk.is_null() {
            None
        } else {
            unsafe {
                //let owned_sock = Box::from_raw(sk);
                Some(Box::from_raw(sk))
            }
        }
    }

    pub fn init_data(&mut self, sock: &c::socket) {
        // Doesn't actually mutate sock
        unsafe {
            c::sock_init_data(sock as *const c::socket as *mut c::socket, self);
        }
    }

    pub fn refcnt_debug_inc(&mut self) {
        unsafe {
            ffi::rs_sk_refcnt_debug_inc(self as *mut c::sock);
        }
    }

    pub fn get_prot(&self) -> Option<&c::proto> {
        unsafe {
            let maybe_prot = (*self).__sk_common.skc_prot;
            if maybe_prot.is_null() {
                None
            } else {
                Some(&*maybe_prot)
            }
        }
    }

    pub unsafe fn from_raw_sock<'a>(sock: &'a mut c::sock) -> &'a mut Self {
        sock
        //if sock.is_null() {
        //    None
        //} else {
        //    Some( sock )
        //}
    }

    pub fn lock(&mut self) -> SocketLockGuard {
        unsafe {
            ffi::rs_lock_sock(self);
        }
        SocketLockGuard { sock: self }
    }

    pub fn state(&self) -> u8 {
        unsafe {
            self.__sk_common.skc_state
        }
    }

    pub fn host_port(&self) -> u16 {
        unsafe {
            self.__sk_common.__bindgen_anon_3.__bindgen_anon_1.skc_num
        }
    }

    pub fn source_addr(&self) -> u32 {
        unsafe {
            self.__sk_common.__bindgen_anon_1.__bindgen_anon_1.skc_rcv_saddr
        }
    }

    pub fn set_source_addr(&mut self, saddr: u32) {
        unsafe {
            self.__sk_common.__bindgen_anon_1.__bindgen_anon_1.skc_rcv_saddr = saddr;
        }
    }

    pub fn dest_addr(&self) -> u32 {
        unsafe {
            self.__sk_common.__bindgen_anon_1.__bindgen_anon_1.skc_daddr
        }
    }

    pub fn set_dest_addr(&mut self, daddr: u32) {
        unsafe {
            self.__sk_common.__bindgen_anon_1.__bindgen_anon_1.skc_daddr = daddr;
        }
    }

    pub fn dest_port(&self) -> u16 {
        unsafe {
            self.__sk_common.__bindgen_anon_3.__bindgen_anon_1.skc_dport
        }
    }

    pub fn set_dest_port(&mut self, dport: u16) {
        unsafe {
            self.__sk_common.__bindgen_anon_3.__bindgen_anon_1.skc_dport = dport;
        }
    }

    pub fn dst_reset(&mut self) {
        unsafe {
            ffi::rs_sk_dst_reset(self);
        }
    }

    pub fn set_max_ack_backlog(&mut self, backlog: u32) {
        unsafe {
            core::ptr::write_volatile(&mut self.sk_max_ack_backlog as *mut u32, backlog);
        }
    }

    pub fn set_ack_backlog(&mut self, backlog: u32) {
        unsafe {
            core::ptr::write_volatile(&mut self.sk_ack_backlog as *mut u32, backlog);
        }
    }

    pub fn store_state(&mut self, state: u8) {
        unsafe {
            ffi::rs_smp_store_release(&mut self.__sk_common.skc_state as *mut u8, state);
        }
    }

    pub fn flag(&self, flag: c::sock_flags) -> bool {
        unsafe {
            ffi::rs_sock_flag(self, flag)
        }
    }

    pub fn set_flag(&mut self, flag: c::sock_flags) {
        unsafe {
            ffi::rs_sock_set_flag(self, flag);
        }
    }

    pub fn prot_inuse_add(&mut self, val: i32) {
        unsafe {
            ffi::rs_sock_prot_inuse_add(ffi::rs_sock_net(self), self.__sk_common.skc_prot, val);
        }
    }

    pub fn set_state(&mut self, state: u8) {
        unsafe {
            self.__sk_common.skc_state = state;
        }
    }

    pub fn net(&self) -> *mut c::net {
        unsafe {
            ffi::rs_sock_net(self)
        }
    }

    // TODO: Get this so it doesn't take a raw pointer
    pub unsafe fn graft(&mut self, parent: *mut c::socket) {
        ffi::rs_sock_graft(self, parent);
    }
}

//impl Deref for Socket {
//    type Target = c::sock;
//
//    fn deref(&self) -> &c::sock {
//        unsafe { &*self }
//    }
//}
//
//impl DerefMut for Socket {
//    fn deref_mut(&mut self) -> &mut c::sock {
//        unsafe { self }
//    }
//}

impl Net {
    pub unsafe fn from_raw<'a>(raw: &'a mut c::net) -> &'a mut Self {
        raw
    }

    pub fn ns_capable(&self, cap: i32) -> bool {
        unsafe {
            c::ns_capable(self.user_ns, cap)
        }
    }
}

impl SkBuff {
    pub fn skb_share_check(mut self: Box<Self>) -> Option<Box<Self>> {
        unsafe {
            // TODO: Get the C funtion to actally accept ownership somehow
            let self_ref = Box::leak(self);
            let skb = ffi::rs_skb_share_check(self_ref);
            if skb.is_null() {
                None
            } else {
                Some(Box::from_raw(skb))
                //Some(owned_skb)
            }
        }
    }

    pub fn pskb_may_pull(&mut self, len: u32) -> bool {
        unsafe {
            ffi::rs_pskb_may_pull(self as *mut c::sk_buff, len)
        }
    }

    pub fn ip_hdr(&self) -> &c::iphdr {
        unsafe {
            &*(self.head.offset(self.network_header as isize) as *const c::iphdr)
        }
    }

    pub fn ip_hdr_mut(&mut self) -> &mut c::iphdr {
        unsafe {
            &mut *(self.head.offset(self.network_header as isize) as *mut c::iphdr)
        }
    }
}
