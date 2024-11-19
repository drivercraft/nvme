use core::{alloc::Layout, marker::PhantomData};

use crate::{err::*, DMAMem, OS};

pub struct DMAVec<T, O: OS> {
    _marker: PhantomData<(T, O)>,
    len: usize,
    dma: DMAMem,
}

impl<T, O: OS> DMAVec<T, O> {
    pub fn zeros(len: usize) -> Result<Self> {
        let size = len * core::mem::size_of::<T>();
        let layout = Layout::from_size_align(size, size).map_err(|_| Error::Layout)?;
        let dma = O::dma_alloc(layout).ok_or(Error::NoMemory)?;
        Ok(DMAVec {
            _marker: PhantomData,
            dma,
            len,
        })
    }

    pub fn bus_addr(&self) -> u64 {
        self.dma.phys
    }
}

impl<T, O: OS> Drop for DMAVec<T, O> {
    fn drop(&mut self) {
        O::dma_dealloc(self.dma);
    }
}

impl<T, O: OS> core::ops::Deref for DMAVec<T, O> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        unsafe { core::slice::from_raw_parts(self.dma.virt.cast().as_ptr(), self.len) }
    }
}

impl<T, O: OS> core::ops::DerefMut for DMAVec<T, O> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { core::slice::from_raw_parts_mut(self.dma.virt.cast().as_mut(), self.len) }
    }
}
