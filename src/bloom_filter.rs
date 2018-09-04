#![allow(dead_code)]

use murmur;

const ARRAY_SIZE: usize = 8192;

/// Data structure that elements can be added to, such that when shown a new element,
/// it can decide if it has been added. Here, added means its hash stays saved.
#[derive(Clone)]
pub struct BloomFilter{
    bitarray: Vec<u8>,
    seeds: Vec<u32>,
}

impl BloomFilter {
    /// Creates and returns a new BloomFilter structure.
    ///
    /// # Arguments
    ///
    /// * `seeds` - seeds to be used for hashing. More seeds yield less collissions.
    pub fn new(seeds: Vec<u32>) -> BloomFilter {
        BloomFilter{
            bitarray: vec![0u8;ARRAY_SIZE], // use box syntax as soon as there is an updated compiler (lazy admin is lazy)
            seeds: seeds,
        }
    }

    /// Adds an element to the BloomFilter structure.
    ///
    /// # Arguments
    ///
    /// * `item` - byte slice representation of element to be hashed.
    pub fn add(&mut self, item: &[u8]){
        let seed_pairs=self.seeds.iter().map(|&x| murmur::murmur_hash3_32(item, x)).map(|x| (x>>16,x&0xffff));

        for (s1,s2) in seed_pairs{
            self.bitarray[(s1>>3) as usize]|=1<<((s1&7) as u8);
            self.bitarray[(s2>>3) as usize]|=1<<((s2&7) as u8);
        }
    }

    /// Checks whether an element was added to the BloomFilter structure.
    ///
    /// # Arguments
    ///
    /// * `item` - byte slice representation of element to be hashed.
    pub fn contains(&self, item: &[u8]) -> bool{
        let mut seed_pairs=self.seeds.iter().map(|&x| murmur::murmur_hash3_32(item, x)).map(|x| (x>>16,x&0xffff));

        seed_pairs.all(|(s1,s2)| self.bitarray[(s1>>3) as usize]&((1<<(s1&7)) as u8)!=0 && self.bitarray[(s2>>3) as usize]&((1<<(s2&7)) as u8)!=0)
    }

    /// Checks whether an element was added to the BloomFilter structure. If it
    /// was not, it gets added. The result of the check gets returned.
    ///
    /// # Arguments
    ///
    /// * `item` - byte slice representation of element to be hashed.
    pub fn contains_add(&mut self, item: &[u8]) -> bool{
        let seed_pairs=self.seeds.iter().map(|&x| murmur::murmur_hash3_32(item, x)).map(|x| (x>>16,x&0xffff));
        let mut contains=true;

        for (s1,s2) in seed_pairs{
            if (self.bitarray[(s1>>3) as usize]&((1<<(s1&7)) as u8))|(self.bitarray[(s2>>3) as usize]&((1<<(s2&7)) as u8))==0{
                contains=false;
                self.bitarray[(s1>>3) as usize]|=1<<((s1&7) as u8);
                self.bitarray[(s2>>3) as usize]|=1<<((s2&7) as u8);
            }
        }

        contains
    }
}

const LARGE_ARRAY_SIZE: usize = 536870912;

/// Data structure that elements can be added to, such that when shown a new element,
/// it can decide if it has been added. Here, added means its hash stays saved.
/// This is a larger version of BloomFilter, which occupies 512 MB of memory.
#[derive(Clone)]
pub struct LargeBloomFilter{
    bitarray: Vec<u8>, // use box of array as soon as box syntax is here (lazy adming is lazy, again)
    seeds: Vec<u32>,
}

impl LargeBloomFilter {
    /// Creates and returns a new LargeBloomFilter structure.
    ///
    /// # Arguments
    ///
    /// * `seeds` - seeds to be used for hashing. More seeds yield less collissions.
    pub fn new(seeds: Vec<u32>) -> LargeBloomFilter {
        LargeBloomFilter{
            bitarray: vec![0u8;LARGE_ARRAY_SIZE],
            seeds: seeds,
        }
    }

    /// Adds an element to the LargeBloomFilter structure.
    ///
    /// # Arguments
    ///
    /// * `item` - byte slice representation of element to be hashed.
    pub fn add(&mut self, item: &[u8]){
        let fourwise_hashes=self.seeds.iter().map(|&x| murmur::murmur_hash3_x64_128(item, x)).map(|(x1,x2)| (x1>>32, x1&0xffffffff, x2>>32, x2&0xffffffff));

        for (s1,s2,s3,s4) in fourwise_hashes{
            self.bitarray[(s1>>3) as usize]|=1<<((s1&7) as u8);
            self.bitarray[(s2>>3) as usize]|=1<<((s2&7) as u8);
            self.bitarray[(s3>>3) as usize]|=1<<((s3&7) as u8);
            self.bitarray[(s4>>3) as usize]|=1<<((s4&7) as u8);
        }
    }

    /// Checks whether an element was added to the LargeBloomFilter structure.
    ///
    /// # Arguments
    ///
    /// * `item` - byte slice representation of element to be hashed.
    pub fn contains(&self, item: &[u8]) -> bool{
        let mut fourwise_hashes=self.seeds.iter().map(|&x| murmur::murmur_hash3_x64_128(item, x)).map(|(x1,x2)| (x1>>32, x1&0xffffffff, x2>>32, x2&0xffffffff));

        fourwise_hashes.all(|(s1,s2,s3,s4)| 
            self.bitarray[(s1>>3) as usize]&((1<<(s1&7)) as u8)!=0 &&
            self.bitarray[(s2>>3) as usize]&((1<<(s2&7)) as u8)!=0 &&
            self.bitarray[(s3>>3) as usize]&((1<<(s3&7)) as u8)!=0 &&
            self.bitarray[(s4>>3) as usize]&((1<<(s4&7)) as u8)!=0
            )
    }

    /// Checks whether an element was added to the LargeBloomFilter structure. If it
    /// was not, it gets added. The result of the check gets returned.
    ///
    /// # Arguments
    ///
    /// * `item` - byte slice representation of element to be hashed.
    pub fn contains_add(&mut self, item: &[u8]) -> bool{
        let fourwise_hashes=self.seeds.iter().map(|&x| murmur::murmur_hash3_x64_128(item, x)).map(|(x1,x2)| (x1>>32, x1&0xffffffff, x2>>32, x2&0xffffffff));
        let mut contains=true;

        for (s1,s2,s3,s4) in fourwise_hashes{
            if  (self.bitarray[(s1>>3) as usize]&((1<<(s1&7)) as u8)) |
                (self.bitarray[(s2>>3) as usize]&((1<<(s2&7)) as u8)) |
                (self.bitarray[(s3>>3) as usize]&((1<<(s3&7)) as u8)) |
                (self.bitarray[(s4>>3) as usize]&((1<<(s4&7)) as u8))==0{

                contains=false;
                self.bitarray[(s1>>3) as usize]|=1<<((s1&7) as u8);
                self.bitarray[(s2>>3) as usize]|=1<<((s2&7) as u8);
                self.bitarray[(s3>>3) as usize]|=1<<((s3&7) as u8);
                self.bitarray[(s4>>3) as usize]|=1<<((s4&7) as u8);
            }
        }

        contains
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::str;

    #[test]
    fn test_bloom_filter() {
        let mut bloom_filter=BloomFilter::new(vec![0xa4a759a4, 0xe5f20661, 0x85684b56, 0xba444a10]);

        let to_add=[b"1" as &[u8], b"hello" as &[u8], b"NaN" as &[u8], b"" as &[u8], b"ohle" as &[u8], b"some rather long string just because. And just in case, lets make it even longer :D" as &[u8]];
        let not_to_add=[b"0" as &[u8], b"bye" as &[u8], b"ehlo" as &[u8], b"lello" as &[u8], b"_" as &[u8], b"another rather long string just because. And just in case, lets make it even longer :D" as &[u8]];
        let to_add_later=[b"2" as &[u8], b"12" as &[u8], b"123" as &[u8], b"1234" as &[u8], b"12345" as &[u8], b"some longer string I guess" as &[u8]];


        for s in to_add.iter(){
            bloom_filter.add(s);
        }

        for s in to_add.iter(){
            assert!(bloom_filter.contains(s), format!("BloomFilter says a word that was added is not contained. ({:?})", str::from_utf8(s)));
        }

        for s in not_to_add.iter(){
            assert!(!bloom_filter.contains(s), format!("BloomFilter says a word that was not added is contained. ({:?})", str::from_utf8(s)));
        }

        for s in to_add_later.iter(){
            assert!(bloom_filter.contains_add(s)==false, format!("BloomFilter says a word that was not added is contained (contains_add). ({:?})", str::from_utf8(s)));
        }

        for s in to_add_later.iter(){
            assert!(bloom_filter.contains_add(s)==true, format!("BloomFilter says a word that was added is not contained (contains_add). ({:?})", str::from_utf8(s)));
        }
    }

    #[test]
    fn test_large_bloom_filter() {
        let mut bloom_filter=LargeBloomFilter::new(vec![0xa4a759a4, 0xe5f20661]);

        let to_add=[b"1" as &[u8], b"hello" as &[u8], b"NaN" as &[u8], b"" as &[u8], b"ohle" as &[u8], b"some rather long string just because. And just in case, lets make it even longer :D" as &[u8]];
        let not_to_add=[b"0" as &[u8], b"bye" as &[u8], b"ehlo" as &[u8], b"lello" as &[u8], b"_" as &[u8], b"another rather long string just because. And just in case, lets make it even longer :D" as &[u8]];
        let to_add_later=[b"2" as &[u8], b"12" as &[u8], b"123" as &[u8], b"1234" as &[u8], b"12345" as &[u8], b"some longer string I guess" as &[u8]];


        for s in to_add.iter(){
            bloom_filter.add(s);
        }

        for s in to_add.iter(){
            assert!(bloom_filter.contains(s), format!("LargeBloomFilter says a word that was added is not contained. ({:?})", str::from_utf8(s)));
        }

        for s in not_to_add.iter(){
            assert!(!bloom_filter.contains(s), format!("LargeBloomFilter says a word that was not added is contained. ({:?})", str::from_utf8(s)));
        }

        for s in to_add_later.iter(){
            assert!(bloom_filter.contains_add(s)==false, format!("LargeBloomFilter says a word that was not added is contained (contains_add). ({:?})", str::from_utf8(s)));
        }

        for s in to_add_later.iter(){
            assert!(bloom_filter.contains_add(s)==true, format!("LargeBloomFilter says a word that was added is not contained (contains_add). ({:?})", str::from_utf8(s)));
        }
    }
}