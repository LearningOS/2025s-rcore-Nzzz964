use super::{
    block_cache_sync_all, get_block_cache, BlockDevice, DirEntry, DiskInode, DiskInodeType,
    EasyFileSystem, DIRENT_SZ,
};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::{Mutex, MutexGuard};

pub struct InodeMetadata {
    /// Inode ID
    pub inode_id: u32,
    /// Reference count
    pub ref_cnt: u32,
    /// Whether it's a file
    pub is_file: bool,
    /// Whether it's a directory
    pub is_dir: bool,
}

/// Virtual filesystem layer over easy-fs
pub struct Inode {
    block_id: usize,
    block_offset: usize,
    fs: Arc<Mutex<EasyFileSystem>>,
    block_device: Arc<dyn BlockDevice>,
}

impl Inode {
    /// Create a vfs inode
    pub fn new(
        block_id: u32,
        block_offset: usize,
        fs: Arc<Mutex<EasyFileSystem>>,
        block_device: Arc<dyn BlockDevice>,
    ) -> Self {
        Self {
            block_id: block_id as usize,
            block_offset,
            fs,
            block_device,
        }
    }
    /// Call a function over a disk inode to read it
    fn read_disk_inode<V>(&self, f: impl FnOnce(&DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .read(self.block_offset, f)
    }
    /// Call a function over a disk inode to modify it
    fn modify_disk_inode<V>(&self, f: impl FnOnce(&mut DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .modify(self.block_offset, f)
    }
    /// Find inode under a disk inode by name
    fn find_inode_id(&self, name: &str, disk_inode: &DiskInode) -> Option<u32> {
        // assert it is a directory
        assert!(disk_inode.is_dir());
        let file_count = (disk_inode.size as usize) / DIRENT_SZ;
        let mut dirent = DirEntry::empty();
        for i in 0..file_count {
            assert_eq!(
                disk_inode.read_at(DIRENT_SZ * i, dirent.as_bytes_mut(), &self.block_device,),
                DIRENT_SZ,
            );
            if dirent.name() == name {
                return Some(dirent.inode_id() as u32);
            }
        }
        None
    }
    /// Find inode under current inode by name
    pub fn find(&self, name: &str) -> Option<Arc<Inode>> {
        let fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            self.find_inode_id(name, disk_inode).map(|inode_id| {
                let (block_id, block_offset) = fs.get_disk_inode_pos(inode_id);
                Arc::new(Self::new(
                    block_id,
                    block_offset,
                    self.fs.clone(),
                    self.block_device.clone(),
                ))
            })
        })
    }
    /// Increase the size of a disk inode
    fn increase_size(
        &self,
        new_size: u32,
        disk_inode: &mut DiskInode,
        fs: &mut MutexGuard<EasyFileSystem>,
    ) {
        if new_size < disk_inode.size {
            return;
        }
        let blocks_needed = disk_inode.blocks_num_needed(new_size);
        let mut v: Vec<u32> = Vec::new();
        for _ in 0..blocks_needed {
            v.push(fs.alloc_data());
        }
        disk_inode.increase_size(new_size, v, &self.block_device);
    }
    /// Create inode under current inode by name
    pub fn create(&self, name: &str) -> Option<Arc<Inode>> {
        let mut fs = self.fs.lock();
        let op = |root_inode: &DiskInode| {
            // assert it is a directory
            assert!(root_inode.is_dir());
            // has the file been created?
            self.find_inode_id(name, root_inode)
        };
        if self.read_disk_inode(op).is_some() {
            return None;
        }
        // create a new file
        // alloc a inode with an indirect block
        let new_inode_id = fs.alloc_inode();
        // initialize inode
        let (new_inode_block_id, new_inode_block_offset) = fs.get_disk_inode_pos(new_inode_id);
        get_block_cache(new_inode_block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(new_inode_block_offset, |new_inode: &mut DiskInode| {
                new_inode.initialize(DiskInodeType::File);
                new_inode.ref_cnt += 1;
            });
        self.modify_disk_inode(|root_inode| {
            // append file in the dirent
            let file_count = (root_inode.size as usize) / DIRENT_SZ;
            let new_size = (file_count + 1) * DIRENT_SZ;
            // increase size
            self.increase_size(new_size as u32, root_inode, &mut fs);
            // write dirent
            let dirent = DirEntry::new(name, new_inode_id);
            root_inode.write_at(
                file_count * DIRENT_SZ,
                dirent.as_bytes(),
                &self.block_device,
            );
        });

        let (block_id, block_offset) = fs.get_disk_inode_pos(new_inode_id);
        block_cache_sync_all();
        // return inode
        Some(Arc::new(Self::new(
            block_id,
            block_offset,
            self.fs.clone(),
            self.block_device.clone(),
        )))
        // release efs lock automatically by compiler
    }
    /// List inodes under current inode
    pub fn ls(&self) -> Vec<String> {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            let file_count = (disk_inode.size as usize) / DIRENT_SZ;
            let mut v: Vec<String> = Vec::new();
            for i in 0..file_count {
                let mut dirent = DirEntry::empty();
                assert_eq!(
                    disk_inode.read_at(i * DIRENT_SZ, dirent.as_bytes_mut(), &self.block_device,),
                    DIRENT_SZ,
                );
                v.push(String::from(dirent.name()));
            }
            v
        })
    }
    /// Read data from current inode
    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| disk_inode.read_at(offset, buf, &self.block_device))
    }
    /// Write data to current inode
    pub fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let mut fs = self.fs.lock();
        let size = self.modify_disk_inode(|disk_inode| {
            self.increase_size((offset + buf.len()) as u32, disk_inode, &mut fs);
            disk_inode.write_at(offset, buf, &self.block_device)
        });
        block_cache_sync_all();
        size
    }
    /// Clear the data in current inode
    pub fn clear(&self) {
        let mut fs = self.fs.lock();
        self.modify_disk_inode(|disk_inode| {
            let size = disk_inode.size;
            // 将 DiskInode 的元数据设置为 0，包括
            // size=0（文件大小），direct=0,indirect1=0,indirect2=0
            let data_blocks_dealloc = disk_inode.clear_size(&self.block_device);
            assert!(data_blocks_dealloc.len() == DiskInode::total_blocks(size) as usize);
            for data_block in data_blocks_dealloc.into_iter() {
                // 根据 block_id 清空实际的数据
                fs.dealloc_data(data_block);
            }
        });
        block_cache_sync_all();
    }

    /// Create a hard link at newpath pointing to the same inode as oldpath
    /// Returns the Inode of the new link if successful
    /// Only call if Inde type is dir
    pub fn linkat(&self, oldpath: &str, newpath: &str) -> Option<Arc<Inode>> {
        // PS: 为了方便，不考虑新文件路径已经存在的情况（属于未定义行为），除非链接同名文件。
        if oldpath == newpath {
            return None;
        }

        let mut fs = self.fs.lock();

        // get oldpath inode_id
        let inode_id =
            self.read_disk_inode(|root_inode: &DiskInode| self.find_inode_id(oldpath, root_inode))?;

        // increase ref_cnt
        let (inode_block_id, inode_block_offset) = fs.get_disk_inode_pos(inode_id);
        get_block_cache(inode_block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(inode_block_offset, |inode: &mut DiskInode| {
                inode.ref_cnt += 1;
            });

        // link oldpath at newpath, create DirEntry with same inode_id
        self.modify_disk_inode(|root_inode| {
            // append file in the dirent
            let file_count = (root_inode.size as usize) / DIRENT_SZ;
            let new_size = (file_count + 1) * DIRENT_SZ;
            // increase size
            self.increase_size(new_size as u32, root_inode, &mut fs);
            // write dirent
            let dirent = DirEntry::new(newpath, inode_id);

            root_inode.write_at(
                file_count * DIRENT_SZ,
                dirent.as_bytes(),
                &self.block_device,
            );
        });

        let (inode_block_id, inode_block_offset) = fs.get_disk_inode_pos(inode_id);
        block_cache_sync_all();

        // return inode
        Some(Arc::new(Self::new(
            inode_block_id,
            inode_block_offset,
            self.fs.clone(),
            self.block_device.clone(),
        )))
    }

    /// unlinkat
    pub fn unlinkat(&self, path: &str) -> bool {
        // fs.lock here
        let mut fs = self.fs.lock();

        let op = |root_inode: &DiskInode| self.find_inode_id(path, root_inode);
        let inode_id = self.read_disk_inode(op);

        // the file is not exists
        if inode_id.is_none() {
            return false;
        }

        let inode_id = inode_id.unwrap();

        // remove dirent
        self.modify_disk_inode(|root_inode| {
            let file_count = (root_inode.size as usize) / DIRENT_SZ;
            let mut dirent_idx = 0;

            for i in 0..file_count {
                let mut dirent = DirEntry::empty();
                root_inode.read_at(i * DIRENT_SZ, dirent.as_bytes_mut(), &self.block_device);
                if dirent.name() == path {
                    dirent_idx = i;
                    break;
                }
            }

            // if dirent_idx is not the last index
            if dirent_idx < file_count - 1 {
                let mut last_dirent = DirEntry::empty();
                root_inode.read_at(
                    (file_count - 1) * DIRENT_SZ,
                    last_dirent.as_bytes_mut(),
                    &self.block_device,
                );
                root_inode.write_at(
                    dirent_idx * DIRENT_SZ,
                    last_dirent.as_bytes(),
                    &self.block_device,
                );
            }

            // 按道理来说，这里应该删除 direntry 所占用的 data_blocks
            let new_size = (file_count - 1) * DIRENT_SZ;
            root_inode.size = new_size as u32;
        });

        // decrease ref_cnt
        let (inode_block_id, inode_block_offset) = fs.get_disk_inode_pos(inode_id);
        get_block_cache(inode_block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(inode_block_offset, |inode: &mut DiskInode| {
                inode.ref_cnt -= 1;

                if inode.ref_cnt == 0 {
                    // clear data block and diskinode
                    let size = inode.size;
                    let data_blocks_dealloc = inode.clear_size(&self.block_device);
                    assert!(data_blocks_dealloc.len() == DiskInode::total_blocks(size) as usize);
                    for data_block in data_blocks_dealloc.into_iter() {
                        fs.dealloc_data(data_block);
                    }
                    // dealloc inode_id in inode_bitmap
                    fs.dealloc_diskinode(inode_id);
                }
            });

        block_cache_sync_all();
        true
    }

    /// Get metadata of the Inode
    pub fn metadata(&self) -> InodeMetadata {
        let inode_id = self
            .fs
            .lock()
            .get_inode_id(self.block_id, self.block_offset);

        self.read_disk_inode(|disk_inode| InodeMetadata {
            inode_id,
            ref_cnt: disk_inode.ref_cnt,
            is_file: disk_inode.is_file(),
            is_dir: disk_inode.is_dir(),
        })
    }
}
