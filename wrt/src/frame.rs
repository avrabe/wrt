    fn store_u16(&self, addr: usize, align: u32, value: u16) -> Result<()> {
        self.get_memory(0)?.store_u16(addr, align, value)
    }

    fn store_v128(&self, addr: usize, align: u32, value: [u8; 16]) -> Result<()> {
        self.get_memory(0)?.store_v128(addr, align, value)
    }

    fn memory_size(&self) -> Result<u32> {
        self.get_memory(0)?.size()
    }

    fn get_memory(&self, index: usize) -> Result<&Memory> {
        // Implementation of get_memory method
        unimplemented!()
    }
} 