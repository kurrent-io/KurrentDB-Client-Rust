use crate::{All, EventData, Single, Streaming};

pub trait Sealed {}

impl Sealed for usize {}
impl Sealed for u64 {}
impl Sealed for u32 {}
impl Sealed for u16 {}
impl Sealed for u8 {}
impl Sealed for EventData {}
impl<A: Sealed> Sealed for Vec<A> {}
impl<A> Sealed for Streaming<A> {}
impl Sealed for All {}
impl Sealed for Single {}
